# Aetheris Protocol

Binary protocol contracts for the Aetheris multiplayer engine — traits, encoders, and protobuf definitions.

## The Foundation of Determinism

In a 60Hz authoritative simulation, the protocol is the law. **Aetheris Protocol** defines the high-performance binary contracts that allow the client and server to share a single source of truth with minimum overhead. It handles the lifecycle of every network-aware entity, from the initial handshake to the high-frequency tick replication across various transport layers.

> **[Read the Protocol Design Document](docs/PROTOCOL_DESIGN.md)** — traits, encoders, and protobuf definitions.
>
> 🚀 **Latest Milestone:** **Protocol Extraction Complete (M10145)!** Solidified the binary contract as a standalone, zero-dependency library.

[![CI](https://github.com/garnizeh-labs/aetheris-protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/garnizeh-labs/aetheris-protocol/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.94%2B-blue.svg?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Quickstart

```bash
# 1. Run full quality gate (fmt, clippy, tests, benchmarks, security)
just check

# 2. Build documentation
just docs
```

### 🛠️ Common Tasks

| Command | Category | Description |
| :--- | :--- | :--- |
| `just check` | **Quality** | Complete PR validation: Linters, tests, security, and performance. |
| `just docs` | **Doc** | Generate technical API documentation. |

For a full list of commands, run `just --list`.

## Documentation Entry Points

- **[PROTOCOL_DESIGN.md](docs/PROTOCOL_DESIGN.md):** Binary contract and trait definitions.
- **[ENCODER_DESIGN.md](docs/ENCODER_DESIGN.md):** Serialization strategies (rmp-serde / bitpack).
- **[TRANSPORT_DESIGN.md](docs/TRANSPORT_DESIGN.md):** Media-agnostic transport abstractions.

## Design Philosophy

1. **Protocol-First:** The binary contract is defined independently of the ECS or transport implementation.
2. **Deterministic-Ready:** Minimal allocations and predictable serialization sizes.
3. **Zero-Overhead Abstractions:** Traits designed to be optimized away by the compiler.

---
License: MIT / Apache-2.0
