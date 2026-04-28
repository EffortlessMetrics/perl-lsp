//! Re-export binary functionality for testing.
//!
//! This module is primarily used to enable `cargo test --lib` CI runs.
//! The primary entry point is the binary in `main.rs`.

pub mod version_sync;
