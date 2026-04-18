//! Aetheris bitpacking encoder logic.
//!
//! **Phase:** P3 - Production Hardening
//! **Constraint:** Zero-allocation, field-level bit-width packing.
//! **Purpose:** Provides maximum bandwidth efficiency by packing component deltas
//! across 32-bit boundaries with quantization.

#![warn(clippy::all, clippy::pedantic)]
