//! Core protocol definitions and communication contracts for the Aetheris Engine.
//!
//! **Phase:** All Phases - Stable Core
//! **Constraint:** Minimal dependency footprint.
//! **Purpose:** Defines the Trait Facade (`GameTransport`, `WorldState`, Encoder) that isolates
//! the engine logic from concrete implementations.

#![warn(clippy::all, clippy::pedantic)]

pub mod error;
pub mod events;
pub mod reassembler;
pub mod traits;
pub mod types;

pub use reassembler::Reassembler;

/// Maximum safe payload size for UDP datagrams to avoid fragmentation.
pub const MAX_SAFE_PAYLOAD_SIZE: usize = 1200;

/// Maximum number of fragments allowed for a single message.
/// 1024 * 1136 bytes ~= 1.1 MiB.
pub const MAX_TOTAL_FRAGMENTS: u16 = 1024;

/// Estimated overhead in bytes for a `WireEvent::Fragment` envelope using `rmp-serde`.
/// This includes the enum tag, `FragmentedEvent` fields, and `Vec<u8>` length prefix.
pub const FRAGMENT_OVERHEAD: usize = 64;

/// The maximum bytes of game payload that can fit into a single MTU-safe fragment.
pub const MAX_FRAGMENT_PAYLOAD_SIZE: usize = MAX_SAFE_PAYLOAD_SIZE - FRAGMENT_OVERHEAD;

#[cfg(feature = "grpc")]
/// gRPC Auth services and types generated from `auth.proto`.
#[allow(clippy::all, clippy::pedantic)]
pub mod auth {
    pub mod v1 {
        tonic::include_proto!("aetheris.auth.v1");
    }
}

#[cfg(feature = "grpc")]
/// gRPC Matchmaking services and types generated from `matchmaking.proto`.
#[allow(clippy::all, clippy::pedantic)]
pub mod matchmaking {
    pub mod v1 {
        tonic::include_proto!("aetheris.matchmaking.v1");
    }
}

#[cfg(feature = "grpc")]
/// gRPC Telemetry service and types generated from `telemetry.proto`.
/// This module is the out-of-band diagnostic channel — independent of WebTransport.
#[allow(clippy::all, clippy::pedantic)]
pub mod telemetry {
    pub mod v1 {
        tonic::include_proto!("aetheris.telemetry.v1");
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_doubles;

#[cfg(test)]
mod tests {
    #[test]
    fn test_protocol_foundation() {
        // Basic assertion to satisfy nextest's requirement for at least one test
        let synchronized = true;
        assert!(synchronized);
    }
}
