#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![deny(unsafe_code)]

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
