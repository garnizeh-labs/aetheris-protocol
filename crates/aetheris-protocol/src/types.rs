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
/// Minimum ID value for dynamically allocated entities (Agents, Beams, etc).
/// IDs below this value are reserved for static world infrastructure (Workspaces, Documents).
pub const MIN_DYNAMIC_NETWORK_ID: u64 = 100;

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

/// Replicated component for Workspace Definition.
pub const WORKSPACE_DEFINITION_KIND: ComponentKind = ComponentKind(129);

/// Replicated component for Workspace Bounds.
pub const WORKSPACE_BOUNDS_KIND: ComponentKind = ComponentKind(130);

/// Replicated component for Workspace Membership.
pub const WORKSPACE_MEMBERSHIP_KIND: ComponentKind = ComponentKind(131);

/// Replicated component for the extraction beam state.
pub const EXTRACTION_BEAM_KIND: ComponentKind = ComponentKind(1024);

/// Replicated component for agent data store state (replicated to owner).
pub const DATA_STORE_KIND: ComponentKind = ComponentKind(1025);

/// Replicated component for resource payload depletion tracking.
pub const RESOURCE_KIND: ComponentKind = ComponentKind(1026);

/// Replicated component for primary tool state.
pub const TOOL_KIND: ComponentKind = ComponentKind(1027);

/// Replicated component for priority pool state.
pub const PRIORITY_POOL_KIND: ComponentKind = ComponentKind(1028);

/// Replicated component for integrity pool state.
pub const INTEGRITY_POOL_KIND: ComponentKind = ComponentKind(1029);

/// Replicated component for data drop state.
pub const DATA_DROP_KIND: ComponentKind = ComponentKind(1030);

/// Replicated component for beam marker state.
///
/// NOTE: This is intentionally an Engine Core foundational component (Kind < 1024).
pub const BEAM_MARKER_KIND: ComponentKind = ComponentKind(13);

/// Action bitflag: use primary tool.
pub const ACTION_USE_TOOL: u32 = 1 << 2;

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

/// Agent classification for rendering and property selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AgentKind {
    Standard = 0,
    Heavy = 1,
    Carrier = 2,
}

/// Constant identifiers for entity types used in replication and rendering.
pub const ENTITY_TYPE_AGENT: u16 = 1;
pub const ENTITY_TYPE_AI_AGENT: u16 = 2;
pub const ENTITY_TYPE_HEAVY_AGENT: u16 = 3;
pub const ENTITY_TYPE_CARRIER_AGENT: u16 = 4;
pub const ENTITY_TYPE_RESOURCE: u16 = 5;
pub const ENTITY_TYPE_DATA_DROP: u16 = 6;
pub const ENTITY_TYPE_TRAINING_TARGET: u16 = 10;
pub const ENTITY_TYPE_BEAM: u16 = 20;

/// Returns the default authoritative vitals (`max_integrity`, `max_priority`) for a given entity type.
///
/// These values are the single source of truth for UI and early client-side prediction
/// before authoritative `AgentProperties` updates arrive.
#[must_use]
pub const fn get_default_properties(entity_type: u16) -> (u16, u16) {
    match entity_type {
        ENTITY_TYPE_AGENT | ENTITY_TYPE_AI_AGENT => (200, 100),
        ENTITY_TYPE_HEAVY_AGENT => (1500, 500),
        ENTITY_TYPE_CARRIER_AGENT => (600, 200),
        ENTITY_TYPE_RESOURCE => (500, 0),
        ENTITY_TYPE_TRAINING_TARGET => (100, 50),
        ENTITY_TYPE_DATA_DROP | ENTITY_TYPE_BEAM => (1, 0),
        _ => (100, 100),
    }
}

/// Unique identifier for a tool type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ToolId(pub u8);

/// A globally unique zone identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ZoneId(pub u64);

/// Payload types extracted from resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PayloadType {
    RawPayload = 0,
}

/// Beam delivery classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum InteractionBeamType {
    PulseBeam = 0,
    TrackingBeam = 1,
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
    /// Toggle extraction beam on a specific target.
    ToggleExtraction { target: NetworkId },
    /// Fire primary tool (for VS-03).
    FireTool,
    /// Cursor movement for external compositors or UI integration.
    CursorMove {
        /// Normalized X position (0.0 to 1.0)
        x: f32,
        /// Normalized Y position (0.0 to 1.0)
        y: f32,
    },
}

/// Maximum allowed actions in a single `InputCommand` to prevent payload `DoS`.
/// Chosen to stay well within `MAX_SAFE_PAYLOAD_SIZE` (1200 bytes).
pub const MAX_ACTIONS: usize = 128;

