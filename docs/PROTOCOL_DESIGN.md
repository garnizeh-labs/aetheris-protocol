---
Version: 0.1.5 (Protocol v3)
Status: Phase 1 — Stable / Phase 2 — Specified
Phase: All
Last Updated: 2026-04-22
Authors: Team (Antigravity)
Spec References: [PF-1000, M1020]
Tier: 1
---

# Aetheris Protocol — Technical Design Document

## Executive Summary

This document defines the core contracts and communication protocol for the Aetheris engine. It serves as the single source of truth for the engine's traits and wire format, ensuring consistency across all subsystems and documentation.

## Core Traits — The Trait Facade

Aetheris is built around three core traits that abstract the engine's dependencies.

### 1. `WorldState` — ECS Adapter

The `WorldState` trait provides a facade over the underlying Entity Component System.

```rust
pub trait WorldState: Send {
    // --- Identity mapping ---

    /// Maps a protocol-level NetworkId to the ECS's local entity handle.
    fn get_local_id(&self, network_id: NetworkId) -> Option<LocalId>;

    /// Maps a local ECS entity handle back to its protocol-level NetworkId.
    fn get_network_id(&self, local_id: LocalId) -> Option<NetworkId>;

    // --- Tick pipeline ---

    /// Called once per tick before inputs are applied (advances change-detection).
    fn advance_tick(&mut self) {}

    /// Injects parsed state updates from the network into the ECS.
    fn apply_updates(&mut self, updates: &[(ClientId, ComponentUpdate)]);

    /// Runs a single simulation frame for the ECS (Stage 3).
    fn simulate(&mut self) {}

    /// Extracts replication deltas for all components modified since the last tick.
    fn extract_deltas(&mut self) -> Vec<ReplicationEvent>;

    /// Extracts discrete game events that should be sent reliably.
    /// Returns `(target ClientId, WireEvent)` pairs; `None` target = broadcast.
    fn extract_reliable_events(&mut self) -> Vec<(Option<ClientId>, WireEvent)> { vec![] }

    /// Called once per tick after extraction to clear ECS change-detection trackers.
    fn post_extract(&mut self) {}

    // --- Spawn / despawn ---

    /// Spawns a new network-replicated entity.
    fn spawn_networked(&mut self) -> NetworkId;

    /// Despawns a network-replicated entity by its NetworkId.
    fn despawn_networked(&mut self, network_id: NetworkId) -> Result<(), WorldError>;

    /// [DEFAULT] Spawns an entity of a specific kind (type discriminant) at the
    /// given position.  A default implementation exists that delegates to
    /// `spawn_networked`, ignoring `kind`, `x`, `y`, and `rot`.  Implementors
    /// may override `spawn_kind` when the kind or position must be stored on the
    /// entity (e.g. to set a `Transform` component or a type discriminant).
    fn spawn_kind(&mut self, kind: u16, x: f32, y: f32, rot: f32) -> NetworkId;

    /// [DEFAULT] Spawns an entity of a specific kind owned by `client_id`.
    /// Attaches `Ownership(client_id)` so the input pipeline can gate
    /// `InputCommand` processing to the correct owner.
    ///
    /// The default implementation delegates to `spawn_kind(kind, x, y, rot)`,
    /// ignoring `client_id`.  Implementors only need to override this when they
    /// must attach ownership metadata (e.g. the `Ownership(ClientId)` Bevy
    /// component) to the spawned entity.
    fn spawn_kind_for(
        &mut self,
        kind: u16, x: f32, y: f32, rot: f32,
        client_id: ClientId,
    ) -> NetworkId { self.spawn_kind(kind, x, y, rot) }

    /// [DEFAULT] Spawns the authoritative session ship for a client.
    ///
    /// The default implementation delegates to `spawn_kind_for(kind, x, y, rot,
    /// client_id)`.  Implementors only need to override this when they must also
    /// attach a `SessionShip` marker component and register the client in the
    /// room index so that `get_client_room` returns the correct room without an
    /// O(n) fallback scan.
    fn spawn_session_ship(
        &mut self,
        kind: u16, x: f32, y: f32, rot: f32,
        client_id: ClientId,
    ) -> NetworkId { self.spawn_kind_for(kind, x, y, rot, client_id) }

    /// Removes/despawns all entities from the world.  Does **not** rebuild any
    /// initial state — callers must invoke `setup_world` afterwards if they need
    /// the room topology restored.
    fn clear_world(&mut self) {}

    // --- Room management ---

    /// Initialises (or re-initialises) the initial room topology, e.g. spawning
    /// the Master Room.  Called once on server startup and again after
    /// `clear_world` to restore the baseline world state.  `clear_world` and
    /// `setup_world` are intentionally separate so callers control whether a
    /// wipe is followed by a bootstrap.
    fn setup_world(&mut self) {}

    /// Returns the Room `NetworkId` that an entity belongs to.
    fn get_entity_room(&self, network_id: NetworkId) -> Option<NetworkId> { None }

    /// Returns the Room `NetworkId` of a client's session ship.
    /// Used by the tick scheduler to scope delta delivery to in-room clients.
    fn get_client_room(&self, client_id: ClientId) -> Option<NetworkId> { None }

    // --- Reliable events ---

    /// Queues a `GameEvent` to be sent reliably to `client_id` (or all if `None`).
    fn queue_reliable_event(
        &mut self,
        client_id: Option<ClientId>,
        event: GameEvent,
    ) {}
}
```

#### New methods in v3 (VS-05 / VS-06)

| Method | Required? | Purpose |
|---|---|---|
| `spawn_kind_for` | Default | Delegates to `spawn_kind`; override to attach `Ownership(ClientId)` |
| `spawn_session_ship` | Default | Delegates to `spawn_kind_for`; override to attach `SessionShip` marker + `RoomIndex` registration |
| `queue_reliable_event` | Default | Enqueue typed `GameEvent` without direct transport access |
| `setup_world` | Default | Idempotent room bootstrap (called at startup + after `clear_world`) |
| `get_entity_room` | Default | Entity → Room lookup for AoI delta scoping |
| `get_client_room` | Default | Client → Room lookup for per-tick target resolution |
| `post_extract` | Default | Resets ECS change-detection after delta extraction |

### 2. `GameTransport` — Network Abstraction

The `GameTransport` trait abstracts the underlying networking library (Renet in P1, Quinn in P3).

```rust
#[async_trait]
pub trait GameTransport: Send + Sync {
    /// Sends an unreliable datagram (volatile data like position).
    async fn send_unreliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError>;

