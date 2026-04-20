//! Protocol-level primitive types.
pub const PROTOCOL_VERSION: u32 = 2;

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
///
/// ### Reservation Policy (M1020/M1015):
/// - `0–1023`: Engine Core (Replicated).
/// - `1024–2047`: Official Engine Extensions.
/// - `128`: Explicitly reserved for Input Commands (Transient/Inbound-Only).
/// - `32768+`: Reserved for Non-Replicated/Inbound variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ComponentKind(pub u16);

/// Discriminant for client-to-server input commands.
/// Tagged as Transient/Inbound-Only.
pub const INPUT_COMMAND_KIND: ComponentKind = ComponentKind(128);

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
    /// The high-level entity type identifier for early client rendering.
    pub entity_type: u16,
}

/// Ship classification for rendering and stat selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ShipClass {
    Interceptor = 0,
    Dreadnought = 1,
    Hauler = 2,
}

/// Unique identifier for a weapon type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WeaponId(pub u8);

/// A globally unique sector/room identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SectorId(pub u64);

/// Material types extracted from asteroids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum OreType {
    RawOre = 0,
}

/// Projectile delivery classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProjectileType {
    PulseLaser = 0,
    SeekerMissile = 1,
}

/// NPC Drone behavior state Machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AIState {
    Patrol = 0,
    Aggro = 1,
    Combat = 2,
    Return = 3,
}

/// Definitive respawn target semantics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RespawnLocation {
    /// The server calculates dynamically the Nearest Safe Zone.
    NearestSafeZone,
    /// Respawn docked at a specific station entity.
    Station(u64),
    /// Respawn at arbitrary x, y coordinates (admin/debug).
    Coordinate(f32, f32),
}

/// Aggregated user input for a single simulation tick.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InputCommand {
    /// The client-side tick this input was generated at.
    pub tick: u64,
    /// Movement X [-1.0, 1.0]
    pub move_x: f32,
    /// Movement Y [-1.0, 1.0]
    pub move_y: f32,
    /// Bitmask for actions (M1028 bits: 1=Primary, 2=Secondary, 4=Interact).
    pub actions: u32,
}

impl InputCommand {
    /// Returns a new `InputCommand` with `move_x` and `move_y` clamped to the [-1.0, 1.0] range.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.move_x = self.move_x.clamp(-1.0, 1.0);
        self.move_y = self.move_y.clamp(-1.0, 1.0);
        self
    }
}

/// Basic vitals for any ship entity.
///
/// NOTE: Zero values in maxima (`max_hp`, `max_shield`, `max_energy`) represent an uninitialized
/// or dead state. Logic that performs divisions or percentage calculations must verify
/// non-zero maxima.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShipStats {
    pub hp: u16,
    pub max_hp: u16,
    pub shield: u16,
    pub max_shield: u16,
    pub energy: u16,
    pub max_energy: u16,
    pub shield_regen_per_s: u16,
    pub energy_regen_per_s: u16,
}

impl Default for ShipStats {
    /// Returns a baseline valid state (100 HP/Shield/Energy).
    fn default() -> Self {
        Self {
            hp: 100,
            max_hp: 100,
            shield: 100,
            max_shield: 100,
            energy: 100,
            max_energy: 100,
            shield_regen_per_s: 0,
            energy_regen_per_s: 0,
        }
    }
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
        let val = self
            .next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |curr| {
                if curr == u64::MAX {
                    None
                } else {
                    Some(curr + 1)
                }
            })
            .map_err(|_| AllocatorError::Overflow)?;

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

    #[test]
    fn test_input_command_clamping() {
        let cmd = InputCommand {
            tick: 1,
            move_x: 2.0,
            move_y: -5.0,
            actions: 0,
        };
        let clamped = cmd.clamped();
        assert!((clamped.move_x - 1.0).abs() < f32::EPSILON);
        assert!((clamped.move_y - -1.0).abs() < f32::EPSILON);

        let valid = InputCommand {
            tick: 1,
            move_x: 0.5,
            move_y: -0.2,
            actions: 0,
        };
        let clamped = valid.clamped();
        assert!((clamped.move_x - 0.5).abs() < f32::EPSILON);
        assert!((clamped.move_y - -0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ship_stats_non_zero_default() {
        let stats = ShipStats::default();
        assert!(stats.max_hp > 0);
        assert!(stats.max_shield > 0);
        assert!(stats.max_energy > 0);
        assert_eq!(stats.hp, stats.max_hp);
    }
}
