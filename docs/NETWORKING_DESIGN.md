---Version: 0.2.0-draft
Status: Phase 1 — MVP / Phase 3 — Specified
Phase: P1 | P3
Last Updated: 2026-04-15
Authors: Team (Antigravity)
Spec References: [PF-2000, PF-2100, PRIORITY_CHANNELS_DESIGN]
Tier: 1
---

# Aetheris Networking — Technical Design Document

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Network Fundamentals — Why UDP, Why QUIC](#2-network-fundamentals--why-udp-why-quic)
3. [Head-of-Line Blocking — The Core Problem](#3-head-of-line-blocking--the-core-problem)
4. [QUIC Multi-Stream Architecture](#4-quic-multi-stream-architecture)
5. [WebTransport — Browser UDP](#5-webtransport--browser-udp)
6. [Dual-Plane Network Topology](#6-dual-plane-network-topology)
7. [Congestion Control & Flow Control](#7-congestion-control--flow-control)
8. [MTU, Fragmentation & Path Discovery](#8-mtu-fragmentation--path-discovery)
9. [NAT Traversal & Connectivity](#9-nat-traversal--connectivity)
10. [Latency Budget & Bandwidth Estimation](#10-latency-budget--bandwidth-estimation)
11. [Connection Resilience & Reconnection](#11-connection-resilience--reconnection)
12. [Performance Contracts](#12-performance-contracts)
13. [Open Questions](#13-open-questions)
14. [Appendix A — Glossary](#appendix-a--glossary)
15. [Appendix B — Decision Log](#appendix-b--decision-log)

---

## Executive Summary

This document covers the **networking fundamentals** that underpin Aetheris's transport layer. While [TRANSPORT_DESIGN.md](TRANSPORT_DESIGN.md) describes the `GameTransport` trait, its concrete implementations, and the channel architecture, this document focuses on the *networking theory and protocol-level decisions* that justify those implementations.

The core insight driving Aetheris's networking design:

> **Real-time game networking is fundamentally incompatible with TCP. The only viable path for browser clients is WebTransport over QUIC, which gives us UDP semantics (unreliable datagrams) alongside selective reliability (ordered streams) — all on a single UDP port with mandatory TLS 1.3.**

### Key Networking Properties

| Property | Value | Justification |
|---|---|---|
| **Data Plane Protocol** | QUIC (RFC 9000) | Multiplexed streams + datagrams, no HOL blocking |
| **Browser Protocol** | WebTransport (W3C) | Only browser API exposing UDP-like semantics |
| **Control Plane Protocol** | gRPC / HTTP/2 | Strong contracts, schema evolution, latency-tolerant |
| **Encryption** | TLS 1.3 (mandatory) | Built into QUIC; not optional |
| **Server UDP Port** | 4433 | Single port for all QUIC + WebTransport connections |
| **Server TCP Port** | 50051 | Control Plane gRPC |
| **Safe Datagram MTU** | 1140 bytes payload | 1200 bytes − 60 bytes QUIC overhead |

---

## 2. Network Fundamentals — Why UDP, Why QUIC

### 2.1 Why Not TCP

TCP provides reliable, ordered delivery of a byte stream. For a database connection or HTTP request, this is ideal. For a 60 Hz game sending 15,000 entity updates per second, TCP has three fatal properties:

1. **Global Head-of-Line (HOL) Blocking.** If packet #47 is lost, TCP holds packets #48–#100 in a receive buffer until #47 is retransmitted. In a game, packets #48–#100 contain fresher position data that makes #47 obsolete. TCP forces the client to wait for a stale packet.

2. **Unnecessary Retransmission.** TCP retransmits lost packets automatically. For a `Position` component at 60 Hz, the retransmitted packet is already 16.6 ms stale when it arrives — the next tick has already overwritten it. The retransmission wastes bandwidth and adds latency.

3. **Congestion Window Recovery.** On packet loss, TCP halves its congestion window (CUBIC/Reno). A single dropped packet in a WiFi handoff causes a throughput collapse that takes hundreds of milliseconds to recover from.

### 2.2 Why Not Raw UDP

Raw UDP solves HOL blocking (there is no ordering) and unnecessary retransmission (there is no retransmission). But it creates new problems:

- **No encryption.** Game packets are plaintext. Client-to-server traffic can be sniffed and replayed.
- **No connection concept.** The server must build its own session management, sequence number tracking, and replay protection.
- **No selective reliability.** Some events (player death, spell cast) *must* be delivered reliably. Building a reliable delivery layer over UDP is a multi-year project (see: ENet, GameNetworkingSockets, Laminar).
- **No congestion control.** A naive UDP sender can saturate the network path and be throttled by ISP middleboxes.

### 2.3 Why QUIC

QUIC (RFC 9000, RFC 9001, RFC 9002) was designed by Google to solve exactly the problems above. It provides:

| Feature | Benefit for Games |
|---|---|
| **Multiplexed streams** | Each `ComponentKind` can have its own stream. Lost packet on Health stream does not block Position stream. |
| **Unreliable datagrams** (RFC 9221) | Fire-and-forget position blasts. No retransmission, no ordering overhead. |
| **Mandatory TLS 1.3** | Every byte is encrypted. No opt-out. 1-RTT handshake (0-RTT on reconnect). |
| **Per-stream flow control** | Backpressure is applied per-stream, not globally. A stalled Inventory stream does not affect Position datagrams. |
| **Connection migration** | QUIC connections survive IP changes (WiFi → cellular). The `ConnectionId` is protocol-level, not IP:port. |
| **No kernel TCP stack** | Implemented in userspace (`quinn` crate). Full control over buffering, pacing, and congestion algorithms. |

---

## 3. Head-of-Line Blocking — The Core Problem

HOL blocking is the single most important networking concept in game engine design. This section explains precisely how QUIC eliminates it.

### 3.1 TCP HOL Blocking

```text
Server sends:    [Pkt1: Position] [Pkt2: Health] [Pkt3: Position] [Pkt4: Death]
Network:         [Pkt1: lost]     [Pkt2: arrives] [Pkt3: arrives]  [Pkt4: arrives]

TCP Client sees: [ waiting... ] [ waiting... ] [ waiting... ] [ Pkt1 retransmit arrives ]
                 → Entire stream is blocked for ~1 RTT (~100ms)
                 → Client renders no updates for 6 ticks
```

### 3.2 QUIC Multi-Stream (No HOL)

```text
Server sends:    [Datagram: Position] [Stream A: Health] [Datagram: Position] [Stream B: Death]
Network:         [Datagram: lost]     [Stream A: arrives] [Datagram: arrives]  [Stream B: arrives]

QUIC Client sees:
  Datagrams:     [✗ dropped (OK)]   [Position frame N+2 renders immediately]
  Stream A:      [Health applied immediately — no dependency on lost datagram]
  Stream B:      [Death applied immediately — independent stream]
  → Zero blocking. Lost Position datagram is overwritten next tick.
```

### 3.3 Quantified Impact

At 60 Hz with 100 ms average RTT and 2% packet loss:

| Protocol | Effective stalls per second | Visual impact |
|---|---|---|
| **TCP** | ~1.2 full stalls × ~100 ms each | Visible per-second freezes |
| **QUIC datagrams** | 0 stalls (lost data simply skipped) | Imperceptible 1-frame glitch |
| **UDP (no reliability)** | 0 stalls but critical events lost | Permanent gameplay desync |

QUIC gives the best of both worlds: datagrams for volatile data, reliable streams for critical events.

---

## 4. QUIC Multi-Stream Architecture

### 4.1 Stream Topology (Phase 3)

In Phase 3, each reliability tier maps to specific QUIC primitives:

```text
QUIC Connection (per client):
  ├── DATAGRAM frames (RFC 9221)      → Volatile tier (Position, Velocity, Rotation)
  │     No ordering, no retransmission, no flow control
  │
  ├── Unidirectional Stream #N        → Critical tier (per ComponentKind)
  │     Reliable delivery, no ordering across streams
  │     One stream per ComponentKind requiring reliability
  │
  └── Bidirectional Stream #0         → Client input commands (upstream)
        Reliable + ordered within the stream
```

### 4.2 Why Per-ComponentKind Streams

If all reliable events share a single stream, a retransmission of `HealthUpdate` packet blocks delivery of `ChatMessage` and `InventoryMutation` — recreating HOL blocking within the reliable tier.

By opening one QUIC unidirectional stream per `ComponentKind`:

```text
UniStream #4:  HealthUpdate     → retransmit does NOT block →
UniStream #7:  DeathEvent       → independent delivery
UniStream #11: ChatMessage      → independent delivery
UniStream #13: InventoryMutation → independent delivery
```

Each stream has its own sequence numbers and flow control. A stall in one stream is invisible to all others.

### 4.3 Stream Lifecycle

- Streams are opened lazily on first use for each `ComponentKind` per client connection.
- Streams are closed when the client disconnects or when the `ComponentKind` is no longer relevant.
- The server limits concurrent streams per client to 256 (matching the 8-bit `ComponentKind` address space in P3).

---

## 5. WebTransport — Browser UDP

### 5.1 The Browser Constraint

Browsers cannot open raw UDP sockets. The WebSocket API runs over TCP (HOL blocking). The only W3C standard that exposes QUIC-level semantics to JavaScript and WASM is **WebTransport**.

WebTransport provides:

| API | Maps To | Use Case |
|---|---|---|
| `transport.datagrams.writable` | QUIC DATAGRAM frames | Volatile entity state (Position) |
| `transport.createUnidirectionalStream()` | QUIC unidirectional stream | Critical events (Health, Death) |
| `transport.createBidirectionalStream()` | QUIC bidirectional stream | Client input commands |

### 5.2 WebTransport vs. WebSocket

| Property | WebSocket | WebTransport |
|---|---|---|
| Transport | TCP | QUIC (UDP) |
| HOL Blocking | Yes (TCP) | No (per-stream) |
| Unreliable delivery | Impossible | `sendDatagram()` |
| Encryption | TLS 1.2+ (optional WSS) | TLS 1.3 (mandatory) |
| Connection migration | No (TCP 4-tuple) | Yes (QUIC ConnectionId) |
| Browser support (2026) | Universal | Chrome, Firefox, Safari |
| Latency on loss | +1 RTT (TCP retransmit) | 0 (datagram skipped) |

### 5.3 Server-Side Implementation

The `wtransport 0.7` crate wraps `quinn` with the HTTP/3 upgrade handshake required by the WebTransport spec. After the upgrade, the connection is a standard QUIC connection. The server's `GameTransport` implementation handles both native QUIC clients and WebTransport browser clients behind a unified `ClientId` via the `MultiTransport` adapter in `aetheris-server`.

See [TRANSPORT_DESIGN.md §6](TRANSPORT_DESIGN.md#6-webtransport--browser-data-plane) for the WebTransport handshake sequence and WASM worker topology.

---

## 6. Dual-Plane Network Topology

Aetheris partitions all network traffic into two physically separate planes. This section explains the networking rationale (for protocol mechanics see [TRANSPORT_DESIGN.md §2](TRANSPORT_DESIGN.md#2-dual-plane-topology)).

### 6.1 Why Separate Planes

| Concern | Data Plane (QUIC/UDP) | Control Plane (gRPC/TCP) |
|---|---|---|
| **Latency** | ≤ 16.6 ms per tick | 50–500 ms acceptable |
| **Delivery** | Selective (unreliable + reliable) | Always reliable + ordered |
| **Schema** | Binary (MessagePack / Bitpack) | Protocol Buffers |
| **Frequency** | 60 Hz continuous | On-demand RPCs |
| **Scaling** | Per-shard (1 server = 1 QUIC endpoint) | Shared (1 server serves many shards) |
| **Monitoring** | Separate counters per plane | Independent dashboards |

Mixing them on a single TCP channel would mean:

- A matchmaking RPC taking 200 ms blocks all position updates for 12 ticks.
- TCP congestion from a burst of inventory mutations reduces game state throughput.
- Debugging latency spikes becomes impossible — is it the game loop or the login API?

### 6.2 Port Allocation

```text
:4433/udp  — Data Plane (QUIC + WebTransport)
:50051/tcp — Control Plane (gRPC + gRPC-Web)
:9000/tcp  — Prometheus metrics scrape endpoint
```

Both planes use TLS 1.3. The Data Plane uses QUIC's built-in TLS. The Control Plane uses gRPC-native TLS.

---

## 7. Congestion Control & Flow Control

### 7.1 QUIC Congestion Control

QUIC mandates congestion control (unlike raw UDP bursts that ISPs may throttle). The `quinn` crate supports multiple algorithms:

| Algorithm | Characteristics | Aetheris Use Case |
|---|---|---|
| **CUBIC** (default) | Conservative, loss-based, slow recovery | General traffic, compatible |
| **BBR v2** | Bandwidth-probing, model-based, fast recovery | Preferred for game traffic (quinn feature) |
| **Custom** | Game-optimized: aggressive on datagrams, conservative on streams | Research (P4) |

**Aetheris P1:** Uses `quinn`'s default CUBIC. While datagrams were traditionally thought to bypass congestion control, per RFC 9221 and the `quinn` implementation, they are subject to it to prevent network collapse. They are paced by the tick scheduler.

**Aetheris P3 target:** BBR v2 for reliable streams. Datagrams are paced by the tick scheduler (one burst per tick, not continuous).

### 7.2 Per-Stream Flow Control

QUIC applies flow control at two levels:

1. **Stream-level:** Each stream has a receive window. A slow consumer (e.g., client processing InventoryMutation slowly) throttles only that stream.
2. **Connection-level:** Total data across all streams is bounded. Prevents a single client from consuming unbounded server memory.

For Aetheris:

```text
Stream-level window:    64 KB per stream (configurable)
Connection-level window: 1 MB per connection
Datagram max size:       1140 bytes (MTU-safe)
```

### 7.3 Application-Level Backpressure

When the server's outbound queue for a client grows beyond threshold:

1. **Priority Channel shedding** — The `PriorityScheduler` in Stage 5 selectively drops or reduces frequency of lower-priority channels (cosmetic → environment → distant) based on per-client `SheddingLevel`. This is the first line of defense and handles most congestion transparently. See [PRIORITY_CHANNELS_DESIGN.md](PRIORITY_CHANNELS_DESIGN.md).
2. **Interest management** reduces the entity update frequency for distant entities.
3. **Quality degradation** switches from field-level deltas to lower-frequency full snapshots.
4. **Disconnect** if the queue exceeds 5 seconds of buffered data (client is irrecoverably behind).

The Priority Channel system leverages QUIC's native multiplexing: each priority channel maps to a separate QUIC stream (Phase 3) with independent flow control, so shedding a low-priority channel does not affect high-priority streams. Inbound client→server traffic is also prioritized — the `IngestPriorityRouter` in Stage 1 sorts incoming messages by channel tag, ensuring combat inputs are processed before chat under server load.

---

## 8. MTU, Fragmentation & Path Discovery

### 8.1 QUIC and MTU

The safe QUIC datagram payload for the global Internet is **1140 bytes** (1200 byte IP packet − 60 bytes QUIC/UDP/IP overhead). This accounts for:

- IPv6 header: 40 bytes
- UDP header: 8 bytes
- QUIC short header: ~12 bytes (variable)

This matches the `MAX_SAFE_PAYLOAD_SIZE = 1200` constant defined in `aetheris-protocol`.

### 8.2 QUIC Path MTU Discovery (PMTUD)

QUIC implementations (including `quinn`) support PMTUD: the endpoint probes with increasingly large packets to discover the actual path MTU. On LAN paths, MTU may be 1400+ bytes. On tunneled paths (VPN, mobile), MTU may be as low as 1280 bytes.

```text
Initial MTU:  1200 bytes (QUIC minimum, guaranteed to work everywhere)
Probed MTU:   Up to 1452 bytes (Ethernet minus IPv6/UDP/QUIC overhead)
Aetheris:     Uses initial 1200 until PMTUD completes (~2 RTTs)
```

### 8.3 Oversized Events

If a single `ReplicationEvent` exceeds the MTU (e.g., a large inventory snapshot):

- **Volatile tier:** The event is split across multiple datagrams with a sequence header. The client reassembles or discards partial arrivals (no retransmission).
- **Critical tier:** Sent on a reliable stream. QUIC handles fragmentation and reassembly transparently.
- **Hard limit:** No single datagram payload exceeds `MAX_SAFE_PAYLOAD_SIZE` bytes.

---

## 9. NAT Traversal & Connectivity

### 9.1 Server-Initiated Model

Aetheris uses a **client-to-server** architecture (not peer-to-peer). NAT traversal is simplified:

- Clients initiate the QUIC connection to a public server IP.
- The server's QUIC endpoint listens on a known port (`:4433`).
- NAT hole-punching is not required — client-to-server UDP works through most NATs without STUN/TURN.

### 9.2 Symmetric NAT (Worst Case)

Some corporate and mobile networks use symmetric NAT, which assigns a different external port for each destination. QUIC handles this via its `ConnectionId` mechanism: even if the NAT rebinds the port, the QUIC connection survives because the server identifies the client by `ConnectionId`, not by `IP:port`.

### 9.3 UDP-Blocked Networks

Some corporate firewalls block all outbound UDP. For these rare cases:

- **P1:** No fallback. Client cannot connect.
- **P3 (planned):** WebSocket fallback tunnel. gRPC-Web already works over TCP for the Control Plane. A WebSocket bridge for the Data Plane would add ~1 RTT latency per message but maintain connectivity.

---

## 10. Latency Budget & Bandwidth Estimation

### 10.1 End-to-End Latency Budget

```text
Client input → Server receives: RTT/2 (~25 ms at 50 ms RTT)
Server processes: 1 tick (16.6 ms)
Server sends → Client receives: RTT/2 (~25 ms)
Client interpolation delay: 100 ms (see CLIENT_DESIGN.md §4.4)
────────────────────────────────────
Total perceived latency: ~167 ms (at 50 ms RTT)
```

### 10.2 Bandwidth Per Client

Phase 1 estimate (full-snapshot MessagePack, 2,500 entities, 10% churn):

| Direction | Data | Rate |
|---|---|---|
| **Server → Client** | 250 entity deltas × ~33 bytes | ~8.25 KB/tick → ~495 KB/s |
| **Client → Server** | 1 InputCommand × ~32 bytes | ~1.9 KB/s |

Phase 3 estimate (bitpack, field-level deltas):

| Direction | Data | Rate |
|---|---|---|
| **Server → Client** | 250 entity deltas × ~7 bytes | ~1.75 KB/tick → ~105 KB/s |
| **Client → Server** | 1 InputCommand × ~12 bytes | ~0.7 KB/s |

At 2,500 concurrent clients (P3):

```text
Total server outbound: 2,500 × 105 KB/s ≈ 262 MB/s
→ Requires 10 Gbps NIC headroom (achievable on modern servers)
```

---

## 11. Connection Resilience & Reconnection

### 11.1 QUIC Connection Migration

When a client's IP changes (WiFi → cellular, VPN reconnect), QUIC maintains the session via `ConnectionId`. The client sends a PATH_CHALLENGE, the server responds with PATH_RESPONSE, and the connection continues without re-handshaking.

### 11.2 Reconnection Strategy

If the QUIC connection is fully lost (timeout, network outage):

1. **Heartbeat timeout:** 10 seconds of silence triggers server-side cleanup (entity kept in ECS for reconnection window).
2. **Reconnection window:** 30 seconds. During this window, the entity remains in the ECS with a `Frozen` state. Other clients see the entity standing still.
3. **Token refresh:** The client calls `AuthService.Authenticate()` to get a fresh session token, then reconnects to the Data Plane.
4. **State reconciliation:** On reconnect, the server sends a full entity snapshot to the client, and the client resumes from the reconciled state.
5. **Window expired:** After 30 seconds, the entity is despawned. The client must go through full matchmaking again.

### 11.3 0-RTT Reconnection (P3)

QUIC supports 0-RTT resumption using cached TLS session keys. On the second connection attempt within the TLS ticket lifetime (typically 24 hours), the client can send application data with the initial QUIC handshake packet — eliminating one full RTT from the reconnection latency.

**Security caveat:** 0-RTT data is replayable. Aetheris only sends the connect token in 0-RTT. Game data is never sent in 0-RTT to prevent replay attacks.

---

## 12. Performance Contracts

| Metric | Target | Measurement |
|---|---|---|
| QUIC handshake latency | ≤ 1 RTT (1-RTT mode) | `quinn` connection establishment |
| Datagram delivery latency | ≤ 0.5 ms server-side (encode + send) | `aetheris_transport_send_duration_ms` |
| Heartbeat interval | 2 seconds | Configurable via `ServerConfig` |
| Connection timeout | 10 seconds | Configurable |
| Max concurrent clients per endpoint | 10,000 (P3) | Load test target |
| Server outbound bandwidth per client | ≤ 500 KB/s (P1), ≤ 120 KB/s (P3) | `aetheris_transport_bytes_sent_per_tick` |
| Packet loss tolerance (volatile tier) | 100% (by design) | No retransmission |
| Packet loss tolerance (critical tier) | 0% (by design) | QUIC reliable stream |

---

## 13. Open Questions

| Question | Context | Impact |
|---|---|---|
| **BBR vs CUBIC for Game Traffic** | BBR is more aggressive in probing bandwidth. Does this cause latency spikes on lossy WiFi? | Congestion control algorithm selection for P3. |
| **WebSocket Fallback** | Should we implement a TCP WebSocket fallback for UDP-blocked corporate networks? | Accessibility vs. engineering cost. |
| **QUIC 0-RTT Security** | Should 0-RTT be disabled entirely to prevent replay attacks, or gated to only the connect token? | Reconnection speed vs. security surface. |
| **Server-to-Server QUIC** | Should inter-shard communication (P4 federation) use QUIC or gRPC? | Latency of entity hand-over protocol. |

---

## Appendix A — Glossary

### Mini-Glossary (Quick Reference)

- **HOL Blocking**: Head-of-line blocking — a lost packet stalls all subsequent packets in the same stream.
- **QUIC**: A UDP-based transport protocol providing multiplexed streams, datagrams, and mandatory encryption (RFC 9000).
- **WebTransport**: A W3C browser API exposing QUIC semantics (datagrams + streams) to JavaScript and WASM.
- **MTU**: Maximum Transmission Unit — the largest packet a network path can carry without fragmentation.
- **PMTUD**: Path MTU Discovery — probing to determine the actual MTU of the network path.
- **Congestion Control**: Algorithms (CUBIC, BBR) that regulate send rate to avoid network saturation.
- **Connection Migration**: QUIC's ability to maintain a session when the client's IP address changes.
- **0-RTT**: Zero Round Trip Time — sending application data with the first handshake packet on reconnection.
- **Unreliable Datagram**: A packet that may be lost or arrive out of order (used for volatile game state).

[Full Glossary Document](../GLOSSARY.md)

---

## Appendix B — Decision Log

| # | Decision | Rationale | Revisit If... | Date |
|---|---|---|---|---|
| D1 | QUIC over raw UDP | Encryption, selective reliability, and connection migration justify the slight overhead. | QUIC overhead measurably exceeds 5% of tick budget. | 2026-04-15 |
| D2 | WebTransport for browsers | Only browser API with UDP-like semantics. No alternative exists. | A new browser API provides better performance. | 2026-04-15 |
| D3 | 1200 bytes initial MTU | QUIC minimum, guaranteed to traverse all Internet paths. | PMTUD proves reliably effective on target audience networks. | 2026-04-15 |
| D4 | No WebSocket fallback in P1 | UDP-blocked networks are <2% of target audience. | User analytics shows >5% connection failures. | 2026-04-15 |
| D5 | Dual-plane topology | Prevents metagame latency from contaminating the game loop. Enables independent scaling. | A unified QUIC-based RPC framework proves simpler. | 2026-04-15 |
| D6 | CUBIC for P1, BBR target for P3 | CUBIC is the safe default. BBR requires measurement under real game traffic patterns. | BBR performs worse than CUBIC on lossy WiFi in stress tests. | 2026-04-15 |
| D7 | Priority Channels as first backpressure mechanism | Channel-level shedding is more surgical than global quality degradation. See [PRIORITY_CHANNELS_DESIGN.md](PRIORITY_CHANNELS_DESIGN.md). | If shedding overhead exceeds the bandwidth savings. | 2026-04-15 |
