<!--
  Live document — update this file whenever:
    • a new trait method is added or removed
    • a new crate is added to the workspace
    • a type is renamed or its semantics change
    • the encoder phase advances
    • the wire format changes in a breaking way
  Version history is tracked via git. Bump the Version field in the frontmatter on every edit.
-->

---
Version: 1.0.0
Last Updated: 2026-04-19
Rust Edition: 2024
MSRV: 1.95.0
Workspace Version: 0.2.4
Phase: 1 (MVP complete)
---

# Copilot Instructions — aetheris-protocol

This is the **single source of truth for wire formats and engine contracts** in the
Aetheris ecosystem. It defines the stable trait boundaries that decouple the game
loop from any specific ECS, transport, or serialization library.

---

## Repository Layout

```
crates/
  aetheris-protocol/          # Core contracts (traits, types, events, errors)
  aetheris-encoder-serde/     # Phase 1: MessagePack encoder (rmp-serde)
  aetheris-encoder-bitpack/   # Phase 3: custom bit-packing encoder (stub)
docs/
  PROTOCOL_DESIGN.md          # Trait Triad, Merkle Chain, SuspicionScore
  ENCODER_DESIGN.md           # MessagePack vs bit-packing, quantization
  TRANSPORT_DESIGN.md         # GameTransport design rationale
  NETWORKING_DESIGN.md        # TCP vs QUIC, HOL blocking, multi-stream
  CONTROL_PLANE_DESIGN.md     # gRPC service definitions
  API_DESIGN.md               # API surface, stability promises
```

---

## The Trait Triad — Core Contracts

Three traits define every boundary in the engine. Never import concrete types in
code that works with these traits; use `Box<dyn Trait>` or generics.

### `GameTransport` — Network I/O

```rust
// crates/aetheris-protocol/src/traits.rs
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GameTransport: Sync + GameTransportBounds {
    /// Fire-and-forget. Used for position updates that are stale next tick.
    async fn send_unreliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError>;

    /// Guaranteed ordered delivery. Used for discrete events (damage, death).
    async fn send_reliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError>;

    /// Unreliable broadcast to all connected clients.
    async fn broadcast_unreliable(&self, data: &[u8]) -> Result<(), TransportError>;

    /// Called exactly once per tick. Drains all inbound events.
    async fn poll_events(&mut self) -> Result<Vec<NetworkEvent>, TransportError>;

    async fn connected_client_count(&self) -> usize;
}
```

**WASM note**: On `wasm32` the trait uses `async_trait(?Send)` because browser tasks
are single-threaded. Do not add `Send` bounds in WASM-targeted code paths.

### `WorldState` — ECS Adapter

```rust
pub trait WorldState: Send {
    fn get_local_id(&self, network_id: NetworkId) -> Option<LocalId>;
    fn get_network_id(&self, local_id: LocalId) -> Option<NetworkId>;

    /// Returns only changed components since last call. Never full snapshots.
    fn extract_deltas(&mut self) -> Vec<ReplicationEvent>;

    /// Injects authoritative updates. `ClientId` is used for ownership checks.
    fn apply_updates(&mut self, updates: &[(ClientId, ComponentUpdate)]);

    fn advance_tick(&mut self) {}
    fn simulate(&mut self) {}
    fn spawn_networked(&mut self) -> NetworkId;
    fn spawn_networked_for(&mut self, client_id: ClientId) -> NetworkId;
    fn despawn_networked(&mut self, network_id: NetworkId) -> Result<(), WorldError>;

    // Stress-test helpers — not called in production game loops
    fn stress_test(&mut self, count: u16, rotate: bool) {}
    fn spawn_kind(&mut self, kind: u16, x: f32, y: f32, rot: f32) -> NetworkId;
    fn clear_world(&mut self) {}
}
```

### `Encoder` — Serialization

```rust
pub trait Encoder: Send + Sync {
    /// Encodes a replication delta into `buffer`. Returns bytes written.
    /// Buffer is caller-owned; no heap allocation on the hot path.
    fn encode(&self, event: &ReplicationEvent, buffer: &mut [u8]) -> Result<usize, EncodeError>;

    fn decode(&self, buffer: &[u8]) -> Result<ComponentUpdate, EncodeError>;

    fn encode_event(&self, event: &NetworkEvent) -> Result<Vec<u8>, EncodeError>;
    fn decode_event(&self, data: &[u8]) -> Result<NetworkEvent, EncodeError>;

    /// Safe upper bound for the encode buffer size. Pre-allocate once per session.
    fn max_encoded_size(&self) -> usize;
}
```

---

## Core Types

```rust
// crates/aetheris-protocol/src/types.rs

/// Global entity ID. Server-assigned. Immutable for the entity lifetime.
/// Never the ECS's internal handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkId(pub u64);

/// ECS-internal handle. Opaque to the network layer. Never sent over the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u64);

/// Client connection session ID. Assigned by transport on connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);

/// Component type discriminant. Used by the Encoder to pick the right codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentKind(pub u16);

/// Standard transform component — ComponentKind(1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: f32,  // radians
    pub entity_type: u16,
}
```

