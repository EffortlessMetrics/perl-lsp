//! Compatibility re-exports for workspace file discovery.
//!
//! The implementation lives in `perl_workspace::discovery` (collapsed from the
//! standalone `perl-workspace-discovery` satellite crate in Wave A, #4426).

pub use perl_workspace::discovery::{DiscoveryMethod, DiscoveryResult, discover_perl_files};
