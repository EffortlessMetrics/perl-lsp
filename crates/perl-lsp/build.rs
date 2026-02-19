// Build script - panics are idiomatic for failing builds
#![allow(clippy::pedantic, clippy::panic)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get git tag for embedding in version output
    let tag = std::process::Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into());

    println!("cargo:rustc-env=GIT_TAG={}", tag.trim());

    // Configure check-cfg for test-only cfg attributes
    println!("cargo:rustc-check-cfg=cfg(ci)");

    // Also try to get the exact tag if we're on one
    let exact_tag = std::process::Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok());

    if let Some(tag) = exact_tag {
        println!("cargo:rustc-env=GIT_EXACT_TAG={}", tag.trim());
    }

    // Re-run build script when HEAD or refs change, so GIT_TAG stays fresh
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/");
    println!("cargo:rerun-if-changed=../../.git/packed-refs");

    // Generate feature catalog from features.toml
    generate_feature_catalog()?;
    Ok(())
}

#[derive(serde::Deserialize)]
struct Meta {
    version: String,
    lsp_version: String,
}

#[derive(serde::Deserialize)]
struct Feature {
    id: String,
    area: Option<String>,
    spec: Option<String>,
    maturity: String,
    advertised: Option<bool>,
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct Catalog {
    meta: Meta,
    feature: Vec<Feature>,
}

fn read_catalog() -> Result<Catalog, Box<dyn std::error::Error>> {
    // Rebuild when override env changes
    println!("cargo:rerun-if-env-changed=FEATURES_TOML_OVERRIDE");

    // Allow test-time override for negative testing
    if let Ok(override_path) = env::var("FEATURES_TOML_OVERRIDE") {
        let p = PathBuf::from(override_path);
        if p.exists() {
            println!("cargo:rerun-if-changed={}", p.display());
            let content = fs::read_to_string(&p)
                .map_err(|e| format!("Failed to read override features from {:?}: {}", p, e))?;
            return toml::from_str(&content)
                .map_err(|e| format!("Failed to parse override features: {}", e).into());
        }
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

    // Try workspace root first
    let workspace_root =
        manifest_dir.parent().and_then(|p| p.parent()).map(|p| p.join("features.toml"));

    // Then try vendored copy
    let vendored = manifest_dir.join("features_sot.toml");

    let source_path = if let Some(ws) = workspace_root {
        if ws.exists() {
            println!("cargo:rerun-if-changed={}", ws.display());
            ws
        } else if vendored.exists() {
            println!("cargo:rerun-if-changed={}", vendored.display());
            vendored
        } else {
            return Err("features.toml not found in workspace root or as vendored copy".into());
        }
    } else if vendored.exists() {
        println!("cargo:rerun-if-changed={}", vendored.display());
        vendored
    } else {
        return Err("features.toml not found".into());
    };

    let content = fs::read_to_string(&source_path)?;
    Ok(toml::from_str(&content)?)
}

fn generate_feature_catalog() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR").map_err(
        |_| "OUT_DIR must be set by cargo during build - this is a build environment issue",
    )?;
    let dest_path = Path::new(&out_dir).join("feature_catalog.rs");

