use color_eyre::eyre::{Context, Result, bail, eyre};
use perl_feature_catalog::{Catalog, Maturity};
use perl_lsp_rs_core::governance::{FeatureProfile, catalog_advertised_feature_ids};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Public API functions called from main.rs
pub fn sync_docs() -> Result<()> {
    sync_docs_impl()
}

pub fn verify() -> Result<()> {
    verify_features()
}

pub fn report() -> Result<()> {
    generate_report()
}

pub fn invariants() -> Result<()> {
    check_invariants()
}

fn load_features() -> Result<Catalog> {
    let manifest_dir = env::current_dir().context("Failed to get current working directory")?;
    let (catalog, _) = perl_feature_catalog::load_catalog_for_build(&manifest_dir)
        .context("Failed to load features catalog from features.toml")?;
    Ok(catalog)
}

fn repo_relative_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() { path.to_path_buf() } else { PathBuf::from(path) }
}

fn lsp_feature_snapshot_path() -> PathBuf {
    repo_relative_path(
        "crates/perl-lsp/tests/snapshots/lsp_features_snapshot_test__advertised_vs_caps.snap",
    )
}

fn snapshot_comparable_feature_ids() -> BTreeSet<String> {
    let mut ids: BTreeSet<String> =
        catalog_advertised_feature_ids(FeatureProfile::All).into_iter().map(String::from).collect();

    // The capability snapshot intentionally excludes formatting because
    // initialize capabilities depend on runtime perltidy availability.
    ids.remove("lsp.formatting");
    ids.remove("lsp.range_formatting");
    ids.remove("lsp.ranges_formatting");

    // These features are advertised and tested, but they do not currently round-trip
    // through the typed `ServerCapabilities` snapshot used here:
    // - `lsp.type_hierarchy` is injected as a raw top-level JSON field because
    //   `lsp-types` 0.97 lacks the field.
    // - `lsp.inline_completion` is served via proposed/experimental plumbing rather
    //   than a stable `ServerCapabilities` field in this stack.
    ids.remove("lsp.type_hierarchy");
    ids.remove("lsp.inline_completion");

    ids
}

fn check_invariants() -> Result<()> {
    println!("🔍 Checking features.toml invariants...");

    let catalog = load_features()?;
    let mut violations = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for feature in catalog.features() {
        if !seen_ids.insert(feature.id.clone()) {
            violations.push(format!("DUPLICATE_ID: {:?} appears more than once", feature.id));
        }

        if feature.advertised
            && feature.maturity == Maturity::Ga
            && feature.tests.is_empty()
            && feature.counts_in_coverage
        {
            violations.push(format!(
                "UNTESTED_GA: {:?} is advertised+GA but has no tests. Either add tests or set counts_in_coverage=false (if it's protocol plumbing).",
                feature.id
            ));
        }
    }

    if !violations.is_empty() {
        let violations_count = violations.len();
        println!("FEATURE INVARIANT VIOLATIONS:");
        println!("{}", "=".repeat(50));
        for violation in violations {
            println!("  - {violation}");
        }
        println!("{}", "=".repeat(50));
        println!("{violations_count} violation(s) found.");
        bail!("feature invariants check failed");
    }

    let total = catalog.features().len();
    let ga_advertised =
        catalog.features().iter().filter(|f| f.advertised && f.maturity == Maturity::Ga).count();
    let headline_features = catalog
        .features()
        .iter()
        .filter(|f| f.advertised && f.maturity == Maturity::Ga && f.counts_in_coverage)
        .count();

    println!(
        "Feature invariants OK: {total} features, {ga_advertised} GA+advertised, {headline_features} in headline metric"
    );
    Ok(())
}

fn sync_docs_impl() -> Result<()> {
    println!("📝 Syncing documentation from features.toml...");

    let catalog = load_features()?;
    let area_stats = catalog.area_statistics();

    // Update ROADMAP.md
    update_roadmap(&catalog, &area_stats)?;

    // Update LSP_ACTUAL_STATUS.md
    update_lsp_status(&catalog)?;

    println!("✅ Documentation synced successfully!");
    Ok(())
}

