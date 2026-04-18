//! Aetheris serde-based encoder logic.
//!
//! **Phase:** P1 - MVP Implementation
//! **Constraint:** Uses `rmp-serde` (`MessagePack`) for binary serialization of Serde-compatible types.
//! **Purpose:** Enables rapid prototyping of the replication protocol with automated
//! serialization, prior to deploying bit-packed optimizations.

#![warn(clippy::all, clippy::pedantic)]

mod serde_encoder;

pub use serde_encoder::SerdeEncoder;
