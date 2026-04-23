// Build script for generating feature-contract support artifacts.
// Wave Final PR B: pearl-feature-catalog absorbed into feature_catalog.rs.
// Build-time catalog logic is inlined via include!().
#![allow(clippy::pedantic, clippy::panic)]

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

// Import catalog helpers inlined from build_catalog.rs
// (separate file because build.rs is its own compilation unit)
mod catalog {
    #![allow(dead_code)] // DAP-specific helpers used only by perl-dap/build.rs
    include!("build_catalog.rs");
}

use catalog::{
    load_catalog_for_build, render_lsp_fallback_module, render_lsp_feature_catalog_module,
};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR").map_err(
        |_| "OUT_DIR must be set by cargo during build - this is a build-time requirement",
    )?;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let dest_path = Path::new(&out_dir).join("feature_contracts.rs");

    println!("cargo:rerun-if-env-changed=FEATURES_TOML_OVERRIDE");

    match load_catalog_for_build(Path::new(&manifest_dir)) {
        Ok((catalog, source)) => {
            println!("cargo:rerun-if-changed={}", source.path.display());
            let code = render_lsp_feature_catalog_module(&catalog, source.comment());
            fs::write(&dest_path, code).map_err(|error| {
                format!("Failed to write feature_contracts.rs to {:?}: {error}", dest_path)
            })?;
        }
        Err(error) => {
            eprintln!("Warning: failed to load LSP feature catalog from features.toml: {error}");
            let code = render_lsp_fallback_module();
            fs::write(&dest_path, code).map_err(|error| {
                format!("Failed to write fallback feature_contracts.rs to {:?}: {error}", dest_path)
            })?;
        }
    }

    Ok(())
}