fn update_roadmap(
    catalog: &Catalog,
    area_stats: &BTreeMap<String, perl_feature_catalog::AreaStats>,
) -> Result<()> {
    let roadmap_path = Path::new("ROADMAP.md");
    let mut content = fs::read_to_string(roadmap_path)?;

    // Ensure fence markers exist
    ensure_fence(&content, "COMPLIANCE_TABLE")?;

    // Calculate overall compliance
    let total: usize = area_stats.values().map(|s| s.total).sum();
    let advertised: usize = area_stats.values().map(|s| s.advertised).sum();
    let compliance =
        if total == 0 { 0 } else { (advertised as f64 / total as f64 * 100.0).round() as u32 };

    // Update compliance percentage in header
    let new_text = format!("partial LSP 3.18 compliance (~{}%)", compliance);
    let old_pattern = r"partial LSP 3.18 compliance \(~\d+%\)";
    content = regex::Regex::new(old_pattern)?.replace_all(&content, new_text.as_str()).to_string();

    // Update the compliance table
    let mut table = String::new();
    table.push_str("| Area | Implemented | Total | Coverage |\n");
    table.push_str("|------|-------------|-------|----------|\n");

    for (area, stats) in area_stats {
        table.push_str(&format!(
            "| {} | {} | {} | {}% |\n",
            area.replace('_', " "),
            stats.advertised,
            stats.total,
            stats.coverage_percent()
        ));
    }

    // Inject the compliance table into the fenced section
    content = replace_fence(&content, "COMPLIANCE_TABLE", &table)?;

    fs::write(roadmap_path, content)?;

    // Keep this side-effect so the BDD-style progress checks can fail fast when catalog
    // fields are missing or out of date.
    let version = catalog.meta.version.clone();
    if version.is_empty() {
        return Err(eyre!("Catalog version is missing"));
    }

    Ok(())
}

fn update_lsp_status(catalog: &Catalog) -> Result<()> {
    let status_path = Path::new("crates/perl-parser/LSP_ACTUAL_STATUS.md");

    // Check if file exists and has fence markers (for future use with fenced sections)
    if status_path.exists() {
        let existing = fs::read_to_string(status_path)?;
        if existing.contains("<!-- BEGIN:") && existing.contains("<!-- END:") {
            println!("Note: Fenced sections detected but full regeneration in use");
        }
    }

    let mut by_area: BTreeMap<String, Vec<&perl_feature_catalog::Feature>> = BTreeMap::new();
    for feature in catalog.features() {
        by_area.entry(feature.area.clone()).or_default().push(feature);
    }

    let mut content = String::new();
    content.push_str("# LSP Feature Status\n\n");
    content.push_str("Auto-generated from `features.toml` - DO NOT EDIT\n\n");
    content.push_str(&format!(
        "Version: {} | LSP: {}\n\n",
        catalog.meta.version, catalog.meta.lsp_version
    ));

    for (area, features) in by_area {
        content.push_str(&format!("## {}\n\n", area.replace('_', " ")));
        content.push_str("| Feature | Spec | Status | Description |\n");
        content.push_str("|---------|------|--------|-------------|\n");

        for feature in features {
            let status = match (feature.maturity, feature.advertised) {
                (Maturity::Ga | Maturity::Production, true) => "✅ Complete",
                (Maturity::Preview, true) => "🔧 Preview",
                (Maturity::Experimental, _) => "⚠️ Experimental",
                _ => "❌ Not Implemented",
            };

            content.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                feature.id.replace("lsp.", ""),
                feature.spec,
                status,
                feature.description
            ));
        }
        content.push('\n');
    }

    fs::write(status_path, content)?;
    Ok(())
}

/// Ensure fence markers exist in document
fn ensure_fence(content: &str, tag: &str) -> Result<()> {
    let begin_marker = format!("<!-- BEGIN: {tag} -->");
    let end_marker = format!("<!-- END: {tag} -->");

    if !content.contains(&begin_marker) || !content.contains(&end_marker) {
        return Err(eyre!(
            "Missing documentation fence for {} - expected both '{}' and '{}'",
            tag,
            begin_marker,
            end_marker
        ));
    }
    Ok(())
}

