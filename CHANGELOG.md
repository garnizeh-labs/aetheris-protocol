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
