---
Version: 0.1.0-draft
Status: Phase 1 — Stable / Phase 2 — Specified
Phase: All
Last Updated: 2026-04-15
Authors: Team (Antigravity)
Spec References: [PF-1000]
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
    /// Maps a protocol-level NetworkId to the ECS's local entity handle.
    fn get_local_id(&self, network_id: NetworkId) -> Option<LocalId>;

    /// Maps a local ECS entity handle back to its protocol-level NetworkId.
    fn get_network_id(&self, local_id: LocalId) -> Option<NetworkId>;

    /// Extracts replication deltas for all components modified since the last tick.
    /// Mutation is required to advance the internal change-detection cursor.
    fn extract_deltas(&mut self) -> Vec<ReplicationEvent>;

    /// Injects parsed state updates from the network into the ECS.
    fn apply_updates(&mut self, updates: Vec<ComponentUpdate>);

    /// Runs a single simulation frame for the ECS (Stage 3).
    fn simulate(&mut self);

    /// Spawn a new networked entity. The ECS allocates and returns the NetworkId.
    fn spawn_networked(&mut self) -> NetworkId;

    /// Despawns a network-replicated entity by its NetworkId.
    fn despawn_networked(&mut self, network_id: NetworkId) -> Result<(), WorldError>;
}
```

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
    /// Serializes a replication event into the provided buffer (allocation-free).
    fn encode(&self, event: &ReplicationEvent, buffer: &mut [u8]) -> Result<usize, EncodeError>;

    /// Deserializes raw bytes into a component update.
    fn decode(&self, buffer: &[u8]) -> Result<ComponentUpdate, EncodeError>;

    /// Encodes a high-level NetworkEvent (Ping/Pong, heartbeats).
    fn encode_event(&self, event: &NetworkEvent, buffer: &mut [u8]) -> Result<usize, EncodeError>;

    /// Decodes a high-level NetworkEvent from a byte slice.
    fn decode_event(&self, data: &[u8]) -> Result<NetworkEvent, EncodeError>;

    /// Returns the maximum possible encoded size for a single event.
    fn max_encoded_size(&self) -> usize;
}
```

## Core Protocol Types

### `NetworkEvent`

```rust
pub enum NetworkEvent {
    /// A new client has connected.
    ClientConnected(ClientId),
    /// A client has disconnected.
    ClientDisconnected(ClientId),
    /// Raw unreliable data received.
    UnreliableMessage { client_id: ClientId, data: Vec<u8> },
    /// Raw reliable data received.
    ReliableMessage { client_id: ClientId, data: Vec<u8> },
    /// Heartbeat ping from a client.
    Ping { client_id: ClientId, tick: u64 },
    /// Heartbeat pong response (Latency RTT).
    Pong { client_id: ClientId, tick: u64 },
}
```

### `ReplicationEvent` & `ComponentUpdate`

```rust
pub struct ReplicationEvent {
    pub network_id: NetworkId,
    pub component_kind: ComponentKind,
    pub payload: Vec<u8>,
    pub tick: u64,
}

pub struct ComponentUpdate {
    pub network_id: NetworkId,
    pub component_kind: ComponentKind,
    pub payload: Vec<u8>,
    pub tick: u64,
}
```

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
