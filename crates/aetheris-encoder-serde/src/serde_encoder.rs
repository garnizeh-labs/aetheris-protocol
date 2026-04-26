//! Implementation of the `SerdeEncoder` using `rmp-serde`.

use std::io::Cursor;

use serde::{Deserialize, Serialize};

use aetheris_protocol::error::EncodeError;
use aetheris_protocol::events::{ComponentUpdate, NetworkEvent, ReplicationEvent, WireEvent};
use aetheris_protocol::traits::Encoder;
use aetheris_protocol::types::{ClientId, ComponentKind, NetworkId};

/// Internal header for serialized replication events.
///
/// Ensures a stable binary format across different `rmp-serde` configurations.
#[derive(Debug, Serialize, Deserialize)]
struct PacketHeader {
    network_id: NetworkId,
    component_kind: ComponentKind,
    tick: u64,
}

/// A `serde`-based encoder that uses `rmp-serde` (`MessagePack`) for binary serialization.
///
/// This implementation targets Phase 1 MVP requirements for rapid iteration.
/// It uses a fixed-size header followed by the raw component payload.
#[derive(Debug, Default)]
pub struct SerdeEncoder;

impl SerdeEncoder {
    /// Creates a new `SerdeEncoder`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Encodes a `NetworkEvent` into raw bytes for transmission.
    ///
    /// # Errors
    /// Returns `EncodeError` if the event fails to serialize or is a local-only variant.
    pub fn encode_event(
        &self,
        event: &aetheris_protocol::events::NetworkEvent,
    ) -> Result<Vec<u8>, EncodeError> {
        let wire_event = Self::to_wire_event(event)?;
        rmp_serde::to_vec(&wire_event)
            .map_err(|e| EncodeError::Io(std::io::Error::other(e.to_string())))
    }

