# aetheris-encoder-bitpack

High-performance bit-packing encoder for the Aetheris Protocol.

## Overview

A high-performance Phase 3 `Encoder`. Uses custom bit-packing for maximum data density and minimal MTU footprint in production.

**Features:**
- **Zero-allocation**: Minimal impact on the simulation loop.
- **Field-level bit-width packing**: Pack component deltas across 32-bit boundaries.
- **Quantization**: Built-in support for quantized floating point values.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
aetheris-encoder-bitpack = "0.2.0"
```

For more details, see the [main repository README](https://github.com/garnizeh-labs/aetheris-protocol).
