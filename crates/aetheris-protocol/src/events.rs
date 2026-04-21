use crate::types::{ClientId, ComponentKind, NetworkId};
use serde::{Deserialize, Serialize};

/// A reliable discrete game event (Phase 1 / VS-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameEvent {
    /// An asteroid was completely depleted of its ore.
    AsteroidDepleted {
        /// The network ID of the asteroid that was depleted.
        network_id: NetworkId,
    },
}

/// An event representing a fragment of a larger message.
/// Used for MTU stability to prevent packet drops and enable reassembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentedEvent {
    /// Unique identifier for the fragmented message.
    pub message_id: u32,
    /// The index of this fragment (0-based).
    pub fragment_index: u16,
    /// Total number of fragments for this message.
    pub total_fragments: u16,
    /// The raw payload of this fragment.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

/// An event representing a change to a single component on a single entity.
/// Produced by `WorldState::extract_deltas()` on the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationEvent {
    /// Which entity changed.
    pub network_id: NetworkId,
    /// Which component type changed.
    pub component_kind: ComponentKind,
    /// The serialized delta payload (only the changed fields).
    /// In Phase 1, this is a full snapshot per component for simplicity.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    /// The server tick at which this change was recorded.
    pub tick: u64,
}

/// An inbound update to be applied to the ECS.
/// Produced by `Encoder::decode()` on the receiving end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentUpdate {
    /// The entity to update.
    pub network_id: NetworkId,
    /// Which component type to update.
    pub component_kind: ComponentKind,
    /// The deserialized field values.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    /// The tick this update originated from.
    pub tick: u64,
}

/// Events produced by `GameTransport::poll_events()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkEvent {
    /// A new client has connected and been assigned a `ClientId`.
    ClientConnected(ClientId),
    /// A client has disconnected (graceful or timeout).
    ClientDisconnected(ClientId),
    /// Raw unreliable data received from a client.
    UnreliableMessage {
        /// The client that sent the message.
        client_id: ClientId,
        /// The raw message bytes.
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// Raw reliable data received from a client.
    ReliableMessage {
        /// The client that sent the message.
        client_id: ClientId,
        /// The raw message bytes.
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// A heartbeat ping from a client.
    Ping {
        /// The client that sent the ping.
        client_id: ClientId,
        /// The client's tick/timestamp when the ping was sent.
        tick: u64,
    },
    /// A heartbeat pong from the server.
    Pong {
        /// The original tick/timestamp from the ping.
        tick: u64,
    },
    /// A session authentication request from the client.
    Auth {
        /// The session token obtained from the Control Plane.
        session_token: String,
    },
    /// A WebTransport session was closed by the remote or due to error.
    SessionClosed(ClientId),
    /// A WebTransport stream was reset.
    StreamReset(ClientId),
    /// A fragment of a larger message.
    Fragment {
        /// The client that sent the fragment.
        client_id: ClientId,
        /// The fragment data.
        fragment: FragmentedEvent,
    },
    /// A testing command to trigger a stress test (Phase 1/Playground only).
    StressTest {
        /// The client that requested the stress test.
        client_id: ClientId,
        /// Number of entities to spawn.
        count: u16,
        /// Whether spawned entities should rotate.
        rotate: bool,
    },
    /// A testing command to spawn a specific entity (Phase 1/Playground only).
    Spawn {
        /// The client that requested the spawn.
        client_id: ClientId,
        /// Which entity type to spawn.
        entity_type: u16,
        /// Position X
        x: f32,
        /// Position Y
        y: f32,
        /// Initial rotation
        rot: f32,
    },
    /// A command to clear all entities from the world (Phase 1/Playground only).
    ClearWorld {
        /// The client that requested the clear.
        client_id: ClientId,
    },
    /// A local event indicating the client transport has been disconnected.
    Disconnected(ClientId),
    /// A discrete game event (e.g. depletion, destruction).
    GameEvent {
        /// The client involved (or targeted).
        client_id: ClientId,
        /// The event data.
        event: GameEvent,
    },
}

impl NetworkEvent {
    /// Returns true if this event is capable of being sent over the wire.
    #[must_use]
    pub const fn is_wire(&self) -> bool {
        match self {
            Self::Ping { .. }
            | Self::Pong { .. }
            | Self::Auth { .. }
            | Self::Fragment { .. }
            | Self::StressTest { .. }
            | Self::Spawn { .. }
            | Self::ClearWorld { .. }
            | Self::GameEvent { .. } => true,
            Self::ClientConnected(_)
            | Self::ClientDisconnected(_)
            | Self::UnreliableMessage { .. }
            | Self::ReliableMessage { .. }
            | Self::SessionClosed(_)
            | Self::StreamReset(_)
            | Self::Disconnected(_) => false,
        }
    }
}

/// A restricted view of `NetworkEvent` for over-the-wire transport.
/// Prevents local-only variants (like `ClientConnected`) from being sent/received.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WireEvent {
    /// A heartbeat ping.
    Ping {
        /// The client's tick/timestamp when the ping was sent.
        tick: u64,
    },
    /// A heartbeat pong.
    Pong {
        /// The original tick/timestamp from the ping.
        tick: u64,
    },
    /// A session authentication request.
    Auth {
        /// The session token.
        session_token: String,
    },
    /// A fragment of a larger message.
    Fragment(FragmentedEvent),
    /// A testing command to trigger a stress test.
    StressTest {
        /// Number of entities to spawn.
        count: u16,
        /// Whether spawned entities should rotate.
        rotate: bool,
    },
    /// A testing command to spawn a specific entity.
    Spawn {
        /// Which entity type to spawn.
        entity_type: u16,
        /// Position X
        x: f32,
        /// Position Y
        y: f32,
        /// Initial rotation
        rot: f32,
    },
    /// A command to clear all entities from the world.
    ClearWorld,
    /// A discrete game event.
    GameEvent(GameEvent),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_sizes_and_derives() {
        let ev = NetworkEvent::ClientConnected(ClientId(1));
        assert_eq!(ev, NetworkEvent::ClientConnected(ClientId(1)));

        let re = ReplicationEvent {
            network_id: NetworkId(1),
            component_kind: ComponentKind(0),
            payload: vec![1, 2, 3],
            tick: 0,
        };
        assert_eq!(re.payload.len(), 3);
        assert_eq!(
            re,
            ReplicationEvent {
                network_id: NetworkId(1),
                component_kind: ComponentKind(0),
                payload: vec![1, 2, 3],
                tick: 0,
            }
        );
    }
}