    /// Encodes a `NetworkEvent` into the provided buffer.
    ///
    /// # Errors
    /// Returns `EncodeError::BufferOverflow` if the buffer is too small.
    pub fn encode_event_into(
        &self,
        event: &NetworkEvent,
        buffer: &mut [u8],
    ) -> Result<usize, EncodeError> {
        let wire_event = Self::to_wire_event(event)?;
        let mut cursor = Cursor::new(&mut *buffer);
        rmp_serde::encode::write(&mut cursor, &wire_event).map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("unexpected end of file") || err_msg.contains("invalid value write")
            {
                EncodeError::BufferOverflow {
                    needed: 256, // Estimate
                    available: cursor.get_ref().len(),
                }
            } else {
                EncodeError::Io(std::io::Error::other(err_msg))
            }
        })?;
        usize::try_from(cursor.position()).map_err(|_| EncodeError::BufferOverflow {
            needed: usize::MAX,
            available: buffer.len(),
        })
    }

    fn to_wire_event(event: &NetworkEvent) -> Result<WireEvent, EncodeError> {
        Ok(match event {
            NetworkEvent::Ping { tick, .. } if event.is_wire() => WireEvent::Ping { tick: *tick },
            NetworkEvent::Pong { tick } => WireEvent::Pong { tick: *tick },
            NetworkEvent::Auth { session_token } => WireEvent::Auth {
                session_token: session_token.clone(),
            },
            NetworkEvent::Fragment { fragment, .. } => WireEvent::Fragment(fragment.clone()),
            NetworkEvent::StressTest { count, rotate, .. } => WireEvent::StressTest {
                count: *count,
                rotate: *rotate,
            },
            NetworkEvent::Spawn {
                entity_type,
                x,
                y,
                rot,
                ..
            } => WireEvent::Spawn {
                entity_type: *entity_type,
                x: *x,
                y: *y,
                rot: *rot,
            },
            NetworkEvent::ClearWorld { .. } => WireEvent::ClearWorld,
            NetworkEvent::StartSession { .. } => WireEvent::StartSession,
            NetworkEvent::RequestSystemManifest { .. } => WireEvent::RequestSystemManifest,
            NetworkEvent::GameEvent { event, .. } => WireEvent::GameEvent(event.clone()),
            NetworkEvent::ReplicationBatch { events, .. } => {
                WireEvent::ReplicationBatch(events.clone())
            }
            _ => {
                return Err(EncodeError::Io(std::io::Error::other(format!(
                    "Cannot encode local-only variant as wire event: {event:?}"
                ))));
            }
        })
    }

    /// Decodes raw bytes into a `NetworkEvent`.
    ///
    /// # Errors
    /// Returns `EncodeError` if the bytes are not a valid `WireEvent`.
    pub fn decode_event(
        &self,
        data: &[u8],
    ) -> Result<aetheris_protocol::events::NetworkEvent, EncodeError> {
        let wire_event: WireEvent = rmp_serde::from_slice(data).map_err(|e| {
            EncodeError::MalformedPayload {
                offset: 0, // In Phase 1 we don't track exact rmp-serde offset easily
                message: e.to_string(),
            }
        })?;

        Ok(match wire_event {
            WireEvent::Ping { tick } => NetworkEvent::Ping {
                client_id: ClientId(0), // Populated by transport/server
                tick,
            },
            WireEvent::Pong { tick } => NetworkEvent::Pong { tick },
            WireEvent::Auth { session_token } => NetworkEvent::Auth { session_token },
            WireEvent::Fragment(fragment) => NetworkEvent::Fragment {
                client_id: ClientId(0),
                fragment,
            },
            WireEvent::StressTest { count, rotate } => NetworkEvent::StressTest {
                client_id: ClientId(0), // Populated by server
                count,
                rotate,
            },
            WireEvent::Spawn {
                entity_type,
                x,
                y,
                rot,
            } => NetworkEvent::Spawn {
                client_id: ClientId(0),
                entity_type,
                x,
                y,
                rot,
            },
            WireEvent::ClearWorld => NetworkEvent::ClearWorld {
                client_id: ClientId(0),
            },
            WireEvent::StartSession => NetworkEvent::StartSession {
                client_id: ClientId(0),
            },
            WireEvent::RequestSystemManifest => NetworkEvent::RequestSystemManifest {
                client_id: ClientId(0),
            },
            WireEvent::GameEvent(event) => NetworkEvent::GameEvent {
                client_id: ClientId(0),
                event,
            },
            WireEvent::ReplicationBatch(events) => NetworkEvent::ReplicationBatch {
                client_id: ClientId(0),
                events,
            },
        })
    }
}

impl Encoder for SerdeEncoder {
    fn codec_id(&self) -> u32 {
        1
    }

    fn encode_event(&self, event: &NetworkEvent) -> Result<Vec<u8>, EncodeError> {
        self.encode_event(event)
    }

    fn encode_event_into(
        &self,
        event: &NetworkEvent,
        buffer: &mut [u8],
    ) -> Result<usize, EncodeError> {
        self.encode_event_into(event, buffer)
    }

    fn decode_event(&self, data: &[u8]) -> Result<NetworkEvent, EncodeError> {
        self.decode_event(data)
    }

    fn encode(&self, event: &ReplicationEvent, buffer: &mut [u8]) -> Result<usize, EncodeError> {
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        let header = PacketHeader {
            network_id: event.network_id,
            component_kind: event.component_kind,
            tick: event.tick,
        };

        let mut cursor = Cursor::new(buffer);
        let mut serializer = rmp_serde::Serializer::new(&mut cursor);

        header.serialize(&mut serializer).map_err(|_e| {
            metrics::counter!("aetheris_encoder_errors_total", "type" => "header_serialize_fail")
                .increment(1);
            // If it fails to serialize, it's likely a buffer overflow.
            EncodeError::BufferOverflow {
                needed: 32, // PacketHeader is small (~20 bytes)
                available: cursor.get_ref().len(),
            }
        })?;

        let header_len = usize::try_from(cursor.position()).unwrap_or(usize::MAX);
        let payload_len = event.payload.len();
        let total_needed = header_len + payload_len;

        if total_needed > cursor.get_ref().len() {
            metrics::counter!("aetheris_encoder_errors_total", "type" => "buffer_overflow")
                .increment(1);
            return Err(EncodeError::BufferOverflow {
                needed: total_needed,
                available: cursor.get_ref().len(),
            });
        }

        // Copy payload manually after the header
        let slice = cursor.into_inner();
        slice[header_len..total_needed].copy_from_slice(&event.payload);

        #[allow(clippy::cast_precision_loss)]
        metrics::histogram!(
            "aetheris_encoder_payload_size_bytes",
            "operation" => "encode"
        )
        .record(total_needed as f64);

        #[cfg(not(target_arch = "wasm32"))]
        metrics::histogram!(
            "aetheris_encoder_encode_duration_seconds",
            "kind" => event.component_kind.0.to_string()
        )
        .record(start.elapsed().as_secs_f64());

        Ok(total_needed)
    }

