<div align="center">
  <h1>Aetheris Protocol</h1>
  <p>Binary contracts, lifecycle management, and communication traits for the Aetheris Engine.</p>

  [![CI](https://img.shields.io/github/actions/workflow/status/garnizeh-labs/aetheris-protocol/ci.yml?branch=main&style=flat-square&logo=github&label=CI)](https://github.com/garnizeh-labs/aetheris-protocol/actions)
  [![Rust Version](https://img.shields.io/badge/rust-1.95.0%2B-blue?style=flat-square&logo=rust)](https://www.rust-lang.org/)
  [![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg?style=flat-square)](https://conventionalcommits.org)
  [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](https://github.com/garnizeh-labs/aetheris-protocol/pulls)
</div>

---

## ⚖️ The Authority of the Wire — The Single Source of Truth

In a 60Hz authoritative simulation, the network protocol is not just a data format — it is the single source of truth. **Aetheris Protocol** provides the high-performance, media-agnostic contracts that allow the client and server to synchronize massive world states with sub-millisecond overhead.

> [!TIP]
> 🚀 **Latest Milestone:** **Phase 1 Performance Stabilization (Replication) complete!** Resolved replication concurrency bottlenecks with a Rayon-parallelized encoding pipeline and stabilized the integration test suite for authoritative session flows.

### 📦 Workspace Components

| Crate | Link | Documentation |
| :--- | :--- | :--- |
| **`aetheris-protocol`** | [![Crates.io](https://img.shields.io/crates/v/aetheris-protocol?style=flat-square)](https://crates.io/crates/aetheris-protocol) | [![Docs.rs](https://img.shields.io/docsrs/aetheris-protocol?style=flat-square&logo=docs.rs&label=docs)](https://docs.rs/aetheris-protocol) |
| **`aetheris-encoder-bitpack`** | [![Crates.io](https://img.shields.io/crates/v/aetheris-encoder-bitpack?style=flat-square)](https://crates.io/crates/aetheris-encoder-bitpack) | [![Docs.rs](https://img.shields.io/docsrs/aetheris-encoder-bitpack?style=flat-square&logo=docs.rs&label=docs)](https://docs.rs/aetheris-encoder-bitpack) |
| **`aetheris-encoder-serde`** | [![Crates.io](https://img.shields.io/crates/v/aetheris-encoder-serde?style=flat-square)](https://crates.io/crates/aetheris-encoder-serde) | [![Docs.rs](https://img.shields.io/docsrs/aetheris-encoder-serde?style=flat-square&logo=docs.rs&label=docs)](https://docs.rs/aetheris-encoder-serde) |

---

## Workspace Components

The protocol is split into highly focused crates to ensure lean builds and clear isolation:

- **[`aetheris-protocol`](crates/aetheris-protocol)**: The core engine contracts. Defines the "Trait Triad" (`WorldState`, `GameTransport`, `Encoder`) and the cryptographic Merkle Hash Chain used for entity integrity.
- **[`aetheris-encoder-serde`](crates/aetheris-encoder-serde)**: A rapid-iteration `Encoder` implementation using MessagePack (`rmp-serde`). Optimized for schema flexibility during development.
- **[`aetheris-encoder-bitpack`](crates/aetheris-encoder-bitpack)**: A high-performance Phase 3 `Encoder`. Uses custom bit-packing for maximum data density and minimal MTU footprint in production.

## Quickstart

```bash
# 1. Run the quality gate (fmt, clippy, tests, security)
#    MUST PASS BEFORE OPENING ANY PR
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

For a full list of commands, run `just --list`.

## The Three Pillars

The Aetheris Protocol is built on three core trait facades that isolate the engine from the outside world:

1.  **`GameTransport`**: Abstract network layer handling reliable/unreliable datagrams and event polling. Gated for WASM compat (`?Send` futures).
2.  **`WorldState`**: The ECS bridge. Translates protocol-level `NetworkId`s to local ECS entities and extracts high-frequency replication deltas.
3.  **`Encoder`**: The serialization engine. Supports everything from rapid-iteration `rmp-serde` to Phase 3 custom bit-packing.

## Automated Release Workflow

This repository uses **`release-plz`** to automate versioning and changelog management.

- **Development**: All changes are merged into `main` using **Conventional Commits** (e.g., `feat: ...`, `fix: ...`).
- **Release PR**: On every push to `main`, `release-plz` creates or updates a "Release PR" that bumps versions in `Cargo.toml` and updates `CHANGELOG.md`.
- **Finalizing**: Merging the Release PR automatically triggers a GitHub Release and publishes the updated crates to the registry.

## Documentation Index

- **[PROTOCOL_DESIGN.md](docs/PROTOCOL_DESIGN.md):** The master wire format and trait contract specification.
- **[TRANSPORT_DESIGN.md](docs/TRANSPORT_DESIGN.md):** Reliable/Unreliable abstraction and channel mechanics.
- **[ENCODER_DESIGN.md](docs/ENCODER_DESIGN.md):** Serialization strategies (rmp-serde / bit-packer).
- **[NETWORKING_DESIGN.md](docs/NETWORKING_DESIGN.md):** The engine's networking fundamentals (UDP, QUIC, WebTransport).
- **[CONTROL_PLANE_DESIGN.md](docs/CONTROL_PLANE_DESIGN.md):** Transactional services, authentication, and matchmaking architecture.
- **[API_DESIGN.md](docs/API_DESIGN.md):** High-level event structures and async patterns.

## Design Philosophy

1.  **Trait Facade Architecture**: Strict boundaries between transport, ECS, and logical protocol.
2.  **Phase-Based Evolution**: Iterative protocol hardening (MVP -> Production -> bit-packing).
3.  **Didactic Codebase**: Self-documenting, spec-first implementation designed for learning.

---

License: MIT / Apache-2.0
