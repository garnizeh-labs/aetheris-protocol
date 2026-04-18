# aetheris-encoder-serde

A MessagePack encoder implementation for the Aetheris Protocol.

## Overview

A rapid-iteration `Encoder` implementation using MessagePack (`rmp-serde`). Optimized for schema flexibility during development.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
aetheris-encoder-serde = "0.2.0"
```

Then initialize the encoder:

```rust
use aetheris_encoder_serde::SerdeEncoder;
// Use with aetheris-protocol traits
```

For more details, see the [main repository README](https://github.com/garnizeh-labs/aetheris-protocol).
