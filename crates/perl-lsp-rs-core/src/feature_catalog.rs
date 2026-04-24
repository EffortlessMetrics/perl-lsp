//! Shared feature catalog parsing and code-generation helpers.
//!
//! Absorbed from `perl-feature-catalog` crate into `perl-lsp-rs-core`
//! as part of Wave Final PR B (#4541). This module centralizes `features.toml`
//! parsing so LSP, DAP, and xtask all consume the same metadata, validation,
//! and rendering behavior.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Default DAP feature identifiers emitted when catalog processing fails.
pub const DEFAULT_DAP_FEATURES: &[&str] =
    &["dap.breakpoints.basic", "dap.core", "dap.inline_values"];

/// Source metadata for the catalog file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Meta {
    /// Canonical release or feature-set version.
    pub version: String,
    /// LSP version this catalog was built against.
    pub lsp_version: String,
    /// Optional tracked compliance percentage from the catalog source.
    #[serde(default)]
    pub compliance_percent: Option<u32>,
}

/// Feature maturity state used to drive advertising and tracking behavior.
#[derive(
    Debug, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "lowercase")]
pub enum Maturity {
    /// Early-stage feature.
    Experimental,
    /// Feature can be tested but not yet stable.
    Preview,
    /// Fully released and advertizable feature.
    Ga,
    /// Planned work item, not yet implemented or measured.
    Planned,
    /// Fully production-ready and typically exposed like GA.
    Production,
}

impl Maturity {
    /// Returns `true` when the feature contributes to advertised API coverage.
    pub const fn is_advertised(self) -> bool {
        matches!(self, Self::Ga | Self::Production)
    }

    /// Returns `true` when the feature should be considered in compliance math.
    pub const fn is_trackable(self) -> bool {
        !matches!(self, Self::Planned)
    }

    /// Human-readable lowercase label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Preview => "preview",
            Self::Ga => "ga",
            Self::Planned => "planned",
            Self::Production => "production",
        }
    }
}

/// Per-feature catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Feature {
    /// Canonical feature identifier (for example: `lsp.completion`).
    pub id: String,
    /// Spec reference string (for example: `LSP 3.18`).
    #[serde(default)]
    pub spec: String,
    /// Area bucket (`text_document`, `workspace`, `debug`, etc.).
    #[serde(default)]
    pub area: String,
    /// Maturity state.
    pub maturity: Maturity,
    /// Whether this feature is advertised/visible to clients.
    #[serde(default)]
    pub advertised: bool,
    /// Test cases validating the feature.
    #[serde(default)]
    pub tests: Vec<String>,
    /// Include this feature in coverage/compliance accounting.
    #[serde(default = "default_counts_in_coverage")]
    pub counts_in_coverage: bool,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
}

const fn default_counts_in_coverage() -> bool {
    true
}

/// Full catalog loaded from `features.toml`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Catalog {
    /// Shared metadata section.
    pub meta: Meta,
    /// Ordered feature rows.
    pub feature: Vec<Feature>,
}

impl Catalog {
    /// All features in declaration order.
    pub fn features(&self) -> &[Feature] {
        &self.feature
    }

    /// IDs for advertised trackable features (GA/production + `advertised = true`).
    pub fn advertised_feature_ids(&self) -> Vec<&str> {
        let mut ids = self
            .feature
            .iter()
            .filter(|feature| feature.advertised && feature.maturity.is_advertised())
            .map(|feature| feature.id.as_str())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids
    }

