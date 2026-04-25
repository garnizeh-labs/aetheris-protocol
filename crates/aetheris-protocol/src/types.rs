//! Protocol-level primitive types.
pub const PROTOCOL_VERSION: u32 = 3;

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
/// - `0–1023` (except 128): Engine Core (Replicated).
/// - `1024–2047`: Official Engine Extensions.
/// - `128`: Explicitly reserved for Input Commands (Transient/Inbound-Only).
/// - `32768+`: Reserved for Non-Replicated/Inbound variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ComponentKind(pub u16);

/// Discriminant for client-to-server input commands.
/// Tagged as Transient/Inbound-Only.
pub const INPUT_COMMAND_KIND: ComponentKind = ComponentKind(128);

/// Replicated component for Room Definition.
pub const ROOM_DEFINITION_KIND: ComponentKind = ComponentKind(129);

/// Replicated component for Room Bounds.
pub const ROOM_BOUNDS_KIND: ComponentKind = ComponentKind(130);

/// Replicated component for Room Membership.
pub const ROOM_MEMBERSHIP_KIND: ComponentKind = ComponentKind(131);

/// Replicated component for the mining laser beam state.
pub const MINING_BEAM_KIND: ComponentKind = ComponentKind(1024);

/// Replicated component for ship cargo state (replicated to owner).
pub const CARGO_HOLD_KIND: ComponentKind = ComponentKind(1025);

/// Replicated component for asteroid ore depletion tracking.
pub const ASTEROID_KIND: ComponentKind = ComponentKind(1026);

/// Replicated component for primary weapon state.
pub const WEAPON_KIND: ComponentKind = ComponentKind(1027);

/// Replicated component for shield pool state.
pub const SHIELD_POOL_KIND: ComponentKind = ComponentKind(1028);

/// Replicated component for hull pool state.
pub const HULL_POOL_KIND: ComponentKind = ComponentKind(1029);

/// Replicated component for cargo drop state.
pub const CARGO_DROP_KIND: ComponentKind = ComponentKind(1030);

/// Action bitflag: fire primary weapon.
pub const ACTION_FIRE_WEAPON: u32 = 1 << 2;

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

/// Individual input actions performed by a player in a single tick.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PlayerInputKind {
    /// Directional thrust/movement.
    Move { x: f32, y: f32 },
    /// Toggle mining beam on a specific target.
    ToggleMining { target: NetworkId },
    /// Fire primary weapon (for VS-03).
    FirePrimary,
}

/// Maximum allowed actions in a single `InputCommand` to prevent payload `DoS`.
/// Chosen to stay well within `MAX_SAFE_PAYLOAD_SIZE` (1200 bytes).
pub const MAX_ACTIONS: usize = 128;

/// Aggregated user input for a single simulation tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputCommand {
    /// The client-side tick this input was generated at.
    pub tick: u64,
    /// List of actions performed in this tick.
    pub actions: Vec<PlayerInputKind>,
    /// Bitmask of actions for high-frequency binary inputs.
    #[serde(default)]
    pub actions_mask: u32,
    /// The tick of the last server state the client saw before sending this input.
    pub last_seen_input_tick: Option<u64>,
}

impl InputCommand {
    /// Returns a new `InputCommand` with all `Move` inputs clamped to [-1.0, 1.0].
    #[must_use]
    pub fn clamped(mut self) -> Self {
        for action in &mut self.actions {
            if let PlayerInputKind::Move { x, y } = action {
                *x = x.clamp(-1.0, 1.0);
                *y = y.clamp(-1.0, 1.0);
            }
        }
        self
    }

    /// Validates the command against protocol constraints.
    ///
    /// # Errors
    /// Returns an error message if the command exceeds `MAX_ACTIONS`.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.actions.len() > MAX_ACTIONS {
            return Err("Too many actions in InputCommand");
        }
        Ok(())
    }
}

/// Replicated state for a ship's mining beam.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct MiningBeam {
    pub active: bool,
    pub target: Option<NetworkId>,
    #[serde(default)]
    pub mining_range: f32,
    #[serde(default)]
    pub base_mining_rate: u16,
}

/// Replicated state for a ship's cargo hold.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct CargoHold {
    pub ore_count: u16,
    pub capacity: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Asteroid {
    pub ore_remaining: u16,
    pub total_capacity: u16,
}

/// Replicated state for a ship's primary weapon.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Weapon {
    pub cooldown_ticks: u16,
    pub last_fired_tick: u64,
}

/// Replicated state for a ship's shield pool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ShieldPool {
    pub current: u16,
    pub max: u16,
}

/// Replicated state for a ship's hull pool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct HullPool {
    pub current: u16,
    pub max: u16,
}

/// Replicated state for a cargo drop entity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct CargoDrop {
    pub quantity: u16,
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

/// Maximum byte length (UTF-8) for [`RoomName`] and [`PermissionString`].
///
/// Chosen well below [`MAX_SAFE_PAYLOAD_SIZE`](crate::MAX_SAFE_PAYLOAD_SIZE)
/// to leave ample room for the surrounding struct framing in the wire format.
pub const MAX_ROOM_STRING_BYTES: usize = 64;

