# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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

### 🚀 Features

- *(protocol)* Add component kind ranges and input command bits for VS-01

### 📚 Documentation

- Formalize ComponentKind reservation policy and update milestone
- Refine reservation policy and bump spec version to 0.1.2
## [0.2.7] - 2026-04-20

### 🚀 Features

- *(protocol)* Add entity_type to Transform struct

### 📚 Documentation

- *(protocol)* Update README with Transform hardening
## [0.2.6] - 2026-04-20

### 🚀 Features

- Synchronize ECS components and ship stats per M1020

### ⚙️ Miscellaneous Tasks

- *(protocol)* Bump version 2 and harden InputCommand/ShipStats
## [0.2.5] - 2026-04-19

### 🚀 Features

- *(protocol)* Add NetworkEvent::Disconnected variant

### 🐛 Bug Fixes

- *(protocol)* Address code review feedback for reassembler and events
- *(protocol)* Restore reassembler compatibility and harden documentation examples
- *(protocol)* Fix badge label and harden release workflow atomicity

### 📚 Documentation

- Update README badges to include Crates.io and Docs.rs links
## [0.2.4] - 2026-04-19

### 🐛 Bug Fixes

- *(protocol)* Handle mutex poisoning in transport test doubles
## [0.2.3] - 2026-04-19

### 🚀 Features

- Refactor GameTransport::poll_events to return Result
## [0.2.2] - 2026-04-19
## [0.2.1] - 2026-04-18

### 📚 Documentation

- Add README.md files to all workspace crates
## [0.2.0] - 2026-04-18

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

### 🎨 Styling

- Reformat error message string in test_doubles.rs for better readability

### ⚙️ Miscellaneous Tasks

- Resolve cargo-deny failures and clean up manifest warnings
- Bump workspace version to 0.2.0 and configure cargo-udeps ignore list for build dependencies