### `NetworkIdAllocator` — Thread-Safe ID Generation

```rust
// Server-side only. One allocator per world, shared across async tasks.
let allocator = NetworkIdAllocator::new(1); // 0 is reserved
let id: NetworkId = allocator.allocate()?;  // Returns AllocatorError::Overflow at u64::MAX
```

---

## Events

### `ReplicationEvent` — Delta from Server ECS

```rust
// Output of WorldState::extract_deltas()
pub struct ReplicationEvent {
    pub network_id: NetworkId,
    pub component_kind: ComponentKind,
    pub payload: Vec<u8>,  // serialized delta — Phase 1: full snapshot per component
    pub tick: u64,
}
```

### `ComponentUpdate` — Input to WorldState::apply_updates()

```rust
// Produced by Encoder::decode(). Same shape as ReplicationEvent.
pub struct ComponentUpdate {
    pub network_id: NetworkId,
    pub component_kind: ComponentKind,
    pub payload: Vec<u8>,
    pub tick: u64,
}
```

### `NetworkEvent` — Transport-Layer Events

```rust
pub enum NetworkEvent {
    ClientConnected(ClientId),
    ClientDisconnected(ClientId),
    UnreliableMessage { client_id: ClientId, data: Vec<u8> },
    ReliableMessage  { client_id: ClientId, data: Vec<u8> },
    Ping { client_id: ClientId, tick: u64 },
    Pong { tick: u64 },
    Auth { session_token: String },
    SessionClosed(ClientId),
    StreamReset(ClientId),
    Fragment { client_id: ClientId, fragment: FragmentedEvent },
    StressTest { client_id: ClientId, count: u16, rotate: bool },
    Spawn { client_id: ClientId, entity_type: u16, x: f32, y: f32, rot: f32 },
    ClearWorld { client_id: ClientId },
}
```

`WireEvent` is the over-the-wire subset — local-only variants
(`ClientConnected`, `UnreliableMessage`, etc.) are excluded and cannot be encoded.

---

## Error Types

```rust
// crates/aetheris-protocol/src/error.rs

#[derive(thiserror::Error, Debug)]
pub enum TransportError {
    #[error("client {0:?} not connected")]
    ClientNotConnected(ClientId),
    #[error("payload too large: {size} > {max}")]
    PayloadTooLarge { size: usize, max: usize },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lock poisoned")]
    LockPoisoned,
}

#[derive(thiserror::Error, Debug)]
pub enum EncodeError {
    #[error("buffer overflow: needed {needed}, available {available}")]
    BufferOverflow { needed: usize, available: usize },
    #[error("malformed payload at offset {offset}: {message}")]
    MalformedPayload { offset: usize, message: String },
    #[error("unknown component kind")]
    UnknownComponent,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum WorldError {
    #[error("entity not found: {0:?}")]
    EntityNotFound(NetworkId),
    #[error("entity already exists: {0:?}")]
    EntityAlreadyExists(NetworkId),
}
```

---

## MTU & Fragmentation Constants

```rust
// crates/aetheris-protocol/src/lib.rs

/// Safe UDP datagram limit (QUIC overhead subtracted from 1280-byte IPv6 MTU).
pub const MAX_SAFE_PAYLOAD_SIZE: usize = 1200;

/// Fragment envelope overhead (enum tag + FragmentedEvent fields).
pub const FRAGMENT_OVERHEAD: usize = 64;

/// Usable payload per fragment.
pub const MAX_FRAGMENT_PAYLOAD_SIZE: usize = MAX_SAFE_PAYLOAD_SIZE - FRAGMENT_OVERHEAD; // 1136

/// Maximum fragments per logical message → max 1.1 MiB reassembled.
pub const MAX_TOTAL_FRAGMENTS: u16 = 1024;
```

Never send a payload larger than `MAX_SAFE_PAYLOAD_SIZE` without fragmenting.
Use `Reassembler` to buffer inbound fragments per client:

```rust
let mut reassembler = Reassembler::new();
// Inside poll_events handler:
if let Some(full_payload) = reassembler.add_fragment(client_id, fragment) {
    // full_payload is the reassembled message
}
// Cleanup stale buffers (call once per tick or per second)
reassembler.prune_stale();
```

---

## Phase 1 Encoder — `SerdeEncoder`

```rust
// crates/aetheris-encoder-serde/src/serde_encoder.rs
use aetheris_encoder_serde::SerdeEncoder;
use aetheris_protocol::traits::Encoder;

let encoder = SerdeEncoder::new();

// Encoding a replication delta — caller pre-allocates buffer
let mut buf = vec![0u8; encoder.max_encoded_size()];
let bytes_written = encoder.encode(&replication_event, &mut buf)?;
transport.send_unreliable(client_id, &buf[..bytes_written]).await?;

// Decoding an inbound message
let update: ComponentUpdate = encoder.decode(&raw_bytes)?;

// Encoding a control event (Ping, Auth, Spawn, etc.)
let encoded = encoder.encode_event(&NetworkEvent::Ping { client_id, tick: 42 })?;

// Decoding a control event
let event: NetworkEvent = encoder.decode_event(&encoded)?;
```

