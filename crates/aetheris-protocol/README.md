# aetheris-protocol

Core traits, types, and gRPC service definitions for the Aetheris Engine.

## Overview

The core engine contracts. Defines the "Trait Triad" (`WorldState`, `GameTransport`, `Encoder`) and the cryptographic Merkle Hash Chain used for entity integrity.

## Features

- **Standard Traits**: Unified interface for transports and encoders.
- **ECS Integration**: Bridge for world state replication.
- **gRPC Support**: Optional gRPC service definitions for control plane operations.
- **Security**: Built-in Merkle Hash Chain for message integrity.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
aetheris-protocol = "0.2.0"
```

For more details, see the [main repository README](https://github.com/garnizeh-labs/aetheris-protocol).
