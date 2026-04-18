//! Protocol-level primitive types.
use serde::{Deserialize, Serialize};

/// A globally unique entity identifier used in all network communication.
/// Assigned by the server. Immutable for the lifetime of the entity.
///
/// This is NOT the ECS's internal entity ID. The `WorldState` adapter
/// translates between `NetworkId` and the ECS's local handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NetworkId(pub u64);

/// The ECS's internal entity handle. Opaque to the network layer.
/// In Phase 1 (Bevy), this wraps `bevy_ecs::entity::Entity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalId(pub u64);

/// A unique identifier for a connected client session.
/// Assigned by the transport layer on connection, released on disconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);

/// A component type identifier. Used by the Encoder to determine
/// how to serialize/deserialize a specific component's fields.
///
/// In Phase 1, this is a simple enum discriminant.
/// In Phase 3, this may become a compile-time type hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ComponentKind(pub u16);

/// Standard transform component used for replication (`ComponentKind` 1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct Transform {
    /// Position X
    pub x: f32,
    /// Position Y
    pub y: f32,
    /// Position Z
    pub z: f32,
    /// Rotation in radians
    pub rotation: f32,
    /// Entity type (Phase 1 / Playground only)
    pub entity_type: u16,
}

use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AllocatorError {
    #[error("NetworkId overflow (reached u64::MAX)")]
    Overflow,
    #[error("NetworkId allocator exhausted (reached limit)")]
    Exhausted,
}

/// Authoritative allocator for [`NetworkId`]s.
///
/// Used by the server to ensure IDs are unique and monotonically increasing.
/// Thread-safe and lock-free.
#[derive(Debug)]
pub struct NetworkIdAllocator {
    start_id: u64,
    next: AtomicU64,
}

impl Default for NetworkIdAllocator {
    fn default() -> Self {
        Self::new(1)
    }
}

impl NetworkIdAllocator {
    /// Creates a new allocator starting from a specific ID. 0 is reserved.
    #[must_use]
    pub fn new(start_id: u64) -> Self {
        Self {
            start_id,
            next: AtomicU64::new(start_id),
        }
    }

    /// Allocates a new unique [`NetworkId`].
    ///
    /// # Errors
    /// Returns [`AllocatorError::Overflow`] if the next ID would exceed `u64::MAX`.
    pub fn allocate(&self) -> Result<NetworkId, AllocatorError> {
        let val = self.next.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |curr| {
            if curr == u64::MAX {
                None
            } else {
                Some(curr + 1)
            }
        }).map_err(|_| AllocatorError::Overflow)?;

        if val == 0 {
            return Err(AllocatorError::Exhausted);
        }

        Ok(NetworkId(val))
    }

    /// Resets the allocator to its initial `start_id`.
    /// Use only in tests or clear-world scenarios.
    pub fn reset(&self) {
        self.next.store(self.start_id, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_derives() {
        let nid1 = NetworkId(42);
        let nid2 = nid1;
        assert_eq!(nid1, nid2);

        let lid1 = LocalId(42);
        let lid2 = LocalId(42);
        assert_eq!(lid1, lid2);

        let cid = ClientId(99);
        assert_eq!(format!("{cid:?}"), "ClientId(99)");

        let kind = ComponentKind(1);
        assert_eq!(kind.0, 1);
    }
}