/// Error returned when a [`RoomName`] or [`PermissionString`] exceeds the
/// allowed byte length.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("string too long: {len} bytes exceeds the maximum of {max} bytes")]
pub struct RoomStringError {
    /// Actual byte length of the rejected string.
    pub len: usize,
    /// Maximum allowed byte length ([`MAX_ROOM_STRING_BYTES`]).
    pub max: usize,
}

/// A validated room name.
///
/// Guaranteed not to exceed [`MAX_ROOM_STRING_BYTES`] bytes (UTF-8).
/// The limit is enforced at construction time via [`RoomName::new`] and at
/// Serde decode time, so a value held in this type can never produce a payload
/// that exceeds [`MAX_SAFE_PAYLOAD_SIZE`](crate::MAX_SAFE_PAYLOAD_SIZE).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RoomName(String);

impl RoomName {
    /// Creates a `RoomName`, returning [`RoomStringError`] if `s` exceeds
    /// [`MAX_ROOM_STRING_BYTES`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RoomStringError`] if the byte length of `s` exceeds
    /// [`MAX_ROOM_STRING_BYTES`].
    #[must_use = "the validated RoomName must be used"]
    pub fn new(s: impl Into<String>) -> Result<Self, RoomStringError> {
        let s = s.into();
        if s.len() > MAX_ROOM_STRING_BYTES {
            return Err(RoomStringError {
                len: s.len(),
                max: MAX_ROOM_STRING_BYTES,
            });
        }
        Ok(Self(s))
    }

    /// Returns the name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RoomName {
    type Error = RoomStringError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<RoomName> for String {
    fn from(n: RoomName) -> String {
        n.0
    }
}

impl std::fmt::Display for RoomName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A validated access-control permission token.
///
/// Guaranteed not to exceed [`MAX_ROOM_STRING_BYTES`] bytes (UTF-8).
/// Used by [`RoomAccessPolicy::Permission`].
/// The limit is enforced at construction time via [`PermissionString::new`] and
/// at Serde decode time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PermissionString(String);

impl PermissionString {
    /// Creates a `PermissionString`, returning [`RoomStringError`] if `s`
    /// exceeds [`MAX_ROOM_STRING_BYTES`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RoomStringError`] if the byte length of `s` exceeds
    /// [`MAX_ROOM_STRING_BYTES`].
    #[must_use = "the validated PermissionString must be used"]
    pub fn new(s: impl Into<String>) -> Result<Self, RoomStringError> {
        let s = s.into();
        if s.len() > MAX_ROOM_STRING_BYTES {
            return Err(RoomStringError {
                len: s.len(),
                max: MAX_ROOM_STRING_BYTES,
            });
        }
        Ok(Self(s))
    }

    /// Returns the permission token as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PermissionString {
    type Error = RoomStringError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<PermissionString> for String {
    fn from(p: PermissionString) -> String {
        p.0
    }
}

impl std::fmt::Display for PermissionString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Access control policy for the room.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomAccessPolicy {
    /// Anyone can enter.
    Open,
    /// Only clients holding the specified [`PermissionString`] token can enter.
    ///
    /// The token is replicated verbatim in the wire format and is guaranteed
    /// not to exceed [`MAX_ROOM_STRING_BYTES`] bytes.
    Permission(PermissionString),
    /// Only explicitly invited clients can enter.
    InviteOnly,
    /// Locked — no one can enter.
    Locked,
}

/// Defines a spatial region as a Room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDefinition {
    /// Human-readable room identifier.
    ///
    /// Replicated verbatim in the wire format. Guaranteed not to exceed
    /// [`MAX_ROOM_STRING_BYTES`] bytes (UTF-8) by the [`RoomName`] type.
    pub name: RoomName,
    pub capacity: u32,
    pub access: RoomAccessPolicy,
    pub is_template: bool,
}

/// Spatial bounds of the room in world coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RoomBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// Defines which Room an entity currently belongs to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RoomMembership(pub NetworkId);

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
            actions: vec![PlayerInputKind::Move { x: 2.0, y: -5.0 }],
            actions_mask: 0,
            last_seen_input_tick: None,
        };
        let clamped = cmd.clamped();
        if let PlayerInputKind::Move { x, y } = clamped.actions[0] {
            assert!((x - 1.0).abs() < f32::EPSILON);
            assert!((y - -1.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected Move action");
        }

        let valid = InputCommand {
            tick: 1,
            actions: vec![PlayerInputKind::Move { x: 0.5, y: -0.2 }],
            actions_mask: 0,
            last_seen_input_tick: None,
        };
        let clamped = valid.clamped();
        if let PlayerInputKind::Move { x, y } = clamped.actions[0] {
            assert!((x - 0.5).abs() < f32::EPSILON);
            assert!((y - -0.2).abs() < f32::EPSILON);
        } else {
            panic!("Expected Move action");
        }
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
