#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![deny(unsafe_code)]

/// Shared capability map utilities used by feature providers.
pub mod capability_map;
/// Configuration models and parsing utilities for the core runtime.
pub mod config;
/// Integration helpers for Perl::Critic output parsing.
pub mod critic_parser;
/// Capability and feature metadata catalog definitions.
pub mod feature_catalog;
/// Feature flag, profile, and policy infrastructure.
pub mod features;
/// Governance helpers for feature lifecycle and enforcement.
pub mod governance;
/// Performance instrumentation and runtime measurement support.
pub mod performance;
/// Platform-specific compatibility helpers.
pub mod platform;
/// Protocol-level types and glue code for language tooling.
pub mod protocol;
/// Language Server Protocol provider implementations.
pub mod providers;
/// Runtime support modules (limits, cancellation, launch, and validation).
pub mod runtime;
/// Developer tooling utilities used by the core crate.
pub mod tooling;
/// Transport abstractions and message handling utilities.
pub mod transport;
/// URI parsing and normalization helpers.
pub mod uri;
