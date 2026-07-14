#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]

/// Crate-wide test support. Shared, process-global synchronization for tests
/// that mutate environment variables — `PATH` in particular is read by
/// production code (`platform::*`, `config::perl_oracle_env`) and mutated by
/// `set_var`/`remove_var` (which Rust 2024 made `unsafe` precisely because the
/// process environment is a shared global). Every test that mutates `PATH` MUST
/// hold [`test_support::PATH_ENV_LOCK`] for the duration of the mutation +
/// restore, so all such tests serialize against the same guard rather than each
/// relying on a function-local lock that only excludes itself.
#[cfg(test)]
pub(crate) mod test_support {
    /// Process-global lock serializing every `PATH`-mutating test in this crate.
    pub(crate) static PATH_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

/// Helpers for translating feature catalog entries into client capability checks.
pub mod capability_map;
/// Runtime configuration loading, validation, and compatibility adapters.
pub mod config;
/// Parser for Perl::Critic output emitted by external lint runs.
pub mod critic_parser;
/// Feature catalog parsing and generation utilities shared by build/runtime code.
pub mod feature_catalog;
/// Feature model, identifiers, and registry plumbing for capability gating.
pub mod features;
/// Policy and governance APIs for feature profiles and rollout controls.
pub mod governance;
/// Hashing helpers shared by workspace tooling and verification pipelines.
pub mod hashing;
/// Performance-focused caches and allocation strategies for large workspaces.
pub mod performance;
/// Cross-platform interpreter and toolchain detection helpers.
pub mod platform;
/// JSON-RPC and LSP protocol types used across providers and transport layers.
pub mod protocol;
/// Language Server Protocol request/notification provider implementations.
pub mod providers;
/// Request lifecycle, scheduling, and runtime orchestration infrastructure.
pub mod runtime;
/// Integrations for external tools such as `perlcritic` and `perltidy`.
pub mod tooling;
/// Message framing and stream transport glue for stdio/socket communication.
pub mod transport;
/// URI parsing and conversion helpers used by protocol-facing components.
pub mod uri;
