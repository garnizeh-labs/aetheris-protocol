//! Test doubles (Mocks) for the Aetheris Engine pipeline.
//!
//! Provides isolated, deterministic implementations of phase 1 traits for testing.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use async_trait::async_trait;

use crate::error::{EncodeError, TransportError, WorldError};
use crate::events::{ComponentUpdate, NetworkEvent, ReplicationEvent};
use crate::traits::{Encoder, GameTransport, WorldState};
use crate::types::{ClientId, ComponentKind, LocalId, NetworkId};

/// Mock network transport layer that records outbound sent data
/// and drains injected inbound events.
#[derive(Debug, Default)]
pub struct MockTransport {
    /// Registry of connected client IDs.
    pub connected_clients: Mutex<HashSet<ClientId>>,
    /// Outbound unreliable packets accumulated per client.
    pub per_client_unreliable: Mutex<HashMap<ClientId, Vec<Vec<u8>>>>,
    /// Outbound reliable packets accumulated per client.
    pub per_client_reliable: Mutex<HashMap<ClientId, Vec<Vec<u8>>>>,
    /// Inbound events to emit on next `poll_events`.
    pub inbound_queue: Mutex<VecDeque<NetworkEvent>>,
}

impl MockTransport {
    /// Creates a new, empty transport mock.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects an event into the inbound queue to be read by the next `poll_events`.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn inject_event(&self, event: NetworkEvent) {
        self.inbound_queue.lock().unwrap().push_back(event);
    }

    /// Takes all unreliable packets meant for a client.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn take_unreliable(&self, cid: ClientId) -> Vec<Vec<u8>> {
        self.per_client_unreliable
            .lock()
            .unwrap()
            .remove(&cid)
            .unwrap_or_default()
    }

    /// Takes all reliable packets meant for a client.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn take_reliable(&self, cid: ClientId) -> Vec<Vec<u8>> {
        self.per_client_reliable
            .lock()
            .unwrap()
            .remove(&cid)
            .unwrap_or_default()
    }

    /// Explicitly connects a client to the mock transport.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn connect(&self, client_id: ClientId) {
        self.connected_clients.lock().unwrap().insert(client_id);
    }

    /// Explicitly disconnects a client from the mock transport.
    ///
    /// # Panics
    /// Panics if any of the internal mutexes are poisoned.
    pub fn disconnect(&self, client_id: ClientId) {
        self.connected_clients.lock().unwrap().remove(&client_id);
        self.per_client_unreliable
            .lock()
            .unwrap()
            .remove(&client_id);
        self.per_client_reliable.lock().unwrap().remove(&client_id);
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl GameTransport for MockTransport {
    async fn send_unreliable(
        &self,
        client_id: ClientId,
        data: &[u8],
    ) -> Result<(), TransportError> {
        if !self.connected_clients.lock().unwrap().contains(&client_id) {
            return Err(TransportError::ClientNotConnected(client_id));
        }
        if data.len() > crate::MAX_SAFE_PAYLOAD_SIZE {
            return Err(TransportError::PayloadTooLarge {
                size: data.len(),
                max: crate::MAX_SAFE_PAYLOAD_SIZE,
            });
        }
        self.per_client_unreliable
            .lock()
            .unwrap()
            .entry(client_id)
            .or_default()
            .push(data.to_vec());
        Ok(())
    }

    async fn send_reliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError> {
        if !self.connected_clients.lock().unwrap().contains(&client_id) {
            return Err(TransportError::ClientNotConnected(client_id));
        }
        if data.len() > 65535 {
            return Err(TransportError::PayloadTooLarge {
                size: data.len(),
                max: 65535,
            });
        }
        self.per_client_reliable
            .lock()
            .unwrap()
            .entry(client_id)
            .or_default()
            .push(data.to_vec());
        Ok(())
    }

    async fn broadcast_unreliable(&self, data: &[u8]) -> Result<(), TransportError> {
        if data.len() > crate::MAX_SAFE_PAYLOAD_SIZE {
            return Err(TransportError::PayloadTooLarge {
                size: data.len(),
                max: crate::MAX_SAFE_PAYLOAD_SIZE,
            });
        }
        let clients = self.connected_clients.lock().unwrap();
        let mut map = self.per_client_unreliable.lock().unwrap();
        // Broadcast to all currently connected clients.
        for &client_id in clients.iter() {
            map.entry(client_id).or_default().push(data.to_vec());
        }
        Ok(())
    }

    async fn poll_events(&mut self) -> Result<Vec<NetworkEvent>, TransportError> {
        let mut queue = self.inbound_queue.lock().unwrap();
        Ok(queue.drain(..).collect())
    }

    async fn connected_client_count(&self) -> usize {
        self.connected_clients.lock().unwrap().len()
    }
}

/// Mock ECS adapter representing a simplified world state without an actual ECS behind it.
#[derive(Debug, Default)]
pub struct MockWorldState {
    next_id: u64,
    /// Forward bidirectional map storing the resolution from `NetworkId` to `LocalId`.
    pub forward_bimap: HashMap<NetworkId, LocalId>,
    /// Reverse bidirectional map storing the resolution from `LocalId` to `NetworkId`.
    pub reverse_bimap: HashMap<LocalId, NetworkId>,
    /// Thread-safe vector of manually queued deltas to be returned next extraction.
    pub pending_deltas: Mutex<Vec<ReplicationEvent>>,
    /// Thread-safe vector of all updates received via `apply_updates`.
    pub applied_updates: Mutex<Vec<(ClientId, ComponentUpdate)>>,
}

impl MockWorldState {
    /// Creates a new `MockWorldState`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: 1, // 0 is reserved
            forward_bimap: HashMap::new(),
            reverse_bimap: HashMap::new(),
            pending_deltas: Mutex::new(Vec::new()),
            applied_updates: Mutex::new(Vec::new()),
        }
    }

    /// Helper to artificially queue a delta event for extraction.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn queue_delta(&self, event: ReplicationEvent) {
        self.pending_deltas.lock().unwrap().push(event);
    }
}