    /// Sends a reliable, ordered message (critical events).
    async fn send_reliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError>;

    /// Broadcasts an unreliable datagram to all connected clients.
    async fn broadcast_unreliable(&self, data: &[u8]) -> Result<(), TransportError>;

    /// Drains all pending inbound network events since the last call.
    async fn poll_events(&mut self) -> Vec<NetworkEvent>;

    /// Returns the number of currently connected clients.
    async fn connected_client_count(&self) -> usize;

    /// [Planned P3] Broadcasts a reliable message to all clients.
    // async fn broadcast_reliable(&self, data: &[u8]) -> Result<(), TransportError>;
}
```

### 3. `Encoder` — Serialization Protocol

The `Encoder` trait defines how replication events are serialized for the wire.

```rust
pub trait Encoder: Send + Sync {
    /// codec ID (1=Serde, 2=Bitpack).
    fn codec_id(&self) -> u32;

    /// Serializes a replication event into the provided buffer.
    fn encode(&self, event: &ReplicationEvent, buffer: &mut [u8]) -> Result<usize, EncodeError>;

    /// Deserializes raw bytes into a component update.
    fn decode(&self, buffer: &[u8]) -> Result<ComponentUpdate, EncodeError>;

    /// Encodes a high-level NetworkEvent.
    fn encode_event(&self, event: &NetworkEvent) -> Result<Vec<u8>, EncodeError>;

    /// Decodes a high-level NetworkEvent from a byte slice.
    fn decode_event(&self, data: &[u8]) -> Result<NetworkEvent, EncodeError>;
}
```

## Core Protocol Types

### `NetworkEvent`

```rust
pub enum NetworkEvent {
    // Transport layer
    ClientConnected(ClientId),
    ClientDisconnected(ClientId),
    Disconnected(ClientId),         // local transport drop
    SessionClosed(ClientId),        // WebTransport session closed
    StreamReset(ClientId),          // WebTransport stream reset
    UnreliableMessage { client_id: ClientId, data: Vec<u8> },
    ReliableMessage   { client_id: ClientId, data: Vec<u8> },
    Fragment          { client_id: ClientId, fragment: FragmentedEvent },