    /// IDs for all features in a specific area.
    pub fn area_feature_ids(&self, area: &str) -> Vec<&str> {
        let mut ids: Vec<&str> = self
            .feature
            .iter()
            .filter(|feature| feature.area == area)
            .map(|feature| feature.id.as_str())
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Trackable feature count (`maturity != planned`).
    pub fn trackable_feature_count(&self) -> usize {
        self.feature.iter().filter(|feature| feature.maturity.is_trackable()).count()
    }

    /// Advertised trackable count.
    pub fn advertised_trackable_count(&self) -> usize {
        self.feature
            .iter()
            .filter(|feature| feature.advertised && feature.maturity.is_advertised())
            .count()
    }

    /// Trackable feature count for BDD/compliance grids.
    /// Excludes entries explicitly marked `counts_in_coverage = false`.
    pub fn trackable_feature_count_for_grid(&self) -> usize {
        self.feature
            .iter()
            .filter(|feature| feature.maturity.is_trackable() && feature.counts_in_coverage)
            .count()
    }

    /// Advertised trackable count for BDD/compliance grids.
    /// Excludes entries explicitly marked `counts_in_coverage = false`.
    pub fn advertised_trackable_count_for_grid(&self) -> usize {
        self.feature
            .iter()
            .filter(|feature| {
                feature.advertised && feature.maturity.is_advertised() && feature.counts_in_coverage
            })
            .count()
    }

    /// Compliance percentage for BDD/compliance grids.
    pub fn compliance_percent_for_grid(&self) -> f32 {
        let trackable = self.trackable_feature_count_for_grid();
        if trackable == 0 {
            return 0.0;
        }
        let advertised = self.advertised_trackable_count_for_grid();
        (advertised as f64 / trackable as f64 * 100.0).round() as f32
    }

    /// Compliance percentage calculated as advertised(trackable) / trackable.
    pub fn compliance_percent(&self) -> f32 {
        let trackable = self.trackable_feature_count();
        if trackable == 0 {
            return 0.0;
        }
        let advertised = self.advertised_trackable_count();
        (advertised as f64 / trackable as f64 * 100.0).round() as f32
    }

    /// Per-area statistics useful for documentation and reporting.
    pub fn area_statistics(&self) -> BTreeMap<String, AreaStats> {
        let mut stats: BTreeMap<String, AreaStats> = BTreeMap::new();

        for feature in &self.feature {
            let entry = stats.entry(feature.area.clone()).or_default();
            entry.total += 1;
            if feature.advertised {
                entry.advertised += 1;
            }

            match feature.maturity {
                Maturity::Ga => entry.ga += 1,
                Maturity::Production => entry.production += 1,
                Maturity::Preview => entry.preview += 1,
                Maturity::Experimental => entry.experimental += 1,
                Maturity::Planned => entry.planned += 1,
            }
        }

        stats
    }

    /// Validate constraints not captured by serde parsing alone.
    pub fn validate(&self) -> Result<(), CatalogError> {
        let mut seen = BTreeSet::new();
        let mut issues = Vec::new();

        for feature in &self.feature {
            if feature.id.trim().is_empty() {
                issues.push("feature id must not be empty".to_string());
                continue;
            }
            if !seen.insert(&feature.id) {
                issues.push(format!("duplicate feature id: {}", feature.id));
            }
        }

        if issues.is_empty() { Ok(()) } else { Err(CatalogError::Validation(issues.join(", "))) }
    }
}

/// Aggregate area-level information.
#[derive(Debug, Default, Clone, Copy)]
pub struct AreaStats {
    /// Total number of rows in the area.
    pub total: usize,
    /// Advertised row count in the area.
    pub advertised: usize,
    /// Experimental count.
    pub experimental: usize,
    /// Preview count.
    pub preview: usize,
    /// GA count.
    pub ga: usize,
    /// Production count.
    pub production: usize,
    /// Planned count.
    pub planned: usize,
}

impl AreaStats {
    /// Number of rows eligible for trackability.
    pub const fn trackable(&self) -> usize {
        self.total - self.planned
    }

    /// Advertised ratio in percent for this area.
    pub fn coverage_percent(&self) -> u32 {
        if self.total == 0 {
            return 0;
        }
        ((self.advertised as f64 / self.total as f64) * 100.0).round() as u32
    }

    /// Advertised ratio for trackable features.
    pub fn trackable_coverage_percent(&self) -> u32 {
        let trackable = self.trackable();
        if trackable == 0 {
            return 0;
        }
        ((self.advertised as f64 / trackable as f64) * 100.0).round() as u32
    }
}

/// Error type used by catalog operations.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// Missing catalog source file on the expected paths.
    #[error("features catalog not found for manifest dir: {0}")]
    MissingSource(PathBuf),

