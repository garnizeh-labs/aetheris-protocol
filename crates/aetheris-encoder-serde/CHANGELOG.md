# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.2.16] - 2026-04-23

### ⚙️ Miscellaneous Tasks

- Release v0.2.15
## [0.2.15] - 2026-04-23

### 🐛 Bug Fixes

- Resolve clippy warnings and add encode_event_into to traits
## [0.2.14] - 2026-04-23

### 🚀 Features

- *(protocol)* Finalize replication batching and update changelog
## [0.2.13] - 2026-04-22

### 🚀 Features

- *(protocol)* Add replication batching support for phase 1 performance
- *(protocol)* Implement replication batching with payload size validation
## [0.2.12] - 2026-04-22

### 🚀 Features

- *(protocol)* Consolidate VS-05 and VS-06 protocol changes
## [0.2.11] - 2026-04-21

### 🚀 Features

- *(protocol)* Add GameEvent::AsteroidDepleted for reliable mining lifecycle
- Harden protocol with typed reliable events and InputCommand caps

### 📚 Documentation

- Resolve clippy warnings for backticks and semicolons
## [0.2.10] - 2026-04-20

### 📚 Documentation

- Synchronize workspace crate badges
## [0.2.9] - 2026-04-20

### 📚 Documentation

- Synchronize crate badges across workspace
## [0.2.8] - 2026-04-20

### 📚 Documentation

- Formalize ComponentKind reservation policy and update milestone
## [0.2.7] - 2026-04-20

### 📚 Documentation

- *(protocol)* Update README with Transform hardening
## [0.2.6] - 2026-04-20

### 🚀 Features

- Synchronize ECS components and ship stats per M1020
## [0.2.5] - 2026-04-19

### 🚀 Features

- *(protocol)* Add NetworkEvent::Disconnected variant

### 🐛 Bug Fixes

- *(protocol)* Address code review feedback for reassembler and events
- *(protocol)* Fix badge label and harden release workflow atomicity

### 📚 Documentation

- Update README badges to include Crates.io and Docs.rs links

### 🎨 Styling

- *(protocol)* Fix cargo fmt violations in serde_encoder tests
## [0.2.4] - 2026-04-19
## [0.2.3] - 2026-04-19
## [0.2.2] - 2026-04-19

### 🐛 Bug Fixes

- *(encoder)* Remove overly restrictive buffer check in SerdeEncoder
## [0.2.1] - 2026-04-18

### 📚 Documentation

- Add README.md files to all workspace crates
- Improve README hyphenation and usage examples
## [0.2.0] - 2026-04-18

### 🚀 Features

- Extract protocol and encoder crates to standalone repo (clean)
- *(protocol)* Harden MTU limits, add event derives, and standardize CI infrastructure
- *(protocol)* Standardize justfile, fix doc links, cleanup deny.toml, and harden infrastructure (part 2)
- *(protocol)* Finalize hardening with ci profiles, safe releases, and mock connection enforcement

### 🐛 Bug Fixes

- *(protocol)* Resolve clippy warnings and formatting discrepancies

### 🚜 Refactor

- Enhance MalformedPayload with descriptive error messages