/// Bitmask of all currently supported action flags.
pub const ALLOWED_ACTIONS_MASK: u32 = ACTION_USE_TOOL;

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
            match action {
                PlayerInputKind::Move { x, y } => {
                    *x = x.clamp(-1.0, 1.0);
                    *y = y.clamp(-1.0, 1.0);
                }
                PlayerInputKind::CursorMove { x, y } => {
                    *x = x.clamp(0.0, 1.0);
                    *y = y.clamp(0.0, 1.0);
                }
                PlayerInputKind::ToggleExtraction { .. } | PlayerInputKind::FireTool => {}
            }
        }
        self
    }

    /// Validates the command against protocol constraints.
    ///
    /// # Errors
    /// Returns an error message if the command exceeds `MAX_ACTIONS` or has unknown bits in `actions_mask`.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.actions.len() > MAX_ACTIONS {
            return Err("Too many actions in InputCommand");
        }
        if (self.actions_mask & !ALLOWED_ACTIONS_MASK) != 0 {
            return Err("Unknown bits in actions_mask");
        }
        Ok(())
    }
}

/// Replicated state for an agent's extraction beam.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ExtractionBeam {
    pub active: bool,
    pub target: Option<NetworkId>,
    #[serde(default)]
    pub extraction_range: f32,
    #[serde(default)]
    pub base_extraction_rate: u16,
}

/// Replicated state for an agent's data store.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DataStore {
    pub payload_count: u16,
    pub capacity: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Resource {
    pub payload_remaining: u16,
    pub total_capacity: u16,
}

/// Replicated state for an agent's primary tool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Tool {
    pub cooldown_ticks: u16,
    pub last_fired_tick: u64,
}

/// Replicated state for an agent's priority pool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PriorityPool {
    pub current: u16,
    pub max: u16,
}

/// Replicated state for an agent's integrity pool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct IntegrityPool {
    pub current: u16,
    pub max: u16,
}

/// Replicated state for a data drop entity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DataDrop {
    pub amount: u16,
}

/// Basic properties for any agent entity.
///
/// NOTE: Zero values in maxima (`max_integrity`, `max_priority`, `max_energy`) represent an uninitialized
/// or dead state. Logic that performs divisions or percentage calculations must verify
/// non-zero maxima.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AgentProperties {
    pub integrity: u16,
    pub max_integrity: u16,
    pub priority: u16,
    pub max_priority: u16,
    pub energy: u16,
    pub max_energy: u16,
    pub priority_regen_per_s: u16,
    pub energy_regen_per_s: u16,
}

impl Default for AgentProperties {
    /// Returns a baseline valid state (100 Integrity/Priority/Energy).
    fn default() -> Self {
        Self {
            integrity: 100,
            max_integrity: 100,
            priority: 100,
            max_priority: 100,
            energy: 100,
            max_energy: 100,
            priority_regen_per_s: 0,
            energy_regen_per_s: 0,
        }
    }
}

/// Maximum byte length (UTF-8) for [`WorkspaceName`] and [`PermissionString`].
///
/// Chosen well below [`MAX_SAFE_PAYLOAD_SIZE`](crate::MAX_SAFE_PAYLOAD_SIZE)
/// to leave ample room for the surrounding struct framing in the wire format.
pub const MAX_WORKSPACE_STRING_BYTES: usize = 64;

/// Error returned when a [`WorkspaceName`] or [`PermissionString`] exceeds the
/// allowed byte length.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("string too long: {len} bytes exceeds the maximum of {max} bytes")]
pub struct WorkspaceStringError {
    /// Actual byte length of the rejected string.
    pub len: usize,
    /// Maximum allowed byte length ([`MAX_WORKSPACE_STRING_BYTES`]).
    pub max: usize,
}

/// A validated workspace name.
///
/// Guaranteed not to exceed [`MAX_WORKSPACE_STRING_BYTES`] bytes (UTF-8).
/// The limit is enforced at construction time via [`WorkspaceName::new`] and at
/// Serde decode time, so a value held in this type can never produce a payload
/// that exceeds [`MAX_SAFE_PAYLOAD_SIZE`](crate::MAX_SAFE_PAYLOAD_SIZE).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkspaceName(String);

fn validate_workspace_string(s: &str) -> Result<(), WorkspaceStringError> {
    if s.len() > MAX_WORKSPACE_STRING_BYTES {
        return Err(WorkspaceStringError {
            len: s.len(),
            max: MAX_WORKSPACE_STRING_BYTES,
        });
    }
    Ok(())
}

impl WorkspaceName {
    /// Creates a `WorkspaceName`, returning [`WorkspaceStringError`] if `s` exceeds
    /// [`MAX_WORKSPACE_STRING_BYTES`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceStringError`] if the byte length of `s` exceeds
    /// [`MAX_WORKSPACE_STRING_BYTES`].
    #[must_use = "the validated WorkspaceName must be used"]
    pub fn new(s: impl Into<String>) -> Result<Self, WorkspaceStringError> {
        let s = s.into();
        validate_workspace_string(&s)?;
        Ok(Self(s))
    }