**Wire format (Phase 1)**:
- Header: `PacketHeader { network_id, component_kind, tick }` → ~18 bytes (MessagePack)
- Body: raw component bytes → ~12 bytes for `Transform` (3× f32 + extras)
- **Total: ~30–33 bytes/entity**

**Phase 3 target**: `BitpackEncoder` — ~2–8 bytes/entity (72% compression via field quantization).

---

## Implementing a Custom `WorldState`

```rust
use aetheris_protocol::traits::WorldState;
use aetheris_protocol::types::{ClientId, LocalId, NetworkId, NetworkIdAllocator};
use aetheris_protocol::events::{ComponentUpdate, ReplicationEvent};
use aetheris_protocol::error::WorldError;

pub struct MyWorld {
    allocator: NetworkIdAllocator,
    // ... your ECS state
}

impl WorldState for MyWorld {
    fn get_local_id(&self, network_id: NetworkId) -> Option<LocalId> {
        // translate NetworkId → ECS handle
        todo!()
    }

    fn get_network_id(&self, local_id: LocalId) -> Option<NetworkId> {
        todo!()
    }

    fn extract_deltas(&mut self) -> Vec<ReplicationEvent> {
        // Return only components that changed since last call.
        // For Phase 1 it is acceptable to return full snapshots.
        todo!()
    }

    fn apply_updates(&mut self, updates: &[(ClientId, ComponentUpdate)]) {
        for (client_id, update) in updates {
            // Validate ownership before applying: reject updates where
            // the entity's owner != client_id to prevent spoofing.
            todo!()
        }
    }

    fn spawn_networked(&mut self) -> NetworkId {
        self.allocator.allocate().expect("NetworkId overflow")
    }

    fn spawn_networked_for(&mut self, _client_id: ClientId) -> NetworkId {
        self.allocator.allocate().expect("NetworkId overflow")
    }

    fn despawn_networked(&mut self, network_id: NetworkId) -> Result<(), WorldError> {
        todo!()
    }

    fn spawn_kind(&mut self, _kind: u16, _x: f32, _y: f32, _rot: f32) -> NetworkId {
        self.allocator.allocate().expect("NetworkId overflow")
    }
}
```

---

## Implementing a Custom `GameTransport`

```rust
use async_trait::async_trait;
use aetheris_protocol::traits::GameTransport;
use aetheris_protocol::types::ClientId;
use aetheris_protocol::events::NetworkEvent;
use aetheris_protocol::error::TransportError;

pub struct MyTransport { /* ... */ }

#[async_trait]
impl GameTransport for MyTransport {
    async fn send_unreliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError> {
        if data.len() > aetheris_protocol::MAX_SAFE_PAYLOAD_SIZE {
            return Err(TransportError::PayloadTooLarge {
                size: data.len(),
                max: aetheris_protocol::MAX_SAFE_PAYLOAD_SIZE,
            });
        }
        // send via underlying socket
        todo!()
    }

    async fn send_reliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError> {
        todo!()
    }

    async fn broadcast_unreliable(&self, data: &[u8]) -> Result<(), TransportError> {
        todo!()
    }

    async fn poll_events(&mut self) -> Result<Vec<NetworkEvent>, TransportError> {
        // Drain all pending events. Must be non-blocking.
        todo!()
    }

    async fn connected_client_count(&self) -> usize {
        todo!()
    }
}
```

---

## gRPC Feature Flag

Control-plane proto definitions (Auth, Matchmaking, Telemetry) are gated behind the
`grpc` feature to keep the core crate lean for WASM builds:

```toml
# Cargo.toml of a server crate
aetheris-protocol = { version = "0.2.4", features = ["grpc"] }

# Client / WASM crate — omit the feature
aetheris-protocol = { version = "0.2.4" }
```

---

## Phase Evolution Map

| Subsystem | Phase 1 (now) | Phase 3 (target) |
|---|---|---|
| Transport impl | `RenetTransport` | `QuinnTransport` |
| ECS adapter | `BevyWorldAdapter` | Custom SoA |
| Encoder | `SerdeEncoder` (MessagePack) | `BitpackEncoder` |

Swapping any implementation requires only a new `impl Trait` block.
The traits and types in this crate never change between phases.

---

## Key Conventions

- `NetworkId` is **never** the ECS entity handle. Always go through `WorldState::get_local_id`.
- `WireEvent` is the over-the-wire enum. Never serialize a full `NetworkEvent`; convert first.
- Pre-allocate encode buffers with `encoder.max_encoded_size()` once; reuse across ticks.
- All errors use `thiserror`. Do not use `anyhow` in library code — reserve it for binaries.
- WASM targets: use `#[cfg(target_arch = "wasm32")]` guards; never assume `Send`.