    fn decode(&self, buffer: &[u8]) -> Result<ComponentUpdate, EncodeError> {
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        let mut cursor = Cursor::new(buffer);
        let mut deserializer = rmp_serde::Deserializer::new(&mut cursor);

        let header = PacketHeader::deserialize(&mut deserializer).map_err(|e| {
            metrics::counter!("aetheris_encoder_errors_total", "type" => "malformed_payload")
                .increment(1);
            EncodeError::MalformedPayload {
                offset: usize::try_from(cursor.position()).unwrap_or(usize::MAX),
                message: e.to_string(),
            }
        })?;

        let header_len = usize::try_from(cursor.position()).unwrap_or(usize::MAX);
        let payload = buffer
            .get(header_len..)
            .ok_or(EncodeError::MalformedPayload {
                offset: header_len,
                message: "Payload slice out of bounds".to_string(),
            })?
            .to_vec();

        #[allow(clippy::cast_precision_loss)]
        metrics::histogram!(
            "aetheris_encoder_payload_size_bytes",
            "operation" => "decode"
        )
        .record(buffer.len() as f64);

        #[cfg(not(target_arch = "wasm32"))]
        metrics::histogram!(
            "aetheris_encoder_decode_duration_seconds",
            "kind" => header.component_kind.0.to_string()
        )
        .record(start.elapsed().as_secs_f64());

        Ok(ComponentUpdate {
            network_id: header.network_id,
            component_kind: header.component_kind,
            payload,
            tick: header.tick,
        })
    }