impl WorldState for MockWorldState {
    fn spawn_networked(&mut self) -> NetworkId {
        let n_id = NetworkId(self.next_id);
        let l_id = LocalId(self.next_id);
        self.next_id += 1;

        self.forward_bimap.insert(n_id, l_id);
        self.reverse_bimap.insert(l_id, n_id);
        n_id
    }

    fn spawn_networked_for(&mut self, _client_id: ClientId) -> NetworkId {
        self.spawn_networked()
    }

    fn despawn_networked(&mut self, network_id: NetworkId) -> Result<(), WorldError> {
        if let Some(l_id) = self.forward_bimap.remove(&network_id) {
            self.reverse_bimap.remove(&l_id);
            Ok(())
        } else {
            Err(WorldError::EntityNotFound(network_id))
        }
    }

    fn get_local_id(&self, network_id: NetworkId) -> Option<LocalId> {
        self.forward_bimap.get(&network_id).copied()
    }

    fn get_network_id(&self, local_id: LocalId) -> Option<NetworkId> {
        self.reverse_bimap.get(&local_id).copied()
    }

    /// Extracts all pending deltas from the world.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    fn extract_deltas(&mut self) -> Vec<ReplicationEvent> {
        let mut queued = self.pending_deltas.lock().unwrap();
        std::mem::take(&mut *queued)
    }

    /// Applies updates to the mock world.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    fn apply_updates(&mut self, updates: &[(ClientId, ComponentUpdate)]) {
        self.applied_updates
            .lock()
            .unwrap()
            .extend(updates.iter().cloned());
    }

    fn simulate(&mut self) {
        // No-op in P1 mock, signifies a simulation step.
    }

    fn stress_test(&mut self, _count: u16, _rotate: bool) {}

    fn spawn_kind(&mut self, _kind: u16, _x: f32, _y: f32, _rot: f32) -> NetworkId {
        self.spawn_networked()
    }

    fn clear_world(&mut self) {}
}

/// Mock encoder that writes a dummy header byte (`0xAE`) in front of the payload.
#[derive(Debug, Default)]
pub struct MockEncoder;

impl MockEncoder {
    /// Sentinel byte indicating a Mock Encoder packet.
    pub const MOCK_SENTINEL: u8 = 0xAE;
    /// Artificial error byte triggering a `MalformedPayload`.
    pub const MOCK_ERROR_BYTE: u8 = 0xFF;

    /// Creates a new `MockEncoder`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for MockEncoder {
    fn encode(&self, event: &ReplicationEvent, buffer: &mut [u8]) -> Result<usize, EncodeError> {
        // Simple encoding for mock round-trip: Sentinel(1) + NetworkId(8) + ComponentKind(2) + Tick(8) + Payload(n)
        let required = 1 + 8 + 2 + 8 + event.payload.len();
        if buffer.len() < required {
            return Err(EncodeError::BufferOverflow {
                needed: required,
                available: buffer.len(),
            });
        }
        buffer[0] = Self::MOCK_SENTINEL;
        buffer[1..9].copy_from_slice(&event.network_id.0.to_le_bytes());
        buffer[9..11].copy_from_slice(&event.component_kind.0.to_le_bytes());
        buffer[11..19].copy_from_slice(&event.tick.to_le_bytes());
        buffer[19..required].copy_from_slice(&event.payload);
        Ok(required)
    }

