//! DAP feature catalog and capability gating helpers.
//!
//! This module exposes the generated debug-feature catalog from `features.toml`.

/// Auto-generated DAP feature catalog from `features.toml`.
#[allow(clippy::all)]
pub mod catalog {
    include!(concat!(env!("OUT_DIR"), "/dap_feature_catalog.rs"));
}

pub use catalog::{advertised_features, has_feature};
