//! Core trait contracts for the Aetheris Engine.
//!
//! These traits form the boundary between the engine's protocol logic and
//! external dependencies (ECS, Transport, Serialization).

use async_trait::async_trait;

pub use crate::error::{EncodeError, TransportError, WorldError};
use crate::events::{ComponentUpdate, NetworkEvent, ReplicationEvent};
pub use crate::types::{ClientId, LocalId, NetworkId, NetworkIdAllocator};

/// Abstracts the underlying network transport.
///
/// # Why this exists
/// In Phase 1, this wraps `renet`. In Phase 3, this wraps `quinn` directly.
/// The game loop never knows which library is underneath.
///
/// # Reliability semantics
/// - `send_unreliable`: Fire-and-forget. Used for position updates that are
///   invalidated by the next tick. If the packet is lost, the client simply
///   interpolates from the last known position.
/// - `send_reliable`: Ordered and guaranteed delivery. Used for discrete game
///   events (damage, death, loot) where loss would desync the client.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GameTransport: Sync + GameTransportBounds {
    /// Sends an unreliable datagram to a specific client.
    ///
    /// Returns immediately. The transport layer may silently drop this packet
    /// under congestion — this is by design for volatile data.
    ///
    /// # Errors
    /// Returns `TransportError::ClientNotConnected` if the `client_id` is unknown,
    /// or `TransportError::PayloadTooLarge` if the packet exceeds MTU.
    async fn send_unreliable(&self, client_id: ClientId, data: &[u8])
    -> Result<(), TransportError>;

    /// Sends a reliable, ordered message to a specific client.
    ///
    /// The transport guarantees delivery and ordering within a single stream.
    /// Callers must not assume delivery timing — only eventual delivery.
    ///
    /// # Errors
    /// Returns `TransportError::ClientNotConnected` if the `client_id` is unknown,
    /// or `TransportError::Io` on underlying transport failure.
    async fn send_reliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError>;

    /// Broadcasts an unreliable datagram to all connected clients.
    ///
    /// Useful for world-wide events (weather changes, global announcements)
    /// where individual targeting is unnecessary.
    ///
    /// # Errors
    /// Returns `TransportError::PayloadTooLarge` if the packet exceeds MTU.
    async fn broadcast_unreliable(&self, data: &[u8]) -> Result<(), TransportError>;

    /// Drains all pending inbound network events since the last call.
    ///
    /// This is called exactly once per tick at the top of the game loop.
    /// Events include: client connections, disconnections, and inbound data.
    async fn poll_events(&mut self) -> Vec<NetworkEvent>;

    /// Returns the number of currently connected clients.
    async fn connected_client_count(&self) -> usize;
}

/// Helper trait to provide conditional `Send` bounds for [`GameTransport`].
#[cfg(target_arch = "wasm32")]
pub trait GameTransportBounds {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> GameTransportBounds for T {}

#[cfg(not(target_arch = "wasm32"))]
pub trait GameTransportBounds: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized + Send> GameTransportBounds for T {}

/// The ECS Facade. Translates between the engine's protocol-level types
/// and the concrete ECS's internal representation.
///
/// # Why this exists
/// Bevy uses `Entity`, an opaque 64-bit handle with generation bits.
/// Our network protocol uses `NetworkId`, a globally unique `u64`.
/// This trait is the translation layer. The game loop never touches
/// a Bevy `Entity` directly — it only speaks `NetworkId`.
///
/// # Delta extraction
/// On every tick, modified components are detected and emitted as
/// `ReplicationEvent` items. Only changed fields are sent — never the full
/// component. This is the foundation of delta compression.
pub trait WorldState: Send {
    /// Maps a protocol-level `NetworkId` to the ECS's local entity handle.
    ///
    /// Returns `None` if the entity has been despawned or never existed.
    fn get_local_id(&self, network_id: NetworkId) -> Option<LocalId>;

    /// Maps a local ECS entity handle back to its protocol-level `NetworkId`.
    ///
    /// Returns `None` if the entity is not network-replicated.
    fn get_network_id(&self, local_id: LocalId) -> Option<NetworkId>;

    /// Extracts replication deltas for all components modified since the last tick.
    ///
    /// The returned events contain only the *changed* fields, not full snapshots.
    /// The caller (the game loop) never interprets these events — it passes them
    /// directly to the `Encoder` for serialization.
    fn extract_deltas(&mut self) -> Vec<ReplicationEvent>;