    match read_catalog() {
        Ok(mut catalog) => {
            // Validate catalog: check for duplicates and valid maturity values
            let mut seen = std::collections::HashSet::new();
            for f in &catalog.feature {
                if !seen.insert(&f.id) {
                    return Err(format!("Duplicate feature id in features.toml: {}", f.id).into());
                }
                match f.maturity.as_str() {
                    "experimental" | "preview" | "ga" | "planned" | "production" => {}
                    other => {
                        return Err(
                            format!("Unknown maturity {:?} for feature {}", other, f.id).into()
                        );
                    }
                }
            }

            // Sort features deterministically by area then ID
            catalog.feature.sort_by(|a, b| {
                let area_a = a.area.as_deref().unwrap_or("");
                let area_b = b.area.as_deref().unwrap_or("");
                area_a.cmp(area_b).then_with(|| a.id.cmp(&b.id))
            });

            // Filter advertised GA features (and sort them too)
            let mut advertised: Vec<String> = catalog
                .feature
                .iter()
                .filter(|f| {
                    f.advertised.unwrap_or(false)
                        && (f.maturity == "ga" || f.maturity == "production")
                })
                .map(|f| f.id.clone())
                .collect();
            advertised.sort();

            // Calculate compliance percentage
            let trackable: usize =
                catalog.feature.iter().filter(|f| f.maturity != "planned").count();

            let percent = if trackable == 0 {
                0.0
            } else {
                ((advertised.len() as f64) / (trackable as f64) * 100.0).round() as f32
            };

            // Generate Rust code
            let mut code = String::new();
            code.push_str("// @generated by build.rs; DO NOT EDIT.\n");
            let source = if env::var("FEATURES_TOML_OVERRIDE").is_ok() {
                "// source: FEATURES_TOML_OVERRIDE\n"
            } else {
                "// source: features.toml\n"
            };
            code.push_str(source);
            code.push_str("#[allow(dead_code, clippy::all)]\n\n");

            // Constants
            code.push_str("/// Current parser version extracted from features.toml metadata\n");
            code.push_str(&format!("pub const VERSION: &str = {:?};\n", catalog.meta.version));
            code.push_str("/// LSP protocol version supported by this parser implementation\n");
            code.push_str(&format!(
                "pub const LSP_VERSION: &str = {:?};\n",
                catalog.meta.lsp_version
            ));
            code.push_str(
                "/// Compliance percentage of advertised GA features vs trackable features\n",
            );
            code.push_str(&format!("pub const COMPLIANCE_PERCENT: f32 = {:.2};\n\n", percent));

            // Feature struct
            code.push_str(
                "/// Represents a single LSP feature with its metadata and implementation status\n",
            );
            code.push_str("#[derive(Debug, Clone)]\n");
            code.push_str("pub struct Feature {\n");
            code.push_str(
                "    /// Unique identifier for this feature (e.g., \"textDocument/hover\")\n",
            );
            code.push_str("    pub id: &'static str,\n");
            code.push_str(
                "    /// LSP specification reference or version where this feature is defined\n",
            );
            code.push_str("    pub spec: &'static str,\n");
            code.push_str("    /// Functional area this feature belongs to (e.g., \"completion\", \"diagnostics\")\n");
            code.push_str("    pub area: &'static str,\n");
            code.push_str("    /// Implementation maturity level: \"experimental\", \"preview\", \"ga\", \"planned\", \"production\"\n");
            code.push_str("    pub maturity: &'static str,\n");
            code.push_str("    /// Whether this feature is advertised to LSP clients in server capabilities\n");
            code.push_str("    pub advertised: bool,\n");
            code.push_str("    /// Human-readable description of the feature's functionality\n");
            code.push_str("    pub description: &'static str,\n");
            code.push_str("}\n\n");

            // All features array
            code.push_str(
                "/// Comprehensive catalog of all LSP features with their implementation status\n",
            );
            code.push_str("pub const ALL_FEATURES: &[Feature] = &[\n");
            for feature in &catalog.feature {
                code.push_str("    Feature {\n");
                code.push_str(&format!("        id: {:?},\n", feature.id));
                code.push_str(&format!(
                    "        spec: {:?},\n",
                    feature.spec.as_deref().unwrap_or("")
                ));
                code.push_str(&format!(
                    "        area: {:?},\n",
                    feature.area.as_deref().unwrap_or("")
                ));
                code.push_str(&format!("        maturity: {:?},\n", feature.maturity));
                code.push_str(&format!(
                    "        advertised: {},\n",
                    feature.advertised.unwrap_or(false)
                ));
                code.push_str(&format!(
                    "        description: {:?},\n",
                    feature.description.as_deref().unwrap_or("")
                ));
                code.push_str("    },\n");
            }
            code.push_str("];\n\n");

            // Advertised features function
            code.push_str("/// Returns a list of feature IDs that are advertised to LSP clients\n");
            code.push_str(
                "/// Only includes GA/production features that are ready for client consumption\n",
            );
            code.push_str("pub fn advertised_features() -> Vec<&'static str> {\n");
            code.push_str("    vec![\n");
            for id in &advertised {
                code.push_str(&format!("        {:?},\n", id));
            }
            code.push_str("    ]\n");
            code.push_str("}\n\n");

            // Has feature function
            code.push_str("/// Checks if a specific feature ID is advertised and available\n");
            code.push_str(
                "/// Returns true only for GA/production features ready for client use\n",
            );
            code.push_str("pub fn has_feature(id: &str) -> bool {\n");
            code.push_str("    advertised_features().contains(&id)\n");
            code.push_str("}\n\n");

            // Compliance percent function
            code.push_str("/// Returns the current LSP compliance percentage as a float\n");
            code.push_str(
                "/// Calculated as (advertised GA features / trackable features) * 100\n",
            );
            code.push_str("pub fn compliance_percent() -> f32 {\n");
            code.push_str("    COMPLIANCE_PERCENT\n");
            code.push_str("}\n");

            fs::write(&dest_path, code).map_err(|e| {
                format!("Failed to write feature_catalog.rs to {:?}: {}", dest_path, e)
            })?;
        }
        Err(e) => {
            eprintln!("Warning: Failed to generate feature catalog: {}", e);
            eprintln!("Generating minimal feature catalog fallback");

            // Generate minimal fallback
            let mut code = String::new();
            code.push_str("// Auto-generated minimal catalog - features.toml not found\n\n");
            code.push_str("#![allow(dead_code)]\n\n");
            code.push_str(
                "/// Minimal feature structure for fallback when features.toml is not available\n",
            );
            code.push_str("pub struct Feature { }\n");
            code.push_str("/// Fallback parser version when features.toml is not available\n");
            code.push_str("pub const VERSION: &str = \"0.9.1\";\n");
            code.push_str("/// Fallback LSP version when features.toml is not available\n");
            code.push_str("pub const LSP_VERSION: &str = \"3.18\";\n");
            code.push_str(
                "/// Fallback compliance percentage when features.toml is not available\n",
            );
            code.push_str("pub const COMPLIANCE_PERCENT: f32 = 0.0;\n");
            code.push_str(
                "/// Empty features array for fallback when features.toml is not available\n",
            );
            code.push_str("pub const ALL_FEATURES: &[Feature] = &[];\n");
            code.push_str("/// Returns empty list when features.toml is not available\n");
            code.push_str("pub fn advertised_features() -> Vec<&'static str> { vec![] }\n");
            code.push_str("/// Always returns false when features.toml is not available\n");
            code.push_str("pub fn has_feature(_id: &str) -> bool { false }\n");
            code.push_str("/// Returns zero compliance when features.toml is not available\n");
            code.push_str("pub fn compliance_percent() -> f32 { 0.0 }\n");

            fs::write(&dest_path, code).map_err(|e| {
                format!("Failed to write minimal feature_catalog.rs to {:?}: {}", dest_path, e)
            })?;
        }
    }
    Ok(())
}