    /// I/O failure while reading the catalog source.
    #[error("failed to read features catalog: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parser error.
    #[error("failed to parse features catalog: {0}")]
    Parse(#[from] toml::de::Error),

    /// Validation failure after deserialization.
    #[error("invalid features catalog: {0}")]
    Validation(String),
}

/// Source selection detail for generated outputs and traceability.
#[derive(Debug, Clone)]
pub struct CatalogSource {
    /// Resolved source path.
    pub path: PathBuf,
    /// Selected source type.
    pub kind: CatalogSourceKind,
}

impl CatalogSource {
    /// Source tag emitted into generated modules.
    pub const fn comment(&self) -> &'static str {
        match self.kind {
            CatalogSourceKind::Override => "// source: FEATURES_TOML_OVERRIDE\n",
            CatalogSourceKind::Workspace => "// source: features.toml\n",
            CatalogSourceKind::Vendored => "// source: features_sot.toml\n",
        }
    }
}

/// Which catalog source path was selected.
#[derive(Debug, Clone, Copy)]
pub enum CatalogSourceKind {
    /// Path came from `FEATURES_TOML_OVERRIDE`.
    Override,
    /// Path came from workspace `features.toml`.
    Workspace,
    /// Path came from crate-local `features_sot.toml`.
    Vendored,
}

/// Resolve catalog path using workspace-first lookup and override support.
pub fn resolve_catalog_source(manifest_dir: &Path) -> Result<CatalogSource, CatalogError> {
    if let Ok(override_path) = env::var("FEATURES_TOML_OVERRIDE") {
        let override_path = PathBuf::from(override_path);
        if override_path.exists() {
            return Ok(CatalogSource { path: override_path, kind: CatalogSourceKind::Override });
        }
    }

    let local_workspace_candidate = manifest_dir.join("features.toml");
    if local_workspace_candidate.exists() {
        return Ok(CatalogSource {
            path: local_workspace_candidate,
            kind: CatalogSourceKind::Workspace,
        });
    }

    let parent_workspace = manifest_dir.parent().and_then(Path::parent).and_then(|p| {
        let path = p.join("features.toml");
        path.exists().then_some(path)
    });
    if let Some(path) = parent_workspace {
        return Ok(CatalogSource { path, kind: CatalogSourceKind::Workspace });
    }

    let vendored = manifest_dir.join("features_sot.toml");
    if vendored.exists() {
        return Ok(CatalogSource { path: vendored, kind: CatalogSourceKind::Vendored });
    }

    Err(CatalogError::MissingSource(manifest_dir.to_path_buf()))
}

/// Load and validate catalog from an explicit path.
pub fn read_catalog(path: &Path) -> Result<Catalog, CatalogError> {
    let content = fs::read_to_string(path)?;
    let catalog: Catalog = toml::from_str(&content)?;
    catalog.validate()?;
    Ok(catalog)
}

/// Load and validate catalog using workspace-style resolution.
pub fn load_catalog_for_build(
    manifest_dir: &Path,
) -> Result<(Catalog, CatalogSource), CatalogError> {
    let source = resolve_catalog_source(manifest_dir)?;
    let catalog = read_catalog(&source.path)?;
    Ok((catalog, source))
}