    /// Injects parsed state updates from the network into the ECS.
    ///
    /// On the server, these are client inputs (movement commands, actions).
    /// On the client, these are authoritative state corrections from the server.
    ///
    /// The `ClientId` in the update pair provides context for ownership verification
    /// to prevent unauthorized updates from malicious clients.
    fn apply_updates(&mut self, updates: &[(ClientId, ComponentUpdate)]);

    /// Advances the world change tick at the start of each server tick, before inputs are applied.
    fn advance_tick(&mut self) {}

    /// Runs a single simulation frame for the ECS.
    fn simulate(&mut self) {}

    /// Spawns a new network-replicated entity and returns its `NetworkId`.
    fn spawn_networked(&mut self) -> NetworkId;

    /// Spawns a new network-replicated entity owned by a specific client.
    fn spawn_networked_for(&mut self, _client_id: ClientId) -> NetworkId {
        self.spawn_networked()
    }

    /// Despawn a network-replicated entity by its `NetworkId`.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError`] if the entity with the given `network_id` does not exist.
    fn despawn_networked(&mut self, network_id: NetworkId) -> Result<(), WorldError>;

    /// Triggers a bulk spawn of entities for stress testing.
    fn stress_test(&mut self, _count: u16, _rotate: bool) {}

    /// Spawns a new network-replicated entity of a specific kind.
    fn spawn_kind(&mut self, _kind: u16, _x: f32, _y: f32, _rot: f32) -> NetworkId {
        self.spawn_networked() // Fallback to basic networked spawn
    }

    /// Despawns all entities from the world.
    fn clear_world(&mut self) {}
}

/// Defines the serialization strategy for network payloads.
///
/// # Why this exists
/// In Phase 1, this wraps `serde` + `rmp-serde` for rapid iteration.
/// In Phase 3, this becomes a custom bit-packer that writes individual
/// bits across 32-bit word boundaries for maximum compression.
///
/// # Performance contract
/// Phase 1 (current) implementations may allocate during serialization
/// to simplify development. However, avoiding allocations is a primary
/// Phase 3 goal for the custom bit-packer.
///
/// In Phase 3, implementations MUST be allocation-free on the hot path.
/// The `encode` method writes into a caller-provided buffer.
/// The `decode` method reads from a borrowed slice.
/// No `Vec`, no `String`, no heap allocation during steady-state operation.
pub trait Encoder: Send + Sync {
    /// Serializes a replication event into the provided buffer.
    ///
    /// Returns the number of bytes written. If the buffer is too small,
    /// returns `EncodeError::BufferOverflow` — the caller must retry
    /// with a larger buffer or fragment the event.
    ///
    /// # Errors
    /// Returns `EncodeError::BufferOverflow` if the buffer is too small.
    fn encode(&self, event: &ReplicationEvent, buffer: &mut [u8]) -> Result<usize, EncodeError>;

    /// Deserializes raw bytes into a component update.
    ///
    /// Returns `EncodeError::MalformedPayload` if the bytes do not
    /// constitute a valid event. The caller must handle this gracefully
    /// (log + discard) — malformed packets are expected from lossy networks.
    ///
    /// # Errors
    /// Returns `EncodeError::MalformedPayload` on invalid payload bytes, or
    /// `EncodeError::UnknownComponent` for unregistered component types.
    fn decode(&self, buffer: &[u8]) -> Result<ComponentUpdate, EncodeError>;

    /// Encodes a high-level `NetworkEvent` into a byte vector.
    ///
    /// # Errors
    /// Returns `EncodeError::Io` if serialization fails.
    fn encode_event(&self, event: &NetworkEvent) -> Result<Vec<u8>, EncodeError>;

    /// Decodes a high-level `NetworkEvent` from a byte slice.
    ///
    /// # Errors
    /// Returns `EncodeError::MalformedPayload` if the bytes are not a valid event.
    fn decode_event(&self, data: &[u8]) -> Result<NetworkEvent, EncodeError>;

    /// Returns the maximum possible encoded size for a single event.
    ///
    /// Used by the transport layer to pre-allocate datagram buffers.
    /// Implementations should return a tight upper bound, not a wild guess.
    fn max_encoded_size(&self) -> usize;
}
