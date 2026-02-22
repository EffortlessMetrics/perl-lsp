use color_eyre::eyre::{Context, Result, eyre};
use perl_feature_catalog::{Catalog, Maturity};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

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

fn load_features() -> Result<Catalog> {
    let manifest_dir = env::current_dir().context("Failed to get current working directory")?;
    let (catalog, _) = perl_feature_catalog::load_catalog_for_build(&manifest_dir)
        .context("Failed to load features catalog from features.toml")?;
    Ok(catalog)
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

    // Placeholder for future fenced section replacement
    let _ = table;

    // For now, save the updated content
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
    let test_dir = Path::new("crates/perl-parser/tests");
    for feature in catalog.features() {
        if feature.advertised && !feature.tests.is_empty() {
            for test in &feature.tests {
                let test_file = test_dir.join(test);
                if !test_file.exists() {
                    warnings.push(format!("Test file not found for {}: {}", feature.id, test));
                }
            }
        }
    }

    // Check advertised feature IDs against the LSP snapshot.
    let snapshot_path =
        test_dir.join("snapshots/lsp_features_snapshot_test__advertised_vs_caps.snap");
    if snapshot_path.exists() {
        match fs::read_to_string(&snapshot_path) {
            Ok(content) => {
                if let Some(yaml_start) = content.find("---\n") {
                    let yaml_content = &content[yaml_start + 4..];
                    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml_content) {
                        Ok(yaml) => {
                            let catalog_advertised: BTreeSet<String> = catalog
                                .advertised_feature_ids()
                                .into_iter()
                                .map(String::from)
                                .collect();

                            if let Some(caps) = yaml.get("caps").and_then(|v| v.as_sequence()) {
                                let caps_set: BTreeSet<String> = caps
                                    .iter()
                                    .filter_map(|value| value.as_str().map(String::from))
                                    .collect();
                                let missing_in_caps =
                                    catalog_advertised.difference(&caps_set).collect::<Vec<_>>();
                                let extra_in_caps =
                                    caps_set.difference(&catalog_advertised).collect::<Vec<_>>();

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

                                if missing_in_caps.is_empty() && extra_in_caps.is_empty() {
                                    println!("📋 Snapshot comparison: ✅ Perfect match");
                                }
                            } else {
                                warnings
                                    .push("Could not find 'caps' array in snapshot".to_string());
                            }
                        }
                        Err(error) => {
                            warnings.push(format!("Failed to parse snapshot YAML: {error}"));
                        }
                    }
                } else {
                    warnings.push("Snapshot file doesn't contain valid YAML section".to_string());
                }
            }
            Err(error) => warnings.push(format!("Failed to read snapshot file: {error}")),
        }
    } else {
        warnings.push(
            "Snapshot file not found - run 'cargo test -p perl-parser --test lsp_features_snapshot_test' to generate"
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
    let overall = if total == 0 { 0 } else { advertised * 100 / total };
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