/// Replace content between `<!-- BEGIN: TAG -->` and `<!-- END: TAG -->` markers.
fn replace_fence(content: &str, tag: &str, new_body: &str) -> Result<String> {
    let begin_marker = format!("<!-- BEGIN: {tag} -->");
    let end_marker = format!("<!-- END: {tag} -->");

    let begin_pos =
        content.find(&begin_marker).ok_or_else(|| eyre!("Missing begin marker for {tag}"))?;
    let end_pos = content.find(&end_marker).ok_or_else(|| eyre!("Missing end marker for {tag}"))?;

    let mut result = String::with_capacity(content.len());
    result.push_str(&content[..begin_pos]);
    result.push_str(&begin_marker);
    result.push('\n');
    result.push_str(new_body);
    result.push_str(&end_marker);
    result.push_str(&content[end_pos + end_marker.len()..]);
    Ok(result)
}

fn snapshot_caps_from_content(content: &str) -> Result<Option<BTreeSet<String>>> {
    let Some(yaml_start) = content.find("---\n") else {
        return Ok(None);
    };

    let after_first_doc = &content[yaml_start + 4..];
    let yaml_content = if let Some(second_doc_start) = after_first_doc.find("\n---\n") {
        &after_first_doc[second_doc_start + 5..]
    } else {
        after_first_doc
    };
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_content)?;
    let caps = yaml.get("caps").and_then(|value| value.as_sequence()).map(|caps| {
        caps.iter().filter_map(|value| value.as_str().map(String::from)).collect::<BTreeSet<_>>()
    });

    Ok(caps)
}

