//! `cargo xtask metrics` subcommand tree.
//!
//! Each leaf module implements one user-facing subcommand.

pub mod diagnostics_stats;
pub mod lsp_stats;
pub mod memory;
pub mod parser_stats;
pub mod ratchet;
pub mod release_health;
pub mod stable_wins;
pub mod sweep_stats;
pub mod workspace_stats;
