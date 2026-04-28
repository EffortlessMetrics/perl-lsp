// Build script - panics are acceptable for build failures.
// Wave Final PR B: perl-feature-catalog absorbed; catalog logic inlined via include!().
#![allow(clippy::pedantic, clippy::panic)]

use std::error::Error;
use std::fs;
use std::path::Path;

// Import catalog helpers from the shared rs-core build_catalog.rs
mod catalog {
    #![allow(dead_code)] // LSP-specific helpers used only by perl-lsp-rs-core/build.rs
    include!("../perl-lsp-rs-core/build_catalog.rs");
}

use catalog::{
    DEFAULT_DAP_FEATURES, load_catalog_for_build, render_dap_fallback_module,
    render_dap_feature_catalog_module,
};

fn generate_catalog_module() -> Result<(), Box<dyn Error>> {
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("dap_feature_catalog.rs");

    println!("cargo:rerun-if-env-changed=FEATURES_TOML_OVERRIDE");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;

    let code = match load_catalog_for_build(Path::new(&manifest_dir)) {
        Ok((catalog, source)) => {
            println!("cargo:rerun-if-changed={}", source.path.display());
            let mut source_features = catalog
                .feature
                .iter()
                .filter(|feature| feature.area == "debug" && feature.advertised)
                .map(|feature| feature.id.as_str())
                .collect::<Vec<_>>();
            source_features.sort_unstable();
            source_features.dedup();
            render_dap_feature_catalog_module(&source_features)
        }
        Err(error) => {
            eprintln!("Warning: failed to load DAP feature catalog from features.toml: {error}");
            render_dap_fallback_module(DEFAULT_DAP_FEATURES)
        }
    };

    fs::write(dest_path, code)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    generate_catalog_module()
}
