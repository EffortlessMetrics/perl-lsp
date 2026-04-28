//! LSP feature system (absorbed from perl-lsp-feature-* crates).
//!
//! This module aggregates the following absorbed crates as submodules:
//! - perl-lsp-feature-ids → ids
//! - perl-lsp-feature-contracts → contracts
//! - perl-lsp-feature-flags → flags
//! - perl-lsp-feature-profile → profile
//! - perl-lsp-feature-profile-cli → profile_cli
//! - perl-lsp-feature-policy → policy
//! - perl-lsp-feature-grid → grid

pub mod contracts;
pub mod flags;
pub mod grid;
pub mod ids;
pub mod policy;
pub mod profile;
pub mod profile_cli;