fn compare_snapshot_caps(
    catalog_advertised: &BTreeSet<String>,
    snapshot_caps: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let missing_in_caps = catalog_advertised.difference(snapshot_caps).collect::<Vec<_>>();
    let extra_in_caps = snapshot_caps.difference(catalog_advertised).collect::<Vec<_>>();

    if !missing_in_caps.is_empty() {
        errors.push(format!(
            "Features advertised in catalog but not in capabilities: {}",
            missing_in_caps.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }

    if !extra_in_caps.is_empty() {
        warnings.push(format!(
            "Features in capabilities but not advertised in catalog: {}",
            extra_in_caps.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }

    (warnings, errors)
}

fn verify_features() -> Result<()> {
    println!("🔍 Verifying features match capabilities...");

    let catalog = load_features()?;
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Check for duplicate IDs and basic validity.
    if let Err(error) = catalog.validate() {
        errors.push(error.to_string());
    }

    // Check that all advertised features have tests.
    for feature in catalog.features() {
        if feature.advertised && feature.tests.is_empty() {
            warnings.push(format!("Feature advertised without tests: {}", feature.id));
        }
    }

    // Check that advertised features have at least one backing test file.
    for feature in catalog.features() {
        if feature.advertised && !feature.tests.is_empty() {
            for test in &feature.tests {
                let test_file = repo_relative_path(test);
                if !test_file.exists() {
                    warnings.push(format!("Test file not found for {}: {}", feature.id, test));
                }
            }
        }
    }

    // Check advertised feature IDs against the LSP snapshot.
    let snapshot_path = lsp_feature_snapshot_path();
    if snapshot_path.exists() {
        match fs::read_to_string(&snapshot_path) {
            Ok(content) => match snapshot_caps_from_content(&content) {
                Ok(Some(snapshot_caps)) => {
                    let catalog_advertised = snapshot_comparable_feature_ids();
                    let (snapshot_warnings, snapshot_errors) =
                        compare_snapshot_caps(&catalog_advertised, &snapshot_caps);

                    if snapshot_warnings.is_empty() && snapshot_errors.is_empty() {
                        println!("📋 Snapshot comparison: ✅ Perfect match");
                    }

                    warnings.extend(snapshot_warnings);
                    errors.extend(snapshot_errors);
                }
                Ok(None) => {
                    warnings.push("Snapshot file doesn't contain valid YAML section".to_string());
                }
                Err(error) => warnings.push(format!("Failed to parse snapshot YAML: {error}")),
            },
            Err(error) => warnings.push(format!("Failed to read snapshot file: {error}")),
        }
    } else {
        warnings.push(
            "Snapshot file not found - run 'cargo test -p perl-lsp --test lsp_features_snapshot_test' to generate"
                .to_string(),
        );
    }

    // Verify compliance percentage matches ROADMAP documentation.
    let computed_compliance = catalog.compliance_percent() as u32;
    if let Ok(roadmap) = fs::read_to_string("ROADMAP.md") {
        let regex = regex::Regex::new(r"partial LSP 3\.18 compliance \(~(\d+)%\)")?;
        if let Some(cap) = regex.captures(&roadmap)
            && let Some(doc_percent) = cap.get(1).and_then(|m| m.as_str().parse::<u32>().ok())
            && doc_percent != computed_compliance
        {
            if env::var("CI_ALLOW_COMPLIANCE_DRIFT").is_err() {
                errors.push(format!(
                    "Compliance percentage drift detected: documented {}% vs computed {}% - run 'cargo xtask features sync-docs' to fix",
                    doc_percent, computed_compliance
                ));
            } else {
                warnings.push(format!(
                    "Compliance percentage mismatch (allowed): documented {}% vs computed {}%",
                    doc_percent, computed_compliance
                ));
            }
        }
    }

    let non_planned = catalog.trackable_feature_count();
    let advertised_ga_prod = catalog.advertised_trackable_count();
    println!(
        "📊 Computed compliance: {}% ({}/{} non-planned features)",
        computed_compliance, advertised_ga_prod, non_planned
    );

    if !errors.is_empty() {
        println!("❌ Errors found:");
        for error in &errors {
            println!("  - {}", error);
        }
        return Err(eyre!("Feature verification failed with {} errors", errors.len()));
    }

    if !warnings.is_empty() {
        println!("⚠️  Warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }

    println!("✅ Feature verification complete!");
    Ok(())
}

fn generate_report() -> Result<()> {
    println!("📊 Generating compliance report...");

    let catalog = load_features()?;
    let area_stats = catalog.area_statistics();

    let total = catalog.feature.len();
    let advertised = catalog.feature.iter().filter(|f| f.advertised).count();
    let ga = catalog
        .feature
        .iter()
        .filter(|f| matches!(f.maturity, Maturity::Ga | Maturity::Production) && f.advertised)
        .count();
    let preview = catalog
        .feature
        .iter()
        .filter(|f| matches!(f.maturity, Maturity::Preview) && f.advertised)
        .count();
    let experimental =
        catalog.feature.iter().filter(|f| matches!(f.maturity, Maturity::Experimental)).count();
    let planned =
        catalog.feature.iter().filter(|f| matches!(f.maturity, Maturity::Planned)).count();

    println!("\n=== LSP Compliance Report ===");
    println!("Version: {} | LSP: {}", catalog.meta.version, catalog.meta.lsp_version);
    let overall =
        if total == 0 { 0 } else { (advertised as f64 / total as f64 * 100.0).round() as usize };
    println!("\nOverall: {}/{} features ({}%)", advertised, total, overall);
    println!("\nBreakdown:");
    println!("  GA:           {} features", ga);
    println!("  Preview:      {} features", preview);
    println!("  Experimental: {} features", experimental);
    println!("  Planned:      {} features", planned);

    println!("\nBy Area:");
    for (area, stats) in area_stats {
        let coverage = if stats.total == 0 {
            0
        } else {
            (stats.advertised as f64 / stats.total as f64 * 100.0).round() as u32
        };
        println!(
            "  {:20} {}/{} ({}%)",
            area.replace('_', " "),
            stats.advertised,
            stats.total,
            coverage
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        compare_snapshot_caps, ensure_fence, lsp_feature_snapshot_path, replace_fence,
        repo_relative_path, snapshot_caps_from_content, snapshot_comparable_feature_ids,
    };
    use color_eyre::eyre::Result;
    use perl_tdd_support::must_err;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn snapshot_caps_from_content_extracts_caps_array() -> Result<()> {
        let content = "header\n---\ncaps:\n  - lsp.hover\n  - lsp.definition\n";
        let caps = snapshot_caps_from_content(content)?;

        assert_eq!(
            caps,
            Some(BTreeSet::from(["lsp.definition".to_string(), "lsp.hover".to_string()]))
        );
        Ok(())
    }

    #[test]
    fn snapshot_caps_from_content_returns_none_without_yaml_delimiter() -> Result<()> {
        let content = "caps:\n  - lsp.hover\n";
        assert_eq!(snapshot_caps_from_content(content)?, None);
        Ok(())
    }

    #[test]
    fn repo_relative_path_keeps_catalog_test_paths_rooted_at_repo() {
        let path = repo_relative_path("crates/perl-lsp/tests/lsp_completion_tests.rs");
        assert_eq!(path, PathBuf::from("crates/perl-lsp/tests/lsp_completion_tests.rs"));
    }

    #[test]
    fn lsp_feature_snapshot_path_points_to_lsp_snapshot() {
        assert_eq!(
            lsp_feature_snapshot_path(),
            PathBuf::from(
                "crates/perl-lsp/tests/snapshots/lsp_features_snapshot_test__advertised_vs_caps.snap"
            )
        );
    }

    #[test]
    fn snapshot_comparable_feature_ids_excludes_non_capability_rows() {
        let ids = snapshot_comparable_feature_ids();
        assert!(!ids.contains("dap.core"));
        assert!(!ids.contains("lsp.workspace_symbol_resolve"));
        assert!(!ids.contains("lsp.code_lens_refresh"));
        assert!(!ids.contains("lsp.formatting"));
        assert!(!ids.contains("lsp.type_hierarchy"));
        assert!(!ids.contains("lsp.inline_completion"));
        assert!(ids.contains("lsp.completion"));
    }

    #[test]
    fn snapshot_caps_from_content_returns_none_without_caps_key() -> Result<()> {
        let content = "header\n---\nprofiles:\n  - all\n";
        assert_eq!(snapshot_caps_from_content(content)?, None);
        Ok(())
    }

    #[test]
    fn snapshot_caps_from_content_handles_insta_two_doc_snapshot() -> Result<()> {
        let content = "\
---\n\
source: crates/perl-lsp/tests/lsp_features_snapshot_test.rs\n\
expression: \"&snapshot_data\"\n\
---\n\
caps:\n\
  - lsp.hover\n\
  - lsp.definition\n";

        let caps = snapshot_caps_from_content(content)?;
        assert_eq!(
            caps,
            Some(BTreeSet::from(["lsp.definition".to_string(), "lsp.hover".to_string()]))
        );
        Ok(())
    }

    #[test]
    fn compare_snapshot_caps_reports_missing_and_extra_features() {
        let catalog = BTreeSet::from(["lsp.hover".to_string(), "lsp.definition".to_string()]);
        let snapshot = BTreeSet::from(["lsp.hover".to_string(), "lsp.rename".to_string()]);

        let (warnings, errors) = compare_snapshot_caps(&catalog, &snapshot);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("lsp.definition"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("lsp.rename"));
    }

    #[test]
    fn compare_snapshot_caps_accepts_perfect_match() {
        let catalog = BTreeSet::from(["lsp.hover".to_string()]);
        let snapshot = BTreeSet::from(["lsp.hover".to_string()]);

        let (warnings, errors) = compare_snapshot_caps(&catalog, &snapshot);

        assert!(warnings.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn ensure_fence_requires_both_markers() {
        let content = "<!-- BEGIN: COMPLIANCE_TABLE -->\nbody\n";
        let error = must_err(ensure_fence(content, "COMPLIANCE_TABLE"));
        assert!(error.to_string().contains("Missing documentation fence"));
    }

    #[test]
    fn replace_fence_replaces_only_tagged_section() -> Result<()> {
        let content = "before\n<!-- BEGIN: COMPLIANCE_TABLE -->\nold\n<!-- END: COMPLIANCE_TABLE -->\nafter\n";
        let replaced = replace_fence(content, "COMPLIANCE_TABLE", "new\n")?;

        assert!(replaced.contains("before"));
        assert!(replaced.contains("after"));
        assert!(
            replaced
                .contains("<!-- BEGIN: COMPLIANCE_TABLE -->\nnew\n<!-- END: COMPLIANCE_TABLE -->")
        );
        assert!(!replaced.contains("\nold\n"));
        Ok(())
    }
}
