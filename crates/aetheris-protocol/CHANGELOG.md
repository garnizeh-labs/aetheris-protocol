# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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
