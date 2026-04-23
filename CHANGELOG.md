## [0.2.13] - 2026-04-22

### 🚀 Features

- *(protocol)* Add `ReplicationBatch` variant to `NetworkEvent` and `WireEvent` to support grouping multiple entity updates into a single network payload.
- *(protocol)* Add `WireEvent::into_network_event(client_id)` helper for safe conversion from over-the-wire events to local `NetworkEvent` context.
- *(protocol)* Add `WorldState::extract_reliable_events` with default empty implementation.
- *(protocol)* Add `WorldState::spawn_session_ship` to support authoritative possession flows.

### 🚜 Refactor

- Standardized `AuthService` trait to be used internally by the engine, decoupling it from gRPC-specific implementations.
- Refactored `NetworkEvent` variants to strictly separate local events (`ClientConnected`, etc.) from wire-safe events.

### 🐛 Bug Fixes

- Resolved type inference issues in `MockEncoder` and `MockTransport` test doubles.
- Stabilized replication batching by explicitly separating reliable and unreliable dispatch paths.

## [0.2.12] - 2026-04-22

### 🚀 Features

- Extract protocol and encoder crates to standalone repo (clean)
- *(protocol)* Harden MTU limits, add event derives, and standardize CI infrastructure
- *(protocol)* Standardize justfile, fix doc links, cleanup deny.toml, and harden infrastructure (part 2)
- *(protocol)* Finalize hardening with ci profiles, safe releases, and mock connection enforcement

### 🐛 Bug Fixes

- *(protocol)* Resolve clippy warnings and formatting discrepancies
- *(protocol)* Resolve udeps false positives and cargo.toml duplicate warnings

### 🚜 Refactor

- Enhance MalformedPayload with descriptive error messages

### 📚 Documentation

- Initial commit of protocol documentation
- Enhance README with technical summary and links
- Align README with aetheris premium template
- Fix cross-repo links and stabilize udeps CI
- *(protocol)* Overhaul readme with premium narrative and architectural pillars
- *(protocol)* Precision refinement of readme to match aetheris standards

### 🎨 Styling

- Reformat error message string in test_doubles.rs for better readability

### ⚙️ Miscellaneous Tasks

- *(protocol)* Finalize rust 1.95.0 standardization across CI and manifests
- *(protocol)* Clean up stale entry in deny.toml
- Resolve cargo-deny failures and clean up manifest warnings
- Bump workspace version to 0.2.0 and configure cargo-udeps ignore list for build dependencies
- Add git-cliff configuration