    fn decode(&self, buffer: &[u8]) -> Result<ComponentUpdate, EncodeError> {
        if buffer.len() < 19 {
            return Err(EncodeError::MalformedPayload {
                offset: 0,
                message: "Buffer too small for mock header".to_string(),
            });
        }
        if buffer[0] == Self::MOCK_ERROR_BYTE {
            return Err(EncodeError::MalformedPayload {
                offset: 0,
                message: "Triggered artificial MOCK_ERROR_BYTE".to_string(),
            });
        }
        if buffer[0] != Self::MOCK_SENTINEL {
            return Err(EncodeError::MalformedPayload {
                offset: 0,
                message: format!(
                    "Invalid sentinel: expected {:#x}, got {:#x}",
                    Self::MOCK_SENTINEL,
                    buffer[0]
                ),
            });
        }

        let network_id = u64::from_le_bytes(buffer[1..9].try_into().unwrap());
        let component_kind = u16::from_le_bytes(buffer[9..11].try_into().unwrap());
        let tick = u64::from_le_bytes(buffer[11..19].try_into().unwrap());

        Ok(ComponentUpdate {
            network_id: NetworkId(network_id),
            component_kind: ComponentKind(component_kind),
            payload: buffer[19..].to_vec(),
            tick,
        })
    }
    fn encode_event(&self, event: &NetworkEvent) -> Result<Vec<u8>, EncodeError> {
        match event {
            NetworkEvent::Auth { .. } => Ok(vec![b'A']),
            _ => Err(EncodeError::Io(std::io::Error::other(format!(
                "MockEncoder: encoding not implemented for {event:?}"
            )))),
        }
    }

    fn decode_event(&self, data: &[u8]) -> Result<NetworkEvent, EncodeError> {
        if data.is_empty() {
            return Err(EncodeError::MalformedPayload {
                offset: 0,
                message: "Empty event data".to_string(),
            });
        }
        // For testing purposes, if the first byte is 'A', we treat it as an Auth event
        if data[0] == b'A' {
            return Ok(NetworkEvent::Auth {
                session_token: "mock_token".to_string(),
            });
        }
        Err(EncodeError::MalformedPayload {
            offset: 0,
            message: format!("Unexpected first byte for mock event: {:#x}", data[0]),
        })
    }
    fn max_encoded_size(&self) -> usize {
        crate::MAX_SAFE_PAYLOAD_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    const fn assert_transport_bounds<T: GameTransport>() {}
    #[allow(dead_code)]
    const fn assert_world_bounds<T: WorldState>() {}
    #[allow(dead_code)]
    const fn assert_encoder_bounds<T: Encoder>() {}

    // Compile-time proof that test doubles satisfy the trait bounds
    #[test]
    fn test_compile_bounds() {
        assert_transport_bounds::<MockTransport>();
        assert_world_bounds::<MockWorldState>();
        assert_encoder_bounds::<MockEncoder>();
    }

    // T220.50 — Full 5-stage tick loop
    #[tokio::test]
    async fn test_tick_loop_integration() {
        let mut transport = MockTransport::new();
        let mut world = MockWorldState::new();
        let encoder = MockEncoder::new();

        // Simulate 1000 ticks
        for tick in 1..=1000 {
            // Stage 1: Poll Network
            let mut events = transport.poll_events().await.unwrap();
            if tick % 100 == 0 {
                let cid = ClientId(tick);
                transport.connect(cid);
                events.push(NetworkEvent::ClientConnected(cid));
            }

            // Stage 2 & 3: Decode & Apply
            let mut updates = Vec::new();
            for event in events {
                if let NetworkEvent::UnreliableMessage { data, client_id } = event
                    && let Ok(update) = encoder.decode(&data)
                {
                    updates.push((client_id, update));
                }
            }
            world.apply_updates(&updates);

            // Stage 3: Simulate
            world.simulate();

            // Mutation
            if tick % 50 == 0 {
                let ent = world.spawn_networked();
                world.queue_delta(ReplicationEvent {
                    network_id: ent,
                    component_kind: ComponentKind(1),
                    payload: vec![1, 2, 3],
                    tick,
                });
            }

            // Stage 4: Extract Deltas
            let deltas = world.extract_deltas();

            // Stage 5: Serialize and dispatch
            for delta in deltas {
                let mut buf = vec![0u8; 1500];
                let size = encoder.encode(&delta, &mut buf).unwrap();
                let _ = transport.broadcast_unreliable(&buf[..size]).await;
            }
        }
    }
}