    // Heartbeat
    Ping { client_id: ClientId, tick: u64 },
    Pong { tick: u64 },

    // Auth / session
    Auth { session_token: String },

    // Playground / stress test
    StressTest { client_id: ClientId, count: u16, rotate: bool },
    Spawn      { client_id: ClientId, entity_type: u16, x: f32, y: f32, rot: f32 },
    ClearWorld { client_id: ClientId },

    // [v3 — VS-05] Session lifecycle
    /// Requests the server to spawn the client's session ship and issue Possession.
    StartSession { client_id: ClientId },
    /// Requests the current system manifest (versions, tick rate, counters).
    RequestSystemManifest { client_id: ClientId },

    // Discrete game events (reliable)
    GameEvent { client_id: ClientId, event: GameEvent },

    // [v3 — VS-06] Replication
    /// A batch of replication updates sent together to save bandwidth/packets.
    ReplicationBatch { client_id: ClientId, events: Vec<ReplicationEvent> },
}
```

### `GameEvent` (reliable discrete events)

```rust
pub enum GameEvent {
    /// An asteroid entity has been fully depleted.
    AsteroidDepleted { network_id: NetworkId },

    // [v3 — VS-05] Session lifecycle events
    /// Server informs the client that it now owns/controls `network_id`.
    /// Sent once per `StartSession`, after the session ship is spawned.
    Possession { network_id: NetworkId },

    /// Server sends extensible metadata to the client.
    /// Keys: `"version_server"`, `"version_protocol"`, `"tick_rate"`,
    ///        `"clients_active"` (admin-gated).
    /// Values are always UTF-8 strings; callers must parse to the target type.
    SystemManifest { manifest: BTreeMap<String, String> },
}
```

### `WireEvent` (over-the-wire subset of `NetworkEvent`)

`WireEvent` is the serializable enum — it excludes local-only variants
(`ClientConnected`, `UnreliableMessage`, etc.) that never cross the wire.

```rust
pub enum WireEvent {
    Ping { tick: u64 },
    Pong { tick: u64 },
    Auth { session_token: String },
    StressTest { count: u16, rotate: bool },
    Spawn      { entity_type: u16, x: f32, y: f32, rot: f32 },
    ClearWorld,
    // [v3 — VS-05]
    StartSession,
    RequestSystemManifest,
    GameEvent(GameEvent),
    Fragment(FragmentedEvent),
    ReplicationBatch(Vec<ReplicationEvent>),
}
```

Conversion: `WireEvent::into_network_event(client_id: ClientId) -> NetworkEvent`
injects the `ClientId` — it is **not** transmitted on the wire.

#### Wire format (Phase 1 — MessagePack)

| Variant | Approx. encoded size | Notes |
|---|---|---|
| `StartSession` | ~4 bytes | Tag only (unit variant) |
| `RequestSystemManifest` | ~4 bytes | Tag only (unit variant) |
| `Possession { network_id }` | ~12 bytes | Tag + u64 |
| `SystemManifest { manifest }` | variable | Tag + MsgPack map; 4–6 keys ≈ 80–120 bytes |
| `ReplicationBatch { events }` | variable | Tag + Array of updates; capped by MTU (1200) |

### `ReplicationEvent` & `ComponentUpdate`

```rust
pub struct ReplicationEvent {
    pub network_id: NetworkId,
    pub component_kind: ComponentKind,
    pub payload: Vec<u8>,
    pub tick: u64,
}

#### Example: ReplicationBatch (JSON-equivalent of MsgPack)

```json
{
  "variant": "ReplicationBatch",
  "events": [
    {
      "network_id": 42,
      "component_kind": 1,
      "payload": "base64_blob...",
      "tick": 1000
    },
    {
      "network_id": 43,
      "component_kind": 1,
      "payload": "base64_blob...",
      "tick": 1000
    }
  ]
}
```