/// Render `features.rs`-compatible LSP runtime module source.
pub fn render_lsp_feature_catalog_module(catalog: &Catalog, source_comment: &str) -> String {
    let mut sorted = catalog.feature.clone();
    sorted.sort_by(|a, b| a.area.cmp(&b.area).then_with(|| a.id.cmp(&b.id)));

    let advertised = catalog.advertised_feature_ids();

    let mut code = String::new();
    code.push_str("// @generated by build.rs; DO NOT EDIT.\n");
    code.push_str(source_comment);
    code.push('\n');

    code.push_str("/// Current parser version extracted from features.toml metadata\n");
    code.push_str(&format!("pub const VERSION: &str = {:?};\n", catalog.meta.version));
    code.push_str("/// LSP protocol version supported by this parser implementation\n");
    code.push_str(&format!("pub const LSP_VERSION: &str = {:?};\n", catalog.meta.lsp_version));
    code.push_str("/// Compliance percentage of advertised GA features vs trackable features\n");
    code.push_str(&format!(
        "pub const COMPLIANCE_PERCENT: f32 = {:.2};\n\n",
        catalog.compliance_percent()
    ));

    code.push_str(
        "/// Represents a single LSP feature with its metadata and implementation status\n",
    );
    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str("pub struct Feature {\n");
    code.push_str("    /// Unique identifier for this feature\n");
    code.push_str("    pub id: &'static str,\n");
    code.push_str("    /// LSP specification reference\n");
    code.push_str("    pub spec: &'static str,\n");
    code.push_str("    /// Functional area for this feature\n");
    code.push_str("    pub area: &'static str,\n");
    code.push_str(
        "    /// Maturity level (`experimental`, `preview`, `ga`, `planned`, `production`)\n",
    );
    code.push_str("    pub maturity: &'static str,\n");
    code.push_str("    /// Advertised feature flag\n");
    code.push_str("    pub advertised: bool,\n");
    code.push_str("    /// Human-readable description\n");
    code.push_str("    pub description: &'static str,\n");
    code.push_str("    /// Include this feature in coverage / compliance accounting\n");
    code.push_str("    pub counts_in_coverage: bool,\n");
    code.push_str("    /// Test cases validating the feature\n");
    code.push_str("    pub tests: &'static [&'static str],\n");
    code.push_str("}\n\n");

    code.push_str(
        "/// Comprehensive catalog of all LSP features with their implementation status\n",
    );
    code.push_str("pub const ALL_FEATURES: &[Feature] = &[\n");
    for feature in sorted {
        code.push_str("    Feature {\n");
        code.push_str(&format!("        id: {:?},\n", feature.id));
        code.push_str(&format!("        spec: {:?},\n", feature.spec));
        code.push_str(&format!("        area: {:?},\n", feature.area));
        code.push_str(&format!("        maturity: {:?},\n", feature.maturity.label()));
        code.push_str(&format!("        advertised: {},\n", feature.advertised));
        code.push_str(&format!("        description: {:?},\n", feature.description));
        code.push_str(&format!("        counts_in_coverage: {},\n", feature.counts_in_coverage));
        code.push_str(&format!("        tests: &{:?},\n", feature.tests));
        code.push_str("    },\n");
    }
    code.push_str("];\n\n");

    code.push_str("/// Advertised feature IDs (GA/production and `advertised = true`).\n");
    code.push_str("pub const ADVERTISED_LSP_FEATURES: &[&str] = &[\n");
    for id in &advertised {
        code.push_str(&format!("    {:?},\n", id));
    }
    code.push_str("];\n\n");

    code.push_str("/// Returns advertised feature IDs (GA/production and `advertised = true`).\n");
    code.push_str("pub fn advertised_features() -> &'static [&'static str] {\n");
    code.push_str("    ADVERTISED_LSP_FEATURES\n");
    code.push_str("}\n\n");

    code.push_str("/// Checks whether a feature is currently advertised.\n");
    code.push_str("pub fn has_feature(id: &str) -> bool {\n");
    code.push_str("    ADVERTISED_LSP_FEATURES.contains(&id)\n");
    code.push_str("}\n\n");

    code.push_str("/// Returns the current LSP compliance percentage as a float.\n");
    code.push_str("pub fn compliance_percent() -> f32 {\n");
    code.push_str("    COMPLIANCE_PERCENT\n");
    code.push_str("}\n");

    code
}

/// Render DAP runtime module source.
pub fn render_dap_feature_catalog_module(ids: &[&str]) -> String {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut code = String::new();
    code.push_str("// @generated by build.rs; DO NOT EDIT.\n\n");
    code.push_str("pub const ADVERTISED_DAP_FEATURES: &[&str] = &[\n");
    for id in &sorted {
        code.push_str(&format!("    {:?},\n", id));
    }
    code.push_str("];\n\n");
    code.push_str("pub fn advertised_features() -> &'static [&'static str] {\n");
    code.push_str("    ADVERTISED_DAP_FEATURES\n");
    code.push_str("}\n\n");
    code.push_str("pub fn has_feature(id: &str) -> bool {\n");
    code.push_str("    ADVERTISED_DAP_FEATURES.contains(&id)\n");
    code.push_str("}\n");
    code
}