    /// Returns the name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WorkspaceName {
    type Error = WorkspaceStringError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<WorkspaceName> for String {
    fn from(n: WorkspaceName) -> String {
        n.0
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A validated access-control permission token.
///
/// Guaranteed not to exceed [`MAX_WORKSPACE_STRING_BYTES`] bytes (UTF-8).
/// Used by [`WorkspaceAccessPolicy::Permission`].
/// The limit is enforced at construction time via [`PermissionString::new`] and
/// at Serde decode time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PermissionString(String);

impl PermissionString {
    /// Creates a `PermissionString`, returning [`WorkspaceStringError`] if `s`
    /// exceeds [`MAX_WORKSPACE_STRING_BYTES`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceStringError`] if the byte length of `s` exceeds
    /// [`MAX_WORKSPACE_STRING_BYTES`].
    #[must_use = "the validated PermissionString must be used"]
    pub fn new(s: impl Into<String>) -> Result<Self, WorkspaceStringError> {
        let s = s.into();
        validate_workspace_string(&s)?;
        Ok(Self(s))
    }

    /// Returns the permission token as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PermissionString {
    type Error = WorkspaceStringError;
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

/// Access control policy for the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkspaceAccessPolicy {
    /// Anyone can enter.
    Open,
    /// Only clients holding the specified [`PermissionString`] token can enter.
    ///
    /// The token is replicated verbatim in the wire format and is guaranteed
    /// not to exceed [`MAX_WORKSPACE_STRING_BYTES`] bytes.
    Permission(PermissionString),
    /// Only explicitly invited clients can enter.
    InviteOnly,
    /// Locked — no one can enter.
    Locked,
}

/// Defines a spatial region as a Workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDefinition {
    /// Human-readable workspace identifier.
    ///
    /// Replicated verbatim in the wire format. Guaranteed not to exceed
    /// [`MAX_WORKSPACE_STRING_BYTES`] bytes (UTF-8) by the [`WorkspaceName`] type.
    pub name: WorkspaceName,
    pub capacity: u32,
    pub access: WorkspaceAccessPolicy,
    pub is_template: bool,
}

/// Spatial bounds of the workspace in world coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkspaceBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// Defines which Workspace an entity currently belongs to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WorkspaceMembership(pub NetworkId);

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
        Self::new(MIN_DYNAMIC_NETWORK_ID)
    }
}

impl NetworkIdAllocator {
    /// Creates a new allocator starting from a specific ID. Must be at least `MIN_DYNAMIC_NETWORK_ID`.
    #[must_use]
    pub fn new(start_id: u64) -> Self {
        let start_id = start_id.max(MIN_DYNAMIC_NETWORK_ID);
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
    fn test_agent_properties_non_zero_default() {
        let properties = AgentProperties::default();
        assert!(properties.max_integrity > 0);
        assert!(properties.max_priority > 0);
        assert!(properties.max_energy > 0);
        assert_eq!(properties.integrity, properties.max_integrity);
    }

    #[test]
    fn test_get_default_properties() {
        assert_eq!(get_default_properties(ENTITY_TYPE_AGENT), (200, 100));
        assert_eq!(get_default_properties(ENTITY_TYPE_AI_AGENT), (200, 100));
        assert_eq!(get_default_properties(ENTITY_TYPE_HEAVY_AGENT), (1500, 500));
        assert_eq!(
            get_default_properties(ENTITY_TYPE_CARRIER_AGENT),
            (600, 200)
        );
        assert_eq!(get_default_properties(ENTITY_TYPE_RESOURCE), (500, 0));
        assert_eq!(get_default_properties(ENTITY_TYPE_DATA_DROP), (1, 0));
        assert_eq!(
            get_default_properties(ENTITY_TYPE_TRAINING_TARGET),
            (100, 50)
        );
        assert_eq!(get_default_properties(ENTITY_TYPE_BEAM), (1, 0));
        assert_eq!(get_default_properties(999), (100, 100)); // Default fallback
    }

    #[test]
    fn test_network_id_allocator_boundary() {
        // Default allocator starts at MIN_DYNAMIC_NETWORK_ID
        let allocator = NetworkIdAllocator::default();
        let id1 = allocator.allocate().unwrap();
        assert_eq!(id1.0, MIN_DYNAMIC_NETWORK_ID);

        // Allocator created with start_id < MIN_DYNAMIC_NETWORK_ID is clamped to MIN_DYNAMIC_NETWORK_ID
        let allocator_custom = NetworkIdAllocator::new(1);
        let id_custom = allocator_custom.allocate().unwrap();
        assert_eq!(id_custom.0, MIN_DYNAMIC_NETWORK_ID);

        // Reset uses the clamped start_id
        allocator_custom.allocate().unwrap();
        allocator_custom.reset();
        let id_reset = allocator_custom.allocate().unwrap();
        assert_eq!(id_reset.0, MIN_DYNAMIC_NETWORK_ID);
    }
}