pub struct ComponentUpdate {
    pub network_id: NetworkId,
    pub component_kind: ComponentKind,
    pub payload: Vec<u8>,
    pub tick: u64,
}
```

### `ShipClass`

Used for rendering and stat selection (Interceptor, Dreadnought, Hauler).

### `WeaponId`

A `u8` unique identifier for a static weapon type definition.

### `SectorId`

A `u64` globally unique identifier for a persistent sector or room instance.

### `OreType`

Defines material types that can be extracted from asteroids (e.g., `RawOre`).

### `ProjectileType`

Classification for projectile delivery and VFX (e.g., `PulseLaser`, `SeekerMissile`).

### `AIState`

NPC behavior machine state (Patrol, Aggro, Combat, Return).

### `RespawnLocation`

Specifies where an entity should appear after death (Nearest Safe Zone, Station, or Coordinates).

### `InputCommand`

Aggregated user input for a single tick, including movement axes and discrete actions.

- **Kind ID**: 128 (Transient/Inbound-Only).
- **Hardening [v3]**: `MAX_ACTIONS = 128` is enforced. Payloads exceeding this limit are rejected by the server to prevent DoS via vector growth.
- **Wire Format**: Contains `tick` and a `Vec<PlayerInputKind>`.

## Component Kind Reservation Policy

To prevent ID collisions across official extensions and community plugins, the `ComponentKind` (`u16`) space is partitioned as follows:

| Range | Owner | Description |
|---|---|---|
| **0–1023 (except 128)** | **Engine Core** | Fundamental engine components (Transform, NetworkId, etc.). |
| **128** | **InputCommand** | Explicitly reserved for client-to-server transient commands. |
| **1024–2047** | **Official Extensions** | Managed extensions like standard weapons, physics bodies. |
| **2048–32767** | **Application Space** | Game-specific logic (e.g., Void Rush specific modules). |
| **32768–65535** | **Reserved** | Reserved for non-replicated or inbound-only variants. |

### `ShipStats`

Authoritative vitals for ship entities, including HP, Shields, Energy, and regeneration rates.

### `SuspicionScore` (Security)

The `SuspicionScore` is a `u32` value assigned to every entity to track potential cheating or anomalies.

- **Type**: `u32` (capped at `u32::MAX`)
- **Persistence**: Ephemeral (in-memory only)
- **Thresholds**:
  - **Baseline**: 0–99
  - **Elevated**: 100–499
  - **Critical**: 500+

## Cryptographic Integrity — Merkle Hash Chain

Aetheris uses a per-entity hash chain to ensure the integrity of the event ledger.

### `MerkleHash` Formula

The hash for the $n$-th event of an entity is computed as:

$$H_n = \text{SHA-256}(H_{n-1} \parallel \text{tick} \parallel \text{network\_id} \parallel \text{component\_kind} \parallel \text{payload})$$

Where:

- $H_{n-1}$ is the hash of the previous event (32 bytes).
- $H_0$ (Genesis) = $\text{SHA-256}(\text{network\_id} \parallel \text{"GENESIS"})$.
- $\parallel$ denotes concatenation.
- `tick`, `network_id`, and `component_kind` are in little-endian format.
- `payload` is the raw byte slice of the component update.

---

## Open Questions

| Question | Context | Impact |
|---|---|---|
| **Zero-Copy Trait** | Should `Encoder` return `Cow<[u8]>` instead of writing to a buffer to support Zero-Copy in P3? | Performance optimization for large payloads. |
| **Protocol Versioning** | How to handle semantic versioning for traits without breaking binary compatibility? | Long-term SDK stability. |
| **Encryption Layer** | Should encryption be part of the `Encoder` or handled by the `GameTransport`? | Security boundary definition. |

---

## Appendix A — Glossary

### Mini-Glossary (Quick Reference)

- **NetworkId**: A globally unique identifier for an entity across the cluster.
- **WorldState**: The primary facade trait for interacting with the ECS simulation.
- **Encoder**: The protocol layer responsible for translating events to bytes.
- **GameTransport**: The network abstraction layer for unreliable and reliable delivery.
- **Merkle Hash Chain**: A cryptographic structure ensuring historical event integrity.

[Full Glossary Document](https://github.com/garnize/aetheris/blob/main/docs/GLOSSARY.md)

---

## Appendix B — Decision Log

| # | Decision | Rationale | Revisit If... | Date |
|---|---|---|---|---|
| D1 | Trait Triad Architecture | Decouples simulation, networking, and serialization for multi-phase evolution. | Implementation complexity exceeds maintenance gains. | 2026-04-15 |
| D2 | SHA-256 for Merkle Chain | Industry-standard, ASIC-resistant hash function for non-realtime audit. | Collision attacks on SHA-256 become practical. | 2026-04-15 |
| D3 | Little-Endian Encoding | Consistent with x86_64/ARM64 defaults and common networking standards. | A strict Big-Endian hardware ecosystem is targeted. | 2026-04-15 |