    fn max_encoded_size(&self) -> usize {
        aetheris_protocol::MAX_SAFE_PAYLOAD_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_roundtrip() {
        let encoder = SerdeEncoder::new();
        let event = ReplicationEvent {
            network_id: NetworkId(42),
            component_kind: ComponentKind(1),
            payload: vec![1, 2, 3, 4],
            tick: 100,
        };

        let mut buffer = [0u8; 1200];
        let bytes_written = encoder.encode(&event, &mut buffer).unwrap();
        assert!(bytes_written > 0);

        let update = encoder.decode(&buffer[..bytes_written]).unwrap();
        assert_eq!(update.network_id, event.network_id);
        assert_eq!(update.component_kind, event.component_kind);
        assert_eq!(update.tick, event.tick);
        assert_eq!(update.payload, event.payload);
    }
    #[test]
    fn test_fragment_roundtrip() {
        let encoder = SerdeEncoder::new();
        let fragment = aetheris_protocol::events::FragmentedEvent {
            message_id: 123,
            fragment_index: 1,
            total_fragments: 5,
            payload: vec![1, 2, 3],
        };

        let event = NetworkEvent::Fragment {
            client_id: aetheris_protocol::types::ClientId(0),
            fragment: fragment.clone(),
        };

        let output = encoder.encode_event(&event).unwrap();
        let decoded = encoder.decode_event(&output).unwrap();

        if let NetworkEvent::Fragment {
            client_id: _,
            fragment: decoded_fragment,
        } = decoded
        {
            assert_eq!(decoded_fragment.message_id, fragment.message_id);
            assert_eq!(decoded_fragment.fragment_index, fragment.fragment_index);
            assert_eq!(decoded_fragment.total_fragments, fragment.total_fragments);
            assert_eq!(decoded_fragment.payload, fragment.payload);
        } else {
            panic!("Decoded event is not a Fragment: {decoded:?}");
        }
    }

    #[test]
    fn test_buffer_overflow() {
        let encoder = SerdeEncoder::new();
        let event = ReplicationEvent {
            network_id: NetworkId(42),
            component_kind: ComponentKind(1),
            payload: vec![1, 2, 3, 4],
            tick: 100,
        };

        let mut small_buffer = [0u8; 1];
        let result = encoder.encode(&event, &mut small_buffer);
        assert!(matches!(result, Err(EncodeError::BufferOverflow { .. })));
    }

    #[test]
    fn test_malformed_payload() {
        let encoder = SerdeEncoder::new();
        let garbage = [0xff, 0xff, 0xff, 0xff];
        let result = encoder.decode(&garbage);
        if let Err(EncodeError::MalformedPayload { message, .. }) = result {
            assert!(!message.is_empty());
        } else {
            panic!("Expected MalformedPayload error, got {result:?}");
        }
    }

    proptest! {
        #[test]
        fn test_fuzz_decode(ref bytes in any::<Vec<u8>>()) {
            let encoder = SerdeEncoder::new();
            // Should never panic
            let _ = encoder.decode(bytes);
        }

        #[test]
        fn test_fuzz_roundtrip(
            nid in any::<u64>(),
            kind in any::<u16>(),
            tick in any::<u64>(),
            ref payload in any::<Vec<u8>>()
        ) {
            let encoder = SerdeEncoder::new();
            let event = ReplicationEvent {
                network_id: NetworkId(nid),
                component_kind: ComponentKind(kind),
                payload: payload.clone(),
                tick,
            };

            let mut buffer = vec![0u8; 2048 + payload.len()];
            if let Ok(written) = encoder.encode(&event, &mut buffer) {
                let update = encoder.decode(&buffer[..written])
                    .expect("Round-trip decode failed during fuzzed test");
                assert_eq!(update.network_id, event.network_id);
                assert_eq!(update.component_kind, event.component_kind);
                assert_eq!(update.tick, event.tick);
                assert_eq!(update.payload, event.payload);
            }
        }
    }

    #[test]
    fn test_disconnected_not_serializable() {
        let encoder = SerdeEncoder::new();
        let event = NetworkEvent::Disconnected(ClientId(42));

        // Attempting to encode a local-only event should return an error
        let result = encoder.encode_event(&event);
        assert!(result.is_err());
        if let Err(EncodeError::Io(e)) = result {
            assert!(e.to_string().contains("Cannot encode local-only variant"));
        } else {
            panic!("Expected EncodeError::Io with local-only message, got {result:?}");
        }
    }

    #[test]
    fn test_game_event_roundtrip() {
        use aetheris_protocol::events::GameEvent;
        use aetheris_protocol::types::NetworkId;

        let encoder = SerdeEncoder::new();
        let game_event = GameEvent::AsteroidDepleted {
            network_id: NetworkId(123),
        };
        let event = NetworkEvent::GameEvent {
            client_id: ClientId(1), // Should be masked to 0 on wire and restored on server poll
            event: game_event.clone(),
        };

        let output = encoder.encode_event(&event).unwrap();
        let decoded = encoder.decode_event(&output).unwrap();

        if let NetworkEvent::GameEvent {
            client_id,
            event: decoded_event,
        } = decoded
        {
            assert_eq!(
                client_id,
                ClientId(0),
                "Wire decoding should default client_id to 0"
            );
            match decoded_event {
                GameEvent::AsteroidDepleted { network_id } => {
                    assert_eq!(network_id, NetworkId(123));
                }
                GameEvent::Possession { .. }
                | GameEvent::SystemManifest { .. }
                | GameEvent::DamageEvent { .. }
                | GameEvent::DeathEvent { .. }
                | GameEvent::RespawnEvent { .. }
                | GameEvent::CargoCollected { .. } => {
                    panic!("Unexpected event type in roundtrip test");
                }
            }
        } else {
            panic!("Decoded event is not a GameEvent: {decoded:?}");
        }
    }
}
