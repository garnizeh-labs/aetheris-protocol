# Aetheris Protocol

Binary contracts, lifecycle management, and communication traits for the Aetheris Engine.

## The Authority of the Wire

In a 60Hz authoritative simulation, the network protocol is not just a data format — it is the single source of truth. **Aetheris Protocol** provides the high-performance, media-agnostic contracts that allow the client and server to synchronize massive world states with sub-millisecond overhead. It handles the extraction of ECS deltas, high-frequency replication, and message reassembly across unreliable UDP channels.

This crate isolates the engine's logical communication from concrete implementations, ensuring that simulation code remains pure and transport-independent.

> **[Read the Architecture Design Document](docs/PROTOCOL_DESIGN.md)** — traits, encoders, and wire format specifications.
>
> 🚀 **Latest Milestone:** **Hardening & Standardization (M10146) complete!** Fixed atomic sequence wraparound, enforced MTU limits in all encoders, and gated traits for first-class WASM support.

---

[![CI](https://github.com/garnizeh-labs/aetheris-protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/garnizeh-labs/aetheris-protocol/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.95%2B-blue.svg?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg)](https://conventionalcommits.org)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](https://github.com/garnizeh-labs/aetheris-protocol/pulls)

## The Three Pillars

The Aetheris Protocol is built on three core trait facades that isolate the engine from the outside world:

1.  **`GameTransport`**: Abstract network layer handling reliable/unreliable datagrams and event polling. Gated for WASM compat (`?Send` futures).
2.  **`WorldState`**: The ECS bridge. Translates protocol-level `NetworkId`s to local ECS entities and extracts high-frequency replication deltas.
3.  **`Encoder`**: The serialization engine. Supports everything from rapid-iteration `rmp-serde` to Phase 3 custom bit-packing.

## Quickstart

```bash
# 1. Run the quality gate (fmt, clippy, tests, security)
just check

# 2. Run the FULL CI gate (includes udeps and strict docs)
just check-all

# 3. Synchronize formatting and apply clippy suggestions
just fix
```

### 🛠️ Common Tasks

| Command | Category | Description |
| :--- | :--- | :--- |
| `just check` | **Quality** | Fast local validation: fmt, clippy, unit tests, and security audit. |
| `just check-all`| **CI** | Comprehensive validation: includes `udeps` and strict rustdoc checks. |
| `just fix` | **Lint** | Automatically formats code and applies non-breaking clippy fixes. |
| `just udeps` | **Lint** | Checks for unused dependencies (requires pinned nightly). |
| `just docs` | **Doc** | Generates the official API documentation. |
| `just release` | **Deploy** | Bumps version, updates CHANGELOG, and prepares the release commit. |

For a full list of commands, run `just --list`.

## Documentation Index

- **[PROTOCOL_DESIGN.md](docs/PROTOCOL_DESIGN.md):** The master wire format and trait contract specification.
- **[TRANSPORT_DESIGN.md](docs/TRANSPORT_DESIGN.md):** Reliable/Unreliable abstraction and channel mechanics.
- **[ENCODER_DESIGN.md](docs/ENCODER_DESIGN.md):** Serialization strategies (rmp-serde / bit-packer).
- **[NETWORKING_DESIGN.md](docs/NETWORKING_DESIGN.md):** The engine's networking fundamentals (UDP, QUIC, WebTransport).
- **[CONTROL_PLANE_DESIGN.md](docs/CONTROL_PLANE_DESIGN.md):** Transactional services, authentication, and matchmaking architecture.
- **[API_DESIGN.md](docs/API_DESIGN.md):** High-level event structures and async patterns.

## Design Philosophy

1.  **Implementation Agnostic**: The game loop never speaks to `renet` or `quinn` directly; it only speaks `GameTransport`.
2.  **Allocation-Aware**: Designed to minimize (and eventually eliminate) heap allocations in the hot replication path.
3.  **WASM First**: All traits feature optional `?Send` bounds to support browser main-thread execution without compromise.

---

License: MIT / Apache-2.0
