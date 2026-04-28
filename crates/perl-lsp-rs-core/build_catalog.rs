// Build-time feature catalog helper, included via `include!()` in build.rs.
// This is a copy of the catalog logic extracted from perl-feature-catalog.
// It exists here because build.rs is a separate compilation unit and cannot
// reference `perl_lsp_rs_core::feature_catalog` at build time.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Default DAP feature identifiers emitted when catalog processing fails.
pub const DEFAULT_DAP_FEATURES: &[&str] =
    &["dap.breakpoints.basic", "dap.core", "dap.inline_values"];

/// Feature maturity state.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Maturity {
    Experimental,
    Preview,
    Ga,
    Planned,
    Production,
}

impl Maturity {
    pub const fn is_advertised(self) -> bool {
        matches!(self, Self::Ga | Self::Production)
    }

    pub const fn is_trackable(self) -> bool {
        !matches!(self, Self::Planned)
    }

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Meta {
    pub version: String,
    pub lsp_version: String,
    #[serde(default)]
    pub compliance_percent: Option<u32>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Feature {
    pub id: String,
    #[serde(default)]
    pub spec: String,
    #[serde(default)]
    pub area: String,
    pub maturity: Maturity,
    #[serde(default)]
    pub advertised: bool,
    #[serde(default)]
    pub tests: Vec<String>,
    #[serde(default = "default_counts_in_coverage")]
    pub counts_in_coverage: bool,
    #[serde(default)]
    pub description: String,
}

fn default_counts_in_coverage() -> bool {
    true
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Catalog {
    pub meta: Meta,
    pub feature: Vec<Feature>,
}

impl Catalog {
    pub fn advertised_feature_ids(&self) -> Vec<&str> {
        let mut ids = self
            .feature
            .iter()
            .filter(|f| f.advertised && f.maturity.is_advertised())
            .map(|f| f.id.as_str())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids
    }

    pub fn trackable_feature_count(&self) -> usize {
        self.feature.iter().filter(|f| f.maturity.is_trackable()).count()
    }

    pub fn advertised_trackable_count(&self) -> usize {
        self.feature
            .iter()
            .filter(|f| f.advertised && f.maturity.is_advertised())
            .count()
    }

    pub fn compliance_percent(&self) -> f32 {
        let trackable = self.trackable_feature_count();
        if trackable == 0 {
            return 0.0;
        }
        let advertised = self.advertised_trackable_count();
        (advertised as f64 / trackable as f64 * 100.0).round() as f32
    }

    pub fn validate(&self) -> Result<(), String> {
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
        if issues.is_empty() { Ok(()) } else { Err(issues.join(", ")) }
    }
}

#[derive(Debug, Clone)]
pub struct CatalogSource {
    pub path: PathBuf,
    pub kind: CatalogSourceKind,
}

impl CatalogSource {
    pub const fn comment(&self) -> &'static str {
        match self.kind {
            CatalogSourceKind::Override => "// source: FEATURES_TOML_OVERRIDE\n",
            CatalogSourceKind::Workspace => "// source: features.toml\n",
            CatalogSourceKind::Vendored => "// source: features_sot.toml\n",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CatalogSourceKind {
    Override,
    Workspace,
    Vendored,
}

pub fn resolve_catalog_source(manifest_dir: &Path) -> Result<CatalogSource, String> {
    if let Ok(override_path) = env::var("FEATURES_TOML_OVERRIDE") {
        let override_path = PathBuf::from(override_path);
        if override_path.exists() {
            return Ok(CatalogSource { path: override_path, kind: CatalogSourceKind::Override });
        }
    }

    let local = manifest_dir.join("features.toml");
    if local.exists() {
        return Ok(CatalogSource { path: local, kind: CatalogSourceKind::Workspace });
    }

    let parent = manifest_dir.parent().and_then(Path::parent).and_then(|p| {
        let path = p.join("features.toml");
        path.exists().then_some(path)
    });
    if let Some(path) = parent {
        return Ok(CatalogSource { path, kind: CatalogSourceKind::Workspace });
    }

    let vendored = manifest_dir.join("features_sot.toml");
    if vendored.exists() {
        return Ok(CatalogSource { path: vendored, kind: CatalogSourceKind::Vendored });
    }

    Err(format!("features catalog not found for manifest dir: {}", manifest_dir.display()))
}

pub fn read_catalog(path: &Path) -> Result<Catalog, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read features catalog: {e}"))?;
    let catalog: Catalog = toml::from_str(&content)
        .map_err(|e| format!("failed to parse features catalog: {e}"))?;
    catalog.validate()?;
    Ok(catalog)
}

pub fn load_catalog_for_build(manifest_dir: &Path) -> Result<(Catalog, CatalogSource), String> {
    let source = resolve_catalog_source(manifest_dir)?;
    let catalog = read_catalog(&source.path)?;
    Ok((catalog, source))
}

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
    code.push_str("    pub id: &'static str,\n");
    code.push_str("    pub spec: &'static str,\n");
    code.push_str("    pub area: &'static str,\n");
    code.push_str("    pub maturity: &'static str,\n");
    code.push_str("    pub advertised: bool,\n");
    code.push_str("    pub description: &'static str,\n");
    code.push_str("    pub counts_in_coverage: bool,\n");
    code.push_str("    pub tests: &'static [&'static str],\n");
    code.push_str("}\n\n");
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
    code.push_str("pub const ADVERTISED_LSP_FEATURES: &[&str] = &[\n");
    for id in &advertised {
        code.push_str(&format!("    {:?},\n", id));
    }
    code.push_str("];\n\n");
    code.push_str("pub fn advertised_features() -> &'static [&'static str] { ADVERTISED_LSP_FEATURES }\n\n");
    code.push_str("pub fn has_feature(id: &str) -> bool { ADVERTISED_LSP_FEATURES.contains(&id) }\n\n");
    code.push_str("pub fn compliance_percent() -> f32 { COMPLIANCE_PERCENT }\n");
    code
}

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
    code.push_str("pub fn advertised_features() -> &'static [&'static str] { ADVERTISED_DAP_FEATURES }\n\n");
    code.push_str("pub fn has_feature(id: &str) -> bool { ADVERTISED_DAP_FEATURES.contains(&id) }\n");
    code
}

pub fn render_dap_fallback_module(default_features: &[&str]) -> String {
    render_dap_feature_catalog_module(default_features)
}

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
    code.push_str("pub fn advertised_features() -> &'static [&'static str] { ADVERTISED_LSP_FEATURES }\n");
    code.push_str("pub fn has_feature(_id: &str) -> bool { false }\n");
    code.push_str("pub fn compliance_percent() -> f32 { 0.0 }\n");
    code
}
