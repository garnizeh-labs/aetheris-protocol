use crate::types::{ClientId, ComponentKind, NetworkId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A reliable discrete game event (Phase 1 / VS-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameEvent {
    /// An asteroid was completely depleted of its ore.
    AsteroidDepleted {
        /// The network ID of the asteroid that was depleted.
        network_id: NetworkId,
    },
    /// Explicitly informs a client that they now own/control a specific entity.
    Possession {
        /// The network ID of the entity now owned by the client.
        network_id: NetworkId,
    },
    /// Sends extensible server-side metadata (versions, counters, debug data).
    SystemManifest {
        /// The collection of metadata key-value pairs.
        manifest: BTreeMap<String, String>,
    },
}

impl GameEvent {
    /// Converts a `GameEvent` into a `WireEvent`.
    #[must_use]
    pub fn into_wire_event(self) -> WireEvent {
        WireEvent::GameEvent(self)
    }
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
    /// Client requests to start a gameplay session: spawns the session ship and grants Possession.
    StartSession {
        /// The client starting the session.
        client_id: ClientId,
    },
    /// A request from a client to receive the current system manifest.
    RequestSystemManifest {
        /// The client that requested the manifest.
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
    /// A batch of replication updates sent together to save bandwidth/packets.
    ReplicationBatch {
        /// The client that should receive the batch.
        client_id: ClientId,
        /// The collection of updates.
        events: Vec<ReplicationEvent>,
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
            | Self::StartSession { .. }
            | Self::RequestSystemManifest { .. }
            | Self::GameEvent { .. }
            | Self::ReplicationBatch { .. } => true,
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
    /// Client requests to start a gameplay session: spawns the session ship and grants Possession.
    StartSession,
    /// A request to receive the current system manifest.
    RequestSystemManifest,
    /// A discrete game event.
    GameEvent(GameEvent),
    /// A batch of replication updates.
    ReplicationBatch(Vec<ReplicationEvent>),
}

impl WireEvent {
    /// Converts a `WireEvent` into a `NetworkEvent` for a specific client context.
    #[must_use]
    pub fn into_network_event(self, client_id: crate::types::ClientId) -> NetworkEvent {
        match self {
            Self::Ping { tick } => NetworkEvent::Ping { client_id, tick },
            Self::Pong { tick } => NetworkEvent::Pong { tick },
            Self::Auth { session_token } => NetworkEvent::Auth { session_token },
            Self::Fragment(fragment) => NetworkEvent::Fragment {
                client_id,
                fragment,
            },
            Self::StressTest { count, rotate } => NetworkEvent::StressTest {
                client_id,
                count,
                rotate,
            },
            Self::Spawn {
                entity_type,
                x,
                y,
                rot,
            } => NetworkEvent::Spawn {
                client_id,
                entity_type,
                x,
                y,
                rot,
            },
            Self::ClearWorld => NetworkEvent::ClearWorld { client_id },
            Self::StartSession => NetworkEvent::StartSession { client_id },
            Self::RequestSystemManifest => NetworkEvent::RequestSystemManifest { client_id },
            Self::GameEvent(event) => NetworkEvent::GameEvent { client_id, event },
            Self::ReplicationBatch(events) => NetworkEvent::ReplicationBatch { client_id, events },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_event_is_wire() {
        assert!(
            NetworkEvent::Ping {
                client_id: ClientId(1),
                tick: 100
            }
            .is_wire()
        );
        assert!(
            NetworkEvent::GameEvent {
                client_id: ClientId(1),
                event: GameEvent::AsteroidDepleted {
                    network_id: NetworkId(1)
                }
            }
            .is_wire()
        );
        assert!(
            NetworkEvent::ReplicationBatch {
                client_id: ClientId(1),
                events: vec![]
            }
            .is_wire()
        );
        assert!(!NetworkEvent::ClientConnected(ClientId(1)).is_wire());
        assert!(!NetworkEvent::ClientDisconnected(ClientId(1)).is_wire());
    }

    #[test]
    fn test_wire_event_conversion_roundtrip() {
        let wire = WireEvent::GameEvent(GameEvent::AsteroidDepleted {
            network_id: NetworkId(42),
        });
        let client_id = ClientId(7);
        let network = wire.clone().into_network_event(client_id);

        if let NetworkEvent::GameEvent {
            client_id: cid,
            event,
        } = network
        {
            assert_eq!(cid, client_id);
            assert_eq!(
                event,
                GameEvent::AsteroidDepleted {
                    network_id: NetworkId(42)
                }
            );
        } else {
            panic!("Conversion failed to preserve GameEvent variant");
        }

        // Test ReplicationBatch conversion
        let event = ReplicationEvent {
            network_id: NetworkId(1),
            component_kind: ComponentKind(1),
            payload: vec![1, 2, 3],
            tick: 100,
        };
        let batch_wire = WireEvent::ReplicationBatch(vec![event.clone()]);
        let batch_network = batch_wire.into_network_event(client_id);
        if let NetworkEvent::ReplicationBatch {
            client_id: cid,
            events,
        } = batch_network
        {
            assert_eq!(cid, client_id);
            assert!(!events.is_empty());
            assert_eq!(events[0].tick, 100);
            assert_eq!(events[0].payload, vec![1, 2, 3]);
            assert_eq!(events[0].network_id, NetworkId(1));
            assert_eq!(events[0].component_kind, ComponentKind(1));
        } else {
            panic!("Conversion failed to preserve ReplicationBatch variant");
        }
    }
}
