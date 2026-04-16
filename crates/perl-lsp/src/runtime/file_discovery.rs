//! Compatibility re-exports for workspace file discovery.
//!
//! The implementation lives in the standalone `perl-workspace-discovery` crate.

pub use perl_workspace::discovery::{DiscoveryMethod, DiscoveryResult, discover_perl_files};