/// Render fallback DAP catalog for offline or error cases.
pub fn render_dap_fallback_module(default_features: &[&str]) -> String {
    render_dap_feature_catalog_module(default_features)
}

/// Minimal fallback module for build failures in `perl-lsp`.
pub fn render_lsp_fallback_module() -> String {
    let mut code = String::new();
    code.push_str("// Auto-generated minimal catalog - features.toml not found\n\n");
    code.push_str("pub struct Feature {\n");
    code.push_str("    pub id: &'static str,\n");
    code.push_str("    pub spec: &'static str,\n");
    code.push_str("    pub area: &'static str,\n");
    code.push_str("    pub maturity: &'static str,\n");
    code.push_str("    pub advertised: bool,\n");
    code.push_str("    pub description: &'static str,\n");
    code.push_str("    pub counts_in_coverage: bool,\n");
    code.push_str("    pub tests: &'static [&'static str],\n");
    code.push_str("}\n");
    code.push_str("pub const VERSION: &str = \"0.10.0\";\n");
    code.push_str("pub const LSP_VERSION: &str = \"3.18\";\n");
    code.push_str("pub const COMPLIANCE_PERCENT: f32 = 0.0;\n");
    code.push_str("pub const ALL_FEATURES: &[Feature] = &[];\n");
    code.push_str("pub const ADVERTISED_LSP_FEATURES: &[&str] = &[];\n");
    code.push_str(
        "pub fn advertised_features() -> &'static [&'static str] { ADVERTISED_LSP_FEATURES }\n",
    );
    code.push_str("pub fn has_feature(_id: &str) -> bool { false }\n");
    code.push_str("pub fn compliance_percent() -> f32 { 0.0 }\n");
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::{must, must_some};
    use tempfile::TempDir;

    fn sample_catalog() -> Catalog {
        Catalog {
            meta: Meta {
                version: "0.42.0".to_string(),
                lsp_version: "3.18".to_string(),
                compliance_percent: None,
            },
            feature: vec![
                Feature {
                    id: "lsp.completion".to_string(),
                    spec: "LSP 3.18".to_string(),
                    area: "text_document".to_string(),
                    maturity: Maturity::Ga,
                    advertised: true,
                    tests: vec!["crates/perl-lsp-rs/tests/completion.rs".to_string()],
                    counts_in_coverage: true,
                    description: "Completion support".to_string(),
                },
                Feature {
                    id: "lsp.semanticTokens".to_string(),
                    spec: "LSP 3.18".to_string(),
                    area: "text_document".to_string(),
                    maturity: Maturity::Preview,
                    advertised: true,
                    tests: vec!["crates/perl-lsp-rs/tests/semantic_tokens.rs".to_string()],
                    counts_in_coverage: true,
                    description: "Semantic token support".to_string(),
                },
                Feature {
                    id: "lsp.codeAction".to_string(),
                    spec: "LSP 3.18".to_string(),
                    area: "workspace".to_string(),
                    maturity: Maturity::Planned,
                    advertised: true,
                    tests: vec![],
                    counts_in_coverage: false,
                    description: "Code actions".to_string(),
                },
                Feature {
                    id: "lsp.references".to_string(),
                    spec: "LSP 3.18".to_string(),
                    area: "workspace".to_string(),
                    maturity: Maturity::Production,
                    advertised: true,
                    tests: vec!["crates/perl-lsp-rs/tests/references.rs".to_string()],
                    counts_in_coverage: true,
                    description: "References".to_string(),
                },
            ],
        }
    }

    #[test]
    fn advertised_ids_are_sorted_and_filter_preview_and_planned() {
        let catalog = sample_catalog();
        assert_eq!(catalog.advertised_feature_ids(), vec!["lsp.completion", "lsp.references"]);
    }

    #[test]
    fn compliance_math_uses_trackable_features_only() {
        let catalog = sample_catalog();
        assert_eq!(catalog.trackable_feature_count(), 3);
        assert_eq!(catalog.advertised_trackable_count(), 2);
        assert_eq!(catalog.compliance_percent(), 67.0);
        assert_eq!(catalog.trackable_feature_count_for_grid(), 3);
        assert_eq!(catalog.advertised_trackable_count_for_grid(), 2);
        assert_eq!(catalog.compliance_percent_for_grid(), 67.0);
    }

    #[test]
    fn area_stats_include_maturity_breakdown() {
        let catalog = sample_catalog();
        let stats = catalog.area_statistics();

        let text_doc = must_some(stats.get("text_document"));
        assert_eq!(text_doc.total, 2);
        assert_eq!(text_doc.advertised, 2);
        assert_eq!(text_doc.ga, 1);
        assert_eq!(text_doc.preview, 1);
        assert_eq!(text_doc.production, 0);
        assert_eq!(text_doc.trackable_coverage_percent(), 100);

        let workspace = must_some(stats.get("workspace"));
        assert_eq!(workspace.total, 2);
        assert_eq!(workspace.production, 1);
        assert_eq!(workspace.planned, 1);
        assert_eq!(workspace.trackable(), 1);
        assert_eq!(workspace.trackable_coverage_percent(), 200);
    }

    #[test]
    fn validation_rejects_duplicate_feature_ids() {
        let mut catalog = sample_catalog();
        catalog.feature.push(Feature {
            id: "lsp.completion".to_string(),
            spec: "LSP 3.18".to_string(),
            area: "text_document".to_string(),
            maturity: Maturity::Ga,
            advertised: true,
            tests: vec![],
            counts_in_coverage: true,
            description: "duplicate row".to_string(),
        });

        let err = catalog.validate().expect_err("duplicate id must fail validation");
        let message = err.to_string();
        assert!(message.contains("duplicate feature id: lsp.completion"));
    }

    #[test]
    fn resolve_catalog_source_prefers_workspace_then_vendored() {
        let temp = must(TempDir::new());
        let manifest_dir = temp.path().join("crates/perl-lsp-rs-core");
        must(std::fs::create_dir_all(&manifest_dir));

        let parent_workspace = temp.path().join("features.toml");
        must(std::fs::write(&parent_workspace, "[meta]\nversion='0.1.0'\nlsp_version='3.18'\n"));
        let source = must(resolve_catalog_source(&manifest_dir));
        assert!(matches!(source.kind, CatalogSourceKind::Workspace));
        assert_eq!(source.path, parent_workspace);

        must(std::fs::remove_file(temp.path().join("features.toml")));
        let vendored = manifest_dir.join("features_sot.toml");
        must(std::fs::write(&vendored, "[meta]\nversion='0.1.0'\nlsp_version='3.18'\n"));
        let source = must(resolve_catalog_source(&manifest_dir));
        assert!(matches!(source.kind, CatalogSourceKind::Vendored));
        assert_eq!(source.path, vendored);
    }

    #[test]
    fn render_lsp_module_sorts_features_and_emits_expected_constants() {
        let catalog = sample_catalog();
        let rendered = render_lsp_feature_catalog_module(&catalog, "// source: test\n");

        assert!(rendered.contains("pub const VERSION: &str = \"0.42.0\";"));
        assert!(rendered.contains("pub const LSP_VERSION: &str = \"3.18\";"));
        assert!(rendered.contains("pub const COMPLIANCE_PERCENT: f32 = 67.00;"));
        assert!(
            rendered.contains("pub const ADVERTISED_LSP_FEATURES: &[&str] = &[\n    \"lsp.completion\",\n    \"lsp.references\",\n];")
        );

        let code_action_idx = must_some(rendered.find("id: \"lsp.codeAction\""));
        let completion_idx = must_some(rendered.find("id: \"lsp.completion\""));
        let references_idx = must_some(rendered.find("id: \"lsp.references\""));
        let semantic_idx = must_some(rendered.find("id: \"lsp.semanticTokens\""));
        assert!(completion_idx < semantic_idx);
        assert!(semantic_idx < code_action_idx);
        assert!(code_action_idx < references_idx);
    }
}
