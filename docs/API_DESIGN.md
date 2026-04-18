---
Version: 0.2.0-draft
Status: Phase 1 — MVP / Phase 3 — Specified
Phase: P1 | P3
Last Updated: 2026-04-15
Authors: Team (Antigravity)
Spec References: [LC-0100, LC-0200, LC-0800, PRIORITY_CHANNELS_DESIGN]
Tier: 1
---

# Aetheris API — Technical Design Document

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [API Surface Overview](#2-api-surface-overview)
3. [The Trait Facade — Core API](#3-the-trait-facade--core-api)
4. [Type System](#4-type-system)
5. [Event System](#5-event-system)
6. [ComponentKind Registry](#6-componentkind-registry)
7. [gRPC Control Plane API](#7-grpc-control-plane-api)
8. [SDK Builder API (P3)](#8-sdk-builder-api-p3)
9. [Client API — WASM Bindings](#9-client-api--wasm-bindings)
10. [Error API](#10-error-api)
11. [Rate Limiting (P2)](#11-rate-limiting-p2)
12. [API Stability & Versioning](#12-api-stability--versioning)
13. [Performance Contracts](#13-performance-contracts)
14. [Open Questions](#14-open-questions)
15. [Appendix A — Glossary](#appendix-a--glossary)
16. [Appendix B — Decision Log](#appendix-b--decision-log)

---

## Executive Summary

Aetheris exposes three distinct API surfaces:

| Surface | Protocol | Audience | Stability |
|---|---|---|---|
| **Trait Facade** | Rust traits (`aetheris-protocol`) | Engine implementers | Stable (P3+) |
| **Control Plane** | gRPC/Protobuf (`auth.proto`) | Clients, operators | Stable (P2+) |
| **SDK Builder** | Rust builder pattern (`aetheris-sdk`) | Game developers | Stable (P3+) |

The Trait Facade is the primary API — it defines the contract between the engine core and all pluggable implementations. All other APIs are derived from or built on top of the Trait Facade.

**Design philosophy:** The API is narrow by design. Fewer methods mean fewer breaking changes, easier testing via mocks, and a shallower learning curve. Each trait method exists because the tick pipeline requires it.

---

## 2. API Surface Overview

```text
┌─────────────────────────────────────────────────────────┐
│                   Game Developer                        │
│                                                         │
│  AetherisServer::builder()   ←── SDK Builder API (P3)   │
│       .transport(MyTransport)                           │
│       .world(MyWorld)                                   │
│       .encoder(MyEncoder)                               │
│       .build()                                          │
│       .run()                                            │
└────────────────────┬────────────────────────────────────┘
                     │ uses
┌────────────────────▼────────────────────────────────────┐
│              Trait Facade (aetheris-protocol)            │
│                                                         │
│  GameTransport    WorldState    Encoder                  │
│  ─────────────    ──────────    ───────                  │
│  poll_events()    simulate()    encode()                 │
│  send_*()         extract_*()   decode()                 │
│  broadcast_*()    spawn/despawn  max_encoded_size()      │
└────────────────────┬────────────────────────────────────┘
                     │ implemented by
┌────────────────────▼────────────────────────────────────┐
│  Phase 1: Renet + WebTransport │ Bevy ECS │ rmp-serde   │
│  Phase 3: Quinn QUIC           │ Custom   │ Bitpack     │
└─────────────────────────────────────────────────────────┘
```

---

## 3. The Trait Facade — Core API

Defined in `aetheris-protocol/src/traits.rs`. All traits use `async_trait` (conditionally `?Send` on WASM).

### 3.1 `GameTransport`

```rust
#[async_trait]
pub trait GameTransport: Send + Sync {
    async fn send_unreliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError>;
    async fn send_reliable(&self, client_id: ClientId, data: &[u8]) -> Result<(), TransportError>;
    async fn broadcast_unreliable(&self, data: &[u8]) -> Result<(), TransportError>;
    async fn poll_events(&mut self) -> Vec<NetworkEvent>;
    async fn connected_client_count(&self) -> usize;
}
```

| Method | Tick Stage | Channel | Failure Mode |
|---|---|---|---|
| `send_unreliable` | 5 (Send) | QUIC datagram / UDP | `ClientNotConnected`, `PayloadTooLarge` |
| `send_reliable` | 5 (Send) | QUIC stream / TCP-like | `ClientNotConnected`, `Io` |
| `broadcast_unreliable` | 5 (Send) | All connected clients | Best-effort per client |
| `poll_events` | 1 (Poll) | — | Returns empty vec on error |
| `connected_client_count` | Any | — | Infallible |

### 3.2 `WorldState`

```rust
pub trait WorldState: Send {
    fn get_local_id(&self, network_id: NetworkId) -> Option<LocalId>;
    fn get_network_id(&self, local_id: LocalId) -> Option<NetworkId>;
    fn extract_deltas(&mut self) -> Vec<ReplicationEvent>;
    fn apply_updates(&mut self, updates: &[ComponentUpdate]) -> Vec<WorldError>;
    fn simulate(&mut self);
    fn spawn_networked(&mut self) -> NetworkId;
    fn despawn_networked(&mut self, network_id: NetworkId) -> Result<(), WorldError>;
}
```

| Method | Tick Stage | Complexity | Notes |
|---|---|---|---|
| `get_local_id` | 2 (Apply) | O(1) bimap lookup | Returns `None` for unknown IDs |
| `get_network_id` | 4 (Extract) | O(1) bimap lookup | Reverse mapping |
| `extract_deltas` | 4 (Extract) | O(changed_entities) | Returns only changed components |
| `apply_updates` | 2 (Apply) | O(updates) | Returns errors for each failed update |
| `simulate` | 3 (Simulate) | O(entities) | Runs all ECS systems |
| `spawn_networked` | 2 (Apply) | O(1) | Allocates monotonic `NetworkId` |
| `despawn_networked` | 2 (Apply) | O(1) | Removes from ECS + bimap |

### 3.3 `Encoder`

```rust
pub trait Encoder: Send + Sync {
    fn encode(&self, event: &ReplicationEvent, buffer: &mut [u8]) -> Result<usize, EncodeError>;
    fn decode(&self, buffer: &[u8]) -> Result<ComponentUpdate, EncodeError>;
    fn encode_event(&self, event: &NetworkEvent) -> Result<Vec<u8>, EncodeError>;
    fn decode_event(&self, data: &[u8]) -> Result<NetworkEvent, EncodeError>;
    fn max_encoded_size(&self) -> usize;
}
```

| Method | Allocates? | Purpose |
|---|---|---|
| `encode` | No (caller buffer) | Hot-path encoding into pre-allocated buffer |
| `decode` | Minimal | Hot-path decoding from network bytes |
| `encode_event` | Yes (`Vec<u8>`) | Convenience for non-hot-path encoding |
| `decode_event` | Yes | Convenience for non-hot-path decoding |
| `max_encoded_size` | No | Buffer pre-allocation hint |

### 3.4 `NetworkIdAllocator`

```rust
pub struct NetworkIdAllocator { counter: AtomicU64 }

impl NetworkIdAllocator {
    pub fn new() -> Self;          // Starts at 1 (0 is null sentinel)
    pub fn allocate(&self) -> NetworkId;  // fetch_add(1, Relaxed)
}
```

Lock-free, monotonically increasing. `NetworkId(0)` is reserved as a null sentinel and is never allocated.

---

## 4. Type System

Defined in `aetheris-protocol/src/types.rs`. All types are newtypes for type safety:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkId(pub u64);      // Global entity identity

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u64);        // ECS-internal handle (not serialized)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);       // Transport session identity

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentKind(pub u16);  // Component type discriminant
```

**Design rule:** `NetworkId`, `ClientId`, and `ComponentKind` cross the network boundary (serializable). `LocalId` is ECS-internal and never transmitted.

---

## 5. Event System

Defined in `aetheris-protocol/src/events.rs`:

### 5.1 `ReplicationEvent`

```rust
pub enum ReplicationEvent {
    Spawn { network_id: NetworkId },
    Despawn { network_id: NetworkId },
    Update(ComponentUpdate),
}
```

Generated by `WorldState::extract_deltas()` during Stage 4 (Extract). Consumed by the encoder in Stage 5.

### 5.2 `ComponentUpdate`

```rust
pub struct ComponentUpdate {
    pub network_id: NetworkId,
    pub component_kind: ComponentKind,
    pub data: Vec<u8>,
}
```

### 5.3 `NetworkEvent`

```rust
pub enum NetworkEvent {
    ClientConnected(ClientId),
    ClientDisconnected(ClientId),
    UnreliableMessage { client_id: ClientId, data: Vec<u8> },
    ReliableMessage { client_id: ClientId, data: Vec<u8> },
}
```

Generated by `GameTransport::poll_events()` during Stage 1 (Poll). Consumed by the Apply stage.

> **Note:** The `channel: u8` field in `UnreliableMessage` is the **Priority Channel tag** — a 1-byte identifier corresponding to a channel defined in the `ChannelRegistry`. In Phase 3, this field is used by the `IngestPriorityRouter` (Stage 1) to sort inbound messages by priority before processing. See [PRIORITY_CHANNELS_DESIGN.md §8](PRIORITY_CHANNELS_DESIGN.md#8-bidirectional-priority-processing).

---

## 6. ComponentKind Registry

### 6.1 Namespace Allocation

`ComponentKind` is a `u16` with a reserved namespace structure:

| Range | Owner | Example |
|---|---|---|
| `0x0000–0x00FF` | Aetheris Engine (stdlib) | Position, Velocity, Health |
| `0x0100–0x0FFF` | Official Aetheris Labs extensions | Inventory, Chat, Physics |
| `0x1000–0xFFFF` | Community / third-party | Game-specific components |

### 6.2 Registration

The encoder maintains a decode table mapping `ComponentKind → decode_fn`. An unregistered `ComponentKind` in inbound data returns `EncodeError::UnknownComponent`.

### 6.3 Versioning

New `ComponentKind` values can be added without breaking existing clients (additive change). Removing a `ComponentKind` follows a 3-release deprecation window (see [MIGRATION_DESIGN.md](MIGRATION_DESIGN.md)).

---

## 7. gRPC Control Plane API

### 7.1 Service Definition

Currently a single service defined in `aetheris-protocol/proto/auth.proto`:

```protobuf
service AuthService {
    rpc Authenticate(AuthRequest) returns (AuthResponse);
}

message AuthRequest {
    string username = 1;
    string password = 2;
}

message AuthResponse {
    string token = 1;
    string player_id = 2;
}
```

### 7.2 gRPC Endpoint

| Method | Address | Authentication | Rate Limited? |
|---|---|---|---|
| `AuthService/Authenticate` | `0.0.0.0:50051` | None (this IS the auth endpoint) | P2 |

### 7.3 Error Responses

| Scenario | gRPC Status | Client Action |
|---|---|---|
| Invalid credentials | `UNAUTHENTICATED` | Show "invalid login" |
| Malformed request | `INVALID_ARGUMENT` | Show "format error" |
| Server overloaded | `UNAVAILABLE` | Retry with backoff |
| Internal error | `INTERNAL` | Show "server error" |

### 7.4 Planned gRPC Services (P2+)

| Service | RPCs | Purpose |
|---|---|---|
| `MatchmakingService` | `JoinQueue`, `LeaveQueue`, `GetStatus` | Matchmaking lobby |
| `WorldService` | `GetServerInfo`, `ListZones` | Server discovery |
| `AdminService` | `KickPlayer`, `BanPlayer`, `SetConfig` | Operator tools |

---

## 8. SDK Builder API (P3)

### 8.1 Builder Pattern

```rust
// Planned: aetheris-sdk crate
let server = AetherisServer::builder()
    .transport(MyQuinnTransport::new(addr))
    .world(MyCustomEcs::new())
    .encoder(MyBitpackEncoder::new())
    .channels(ChannelRegistry::default_game_channels()) // Priority Channels
    .tick_rate(60)
    .max_clients(10_000)
    .build()?;

server.run(shutdown_token).await;
```

The `.channels()` method accepts a `ChannelRegistry` — the developer-configurable priority channel layout. Games may use the default 6-channel configuration or define custom channels via the builder API. See [PRIORITY_CHANNELS_DESIGN.md §3](PRIORITY_CHANNELS_DESIGN.md#3-channel-registry--developer-configurable-channels).

### 8.2 Type-Safe Configuration

The builder uses Rust's type system to enforce compile-time completeness:

```rust
struct AetherisServerBuilder<T, W, E> {
    transport: Option<T>,
    world: Option<W>,
    encoder: Option<E>,
    tick_rate: u64,
    max_clients: usize,
}

impl<T: GameTransport, W: WorldState, E: Encoder>
    AetherisServerBuilder<T, W, E>
{
    pub fn build(self) -> Result<AetherisServer<T, W, E>, BuildError>;
}
```

### 8.3 Monomorphized Dispatch

The final `AetherisServer<T, W, E>` is fully monomorphized — no `dyn Trait` on the hot path. The only dynamic dispatch is the `ComponentRegistry` decode table lookup.

---

## 9. Client API — WASM Bindings

### 9.1 `wasm-bindgen` Exports

The WASM client (`aetheris-client-wasm`) exposes JavaScript bindings:

```typescript
// Generated TypeScript declarations (pkg/aetheris_client_wasm.d.ts)
export function init_game(cert_hash: string): void;
export function tick(): Float32Array;
export function send_input(input: Uint8Array): void;
```

### 9.2 Worker Architecture

| Worker | Purpose | API |
|---|---|---|
| Game Worker | Runs WASM, manages simulation | `postMessage` to/from Main Thread |
| Render Worker | GPU rendering via `OffscreenCanvas` | `postMessage` with vertex buffers |

### 9.3 Message Protocol (Main ↔ Game Worker)

```typescript
// Main → Game Worker
{ type: 'input', data: Uint8Array }
{ type: 'connect', certHash: string }

// Game Worker → Main
{ type: 'state_update', entities: Float32Array }
{ type: 'connection_error', reason: string }
```

---

## 10. Error API

Each trait returns its own error type (see [ERROR_HANDLING_DESIGN.md](ERROR_HANDLING_DESIGN.md)):

| Trait | Error Type | Variants |
|---|---|---|
| `GameTransport` | `TransportError` | `ClientNotConnected`, `PayloadTooLarge`, `Io` |
| `Encoder` | `EncodeError` | `BufferOverflow`, `MalformedPayload`, `UnknownComponent`, `Io` |
| `WorldState` | `WorldError` | `EntityNotFound`, `EntityAlreadyExists` |

All error types derive `thiserror::Error` for automatic `Display` and `From` implementations.

---

## 11. Rate Limiting (P2)

### 11.1 gRPC Rate Limiting

| Endpoint | Limit | Window | Enforcement |
|---|---|---|---|
| `Authenticate` | 10 requests | per minute per IP | `RESOURCE_EXHAUSTED` status |
| `JoinQueue` | 5 requests | per minute per token | `RESOURCE_EXHAUSTED` status |
| Admin RPCs | 100 requests | per minute per token | `RESOURCE_EXHAUSTED` status |

### 11.2 Data Plane Rate Limiting

Client input packets are rate-limited by the tick loop itself — the server processes at most one input per client per tick. Excess packets are dropped. Clients sending > 120 packets/second are flagged for behavioral analysis (see [SECURITY_DESIGN.md](SECURITY_DESIGN.md)).

---

## 12. API Stability & Versioning

### 12.1 Stability Tiers

| Crate | Stability | Semver |
|---|---|---|
| `aetheris-sdk` (P3+) | **Stable** | Breaking changes = major version bump |
| `aetheris-protocol` | **Stable** (traits) | Trait methods frozen once published |
| `aetheris-transport-*` | Unstable | Internal API, may change between minor versions |
| `aetheris-ecs-*` | Unstable | Internal API |
| `aetheris-encoder-*` | Unstable | Internal API |

### 12.2 Breaking Change Policy

- Trait Facade methods: **Never removed**, only deprecated.
- Trait Facade new methods: Added with default implementations to avoid breaking downstream.
- Types (`NetworkId`, `ClientId`, etc.): **Frozen** once stable.
- gRPC Protobuf: Field numbers never reused. Removed fields marked `reserved`.

### 12.3 Binary Size Targets

| Artifact | Target | Current |
|---|---|---|
| Server binary (release, stripped) | ≤ 50 MB | — |
| Native client (release, stripped) | ≤ 30 MB | — |
| WASM client (gzip) | ≤ 1.2 MB | — |

---

## 13. Performance Contracts

| Operation | Budget | Notes |
|---|---|---|
| `poll_events()` | < 100 μs | Drain internal queue, no allocation |
| `encode()` | < 10 μs per entity | Zero-allocation into caller buffer |
| `decode()` | < 10 μs per packet | Parse from byte slice |
| `simulate()` | < 8 ms | Phase 1 Bevy systems, scales with entity count |
| `extract_deltas()` | < 2 ms | Change-detection pass |
| `broadcast_unreliable()` | < 1 ms | Fan-out to all clients |
| gRPC `Authenticate` | < 50 ms | Hash verification + token generation |

---

## 14. Open Questions

| Question | Context | Impact |
|---|---|---|
| **Rate Limiting** | How will we handle application-level rate limiting for the public API? | Stability and security. Addressed in §11. |
| **REST API** | Should there be a REST/HTTP API alongside gRPC for simpler integrations? | Developer adoption for non-Rust clients. |
| **WebSocket Fallback** | Should the client support WebSocket when WebTransport is unavailable? | Browser compatibility. |
| **Component Registration API** | How do third-party developers register custom `ComponentKind` values? | Extensibility and ecosystem growth. |
| **API Documentation** | Should we generate OpenAPI/gRPC docs automatically from proto files? | Developer experience. |

---

## Appendix A — Glossary

### Mini-Glossary (Quick Reference)

- **Public API**: The external interface exposed to third-party developers (Trait Facade + SDK Builder).
- **Trait Facade**: The three-trait abstraction (`GameTransport`, `WorldState`, `Encoder`) that defines the engine contract.
- **ComponentKind**: A `u16` discriminant that identifies a component type for encoding/decoding.
- **NetworkEvent**: An inbound event from the transport (connect, disconnect, message).
- **ReplicationEvent**: An outbound event from the ECS (spawn, despawn, update).

[Full Glossary Document](../GLOSSARY.md)

---

## Appendix B — Decision Log

| # | Decision | Rationale | Revisit If... | Date |
|---|---|---|---|---|
| D1 | Three narrow traits, not one god-trait | Separation of concerns, independent testing, independent swapping. | Cross-cutting concerns require shared state between traits. | 2026-04-15 |
| D2 | Newtype wrappers for all IDs | Type safety prevents `ClientId`/`NetworkId` confusion at compile time. | Performance profiling shows newtype overhead (unlikely — zero-cost). | 2026-04-15 |
| D3 | `encode()` uses caller-supplied buffer | Zero-allocation on the hot path. `encode_event()` allocates for convenience paths. | Buffer management becomes a footgun for implementers. | 2026-04-15 |
| D4 | gRPC for Control Plane, not REST | Strong typing, code generation, bi-directional streaming, native Rust support via tonic. | Web dashboard needs REST; add a REST gateway or `grpc-web`. | 2026-04-15 |
| D5 | Monomorphized SDK (`AetherisServer<T,W,E>`) | Zero overhead. The compiler inlines trait method calls. | Dynamic plugin loading requires `dyn Trait` dispatch. | 2026-04-15 |
| D6 | `ChannelRegistry` as SDK builder parameter | Priority Channels are game-specific; the builder API must accept a configurable channel layout. See [PRIORITY_CHANNELS_DESIGN.md §3](PRIORITY_CHANNELS_DESIGN.md#3-channel-registry--developer-configurable-channels). | If all games converge on the same channel topology and configurability adds no value. | 2026-04-15 |
