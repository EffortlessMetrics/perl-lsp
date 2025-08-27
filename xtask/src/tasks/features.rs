use color_eyre::eyre::{Context, Result, eyre};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct FeaturesCatalog {
    meta: Meta,
    feature: Vec<Feature>,
}

#[derive(Debug, serde::Deserialize)]
struct Meta {
    version: String,
    lsp_version: String,
    #[allow(dead_code)]
    compliance_percent: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
struct Feature {
    id: String,
    spec: String,
    area: String,
    maturity: String,
    advertised: bool,
    tests: Vec<String>,
    description: String,
}

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

fn load_features() -> Result<FeaturesCatalog> {
    let features_path = Path::new("features.toml");
    let content = fs::read_to_string(features_path).context("Failed to read features.toml")?;
    toml::from_str(&content).context("Failed to parse features.toml")
}

fn sync_docs_impl() -> Result<()> {
    println!("📝 Syncing documentation from features.toml...");

    let catalog = load_features()?;

    // Calculate compliance by area
    let mut area_stats: HashMap<String, (usize, usize)> = HashMap::new();
    for feature in &catalog.feature {
        let entry = area_stats.entry(feature.area.clone()).or_insert((0, 0));
        entry.1 += 1; // total
        if feature.advertised {
            entry.0 += 1; // advertised
        }
    }

    // Update ROADMAP.md
    update_roadmap(&catalog, &area_stats)?;

    // Update LSP_ACTUAL_STATUS.md
    update_lsp_status(&catalog, &area_stats)?;

    println!("✅ Documentation synced successfully!");
    Ok(())
}

fn update_roadmap(
    _catalog: &FeaturesCatalog,
    area_stats: &HashMap<String, (usize, usize)>,
) -> Result<()> {
    let roadmap_path = Path::new("ROADMAP.md");
    let mut content = fs::read_to_string(roadmap_path)?;

    // Ensure fence markers exist
    ensure_fence(&content, "COMPLIANCE_TABLE")?;

    // Calculate overall compliance
    let total: usize = area_stats.values().map(|(_, t)| t).sum();
    let advertised: usize = area_stats.values().map(|(a, _)| a).sum();
    let compliance = ((advertised as f64 / total as f64 * 100.0).round()) as u32;

    // Update compliance percentage in header
    let old_pattern = r"partial LSP 3.18 compliance \(~\d+%\)";
    let new_text = format!("partial LSP 3.18 compliance (~{}%)", compliance);
    content = regex::Regex::new(old_pattern)?.replace_all(&content, new_text.as_str()).to_string();

    // Update the compliance table
    let mut table = String::new();
    table.push_str("| Area | Implemented | Total | Coverage |\n");
    table.push_str("|------|-------------|-------|----------|\n");

    for (area, (impl_count, total_count)) in area_stats {
        let coverage = ((*impl_count as f64 / *total_count as f64 * 100.0).round()) as u32;
        table.push_str(&format!(
            "| {} | {} | {} | {}% |\n",
            area.replace('_', " "),
            impl_count,
            total_count,
            coverage
        ));
    }

    // We would need more complex logic to replace the exact table
    // For now, just save the updated content
    fs::write(roadmap_path, content)?;

    Ok(())
}

fn update_lsp_status(
    catalog: &FeaturesCatalog,
    _area_stats: &HashMap<String, (usize, usize)>,
) -> Result<()> {
    let status_path = Path::new("crates/perl-parser/LSP_ACTUAL_STATUS.md");

    // Check if file exists and has fence markers (for future use with fenced sections)
    if status_path.exists() {
        let existing = fs::read_to_string(status_path)?;
        // For now we regenerate the whole file, but check for future fenced sections
        if existing.contains("<!-- BEGIN:") && existing.contains("<!-- END:") {
            // Future: preserve fenced sections
            println!("Note: Fenced sections detected but full regeneration in use");
        }
    }

    let mut content = String::new();
    content.push_str("# LSP Feature Status\n\n");
    content.push_str("Auto-generated from `features.toml` - DO NOT EDIT\n\n");
    content.push_str(&format!(
        "Version: {} | LSP: {}\n\n",
        catalog.meta.version, catalog.meta.lsp_version
    ));

    // Group features by area
    let mut by_area: HashMap<String, Vec<&Feature>> = HashMap::new();
    for feature in &catalog.feature {
        by_area.entry(feature.area.clone()).or_default().push(feature);
    }

    // Generate sections for each area
    for (area, features) in by_area {
        content.push_str(&format!("## {}\n\n", area.replace('_', " ")));
        content.push_str("| Feature | Spec | Status | Description |\n");
        content.push_str("|---------|------|--------|-------------|\n");

        for feature in features {
            let status = match feature.maturity.as_str() {
                "ga" if feature.advertised => "✅ Complete",
                "preview" if feature.advertised => "🔧 Preview",
                "experimental" => "⚠️ Experimental",
                "planned" => "📋 Planned",
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
        content.push_str("\n");
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

    // Check for duplicate feature IDs
    let mut seen_ids = std::collections::HashSet::new();
    for feature in &catalog.feature {
        if !seen_ids.insert(&feature.id) {
            errors.push(format!("Duplicate feature ID: {}", feature.id));
        }
    }

    // Check for valid maturity values
    for feature in &catalog.feature {
        match feature.maturity.as_str() {
            "experimental" | "preview" | "ga" | "planned" | "production" => {}
            other => {
                errors.push(format!("Unknown maturity '{}' for feature {}", other, feature.id))
            }
        }
    }

    // Check that all advertised features have tests
    let mut missing_tests = Vec::new();
    for feature in &catalog.feature {
        if feature.advertised && feature.tests.is_empty() {
            missing_tests.push(&feature.id);
        }
    }

    if !missing_tests.is_empty() {
        let test_list: Vec<String> = missing_tests.iter().map(|s| s.to_string()).collect();
        warnings.push(format!("Features missing tests: {}", test_list.join(", ")));
    }

    // Check that advertised features exist in test directories
    let test_dir = Path::new("crates/perl-parser/tests");
    for feature in &catalog.feature {
        if feature.advertised && !feature.tests.is_empty() {
            for test in &feature.tests {
                let test_file = test_dir.join(format!("{}.rs", test));
                if !test_file.exists() {
                    warnings.push(format!("Test file not found for {}: {}", feature.id, test));
                }
            }
        }
    }

    // Check for unmapped feature IDs by parsing the snapshot
    let snapshot_path =
        test_dir.join("snapshots/lsp_features_snapshot_test__advertised_vs_caps.snap");
    if snapshot_path.exists() {
        match fs::read_to_string(&snapshot_path) {
            Ok(content) => {
                // Insta snapshots have a header, find the actual YAML content
                if let Some(yaml_start) = content.find("---\n") {
                    let yaml_content = &content[yaml_start + 4..];
                    match serde_yaml::from_str::<serde_yaml::Value>(yaml_content) {
                        Ok(yaml) => {
                            // Extract the advertised features from catalog
                            let catalog_advertised: std::collections::BTreeSet<String> = catalog
                                .feature
                                .iter()
                                .filter(|f| {
                                    f.advertised
                                        && (f.maturity == "ga" || f.maturity == "production")
                                })
                                .map(|f| f.id.clone())
                                .collect();

                            // Extract capabilities from snapshot
                            if let Some(caps) = yaml.get("caps").and_then(|v| v.as_sequence()) {
                                let caps_set: std::collections::BTreeSet<String> = caps
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect();

                                // Check for mismatches
                                let missing_in_caps: Vec<_> =
                                    catalog_advertised.difference(&caps_set).collect();
                                let extra_in_caps: Vec<_> =
                                    caps_set.difference(&catalog_advertised).collect();

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
                        Err(e) => warnings.push(format!("Failed to parse snapshot YAML: {}", e)),
                    }
                } else {
                    warnings.push("Snapshot file doesn't contain valid YAML section".to_string());
                }
            }
            Err(e) => warnings.push(format!("Failed to read snapshot file: {}", e)),
        }
    } else {
        warnings.push("Snapshot file not found - run 'cargo test -p perl-parser --test lsp_features_snapshot_test' to generate".to_string());
    }

    // Verify compliance percentage matches what's documented
    let _total_features = catalog.feature.len();
    let non_planned = catalog.feature.iter().filter(|f| f.maturity != "planned").count();
    let advertised_ga_prod = catalog
        .feature
        .iter()
        .filter(|f| f.advertised && (f.maturity == "ga" || f.maturity == "production"))
        .count();

    let computed_compliance = if non_planned > 0 {
        ((advertised_ga_prod as f64 / non_planned as f64 * 100.0).round()) as u32
    } else {
        0
    };

    // Check ROADMAP.md for documented percentage
    if let Ok(roadmap) = fs::read_to_string("ROADMAP.md") {
        let regex = regex::Regex::new(r"partial LSP 3\.18 compliance \(~(\d+)%\)")?;
        if let Some(cap) = regex.captures(&roadmap) {
            if let Some(doc_percent) = cap.get(1).and_then(|m| m.as_str().parse::<u32>().ok()) {
                if doc_percent != computed_compliance {
                    // Make this a hard error unless explicitly allowed
                    if std::env::var("CI_ALLOW_COMPLIANCE_DRIFT").is_err() {
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
        }
    }

    println!(
        "📊 Computed compliance: {}% ({}/{} non-planned features)",
        computed_compliance, advertised_ga_prod, non_planned
    );

    // Report results
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

    // Calculate stats
    let total = catalog.feature.len();
    let advertised = catalog.feature.iter().filter(|f| f.advertised).count();
    let ga = catalog.feature.iter().filter(|f| f.maturity == "ga" && f.advertised).count();
    let preview =
        catalog.feature.iter().filter(|f| f.maturity == "preview" && f.advertised).count();
    let experimental = catalog.feature.iter().filter(|f| f.maturity == "experimental").count();
    let planned = catalog.feature.iter().filter(|f| f.maturity == "planned").count();

    println!("\n=== LSP Compliance Report ===");
    println!("Version: {} | LSP: {}", catalog.meta.version, catalog.meta.lsp_version);
    println!("\nOverall: {}/{} features ({}%)", advertised, total, advertised * 100 / total);
    println!("\nBreakdown:");
    println!("  GA:           {} features", ga);
    println!("  Preview:      {} features", preview);
    println!("  Experimental: {} features", experimental);
    println!("  Planned:      {} features", planned);

    // By area
    let mut area_stats: HashMap<String, (usize, usize)> = HashMap::new();
    for feature in &catalog.feature {
        let entry = area_stats.entry(feature.area.clone()).or_insert((0, 0));
        entry.1 += 1; // total
        if feature.advertised {
            entry.0 += 1; // advertised
        }
    }

    println!("\nBy Area:");
    for (area, (impl_count, total_count)) in area_stats {
        let coverage = (impl_count as f32 / total_count as f32 * 100.0) as u32;
        println!("  {:20} {}/{} ({}%)", area.replace('_', " "), impl_count, total_count, coverage);
    }

    Ok(())
}
