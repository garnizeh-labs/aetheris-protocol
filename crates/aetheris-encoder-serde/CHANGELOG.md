# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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
