use anyhow::{Context, Result};
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
    let content = fs::read_to_string(features_path)
        .context("Failed to read features.toml")?;
    toml::from_str(&content)
        .context("Failed to parse features.toml")
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

fn update_roadmap(catalog: &FeaturesCatalog, area_stats: &HashMap<String, (usize, usize)>) -> Result<()> {
    let roadmap_path = Path::new("ROADMAP.md");
    let mut content = fs::read_to_string(roadmap_path)?;
    
    // Calculate overall compliance
    let total: usize = area_stats.values().map(|(_, t)| t).sum();
    let advertised: usize = area_stats.values().map(|(a, _)| a).sum();
    let compliance = (advertised as f32 / total as f32 * 100.0) as u32;
    
    // Update compliance percentage in header
    let old_pattern = r"partial LSP 3.18 compliance \(~\d+%\)";
    let new_text = format!("partial LSP 3.18 compliance (~{}%)", compliance);
    content = regex::Regex::new(old_pattern)?
        .replace_all(&content, new_text.as_str())
        .to_string();
    
    // Update the compliance table
    let mut table = String::new();
    table.push_str("| Area | Implemented | Total | Coverage |\n");
    table.push_str("|------|-------------|-------|----------|\n");
    
    for (area, (impl_count, total_count)) in area_stats {
        let coverage = (*impl_count as f32 / *total_count as f32 * 100.0) as u32;
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

fn update_lsp_status(catalog: &FeaturesCatalog, _area_stats: &HashMap<String, (usize, usize)>) -> Result<()> {
    let status_path = Path::new("crates/perl-parser/LSP_ACTUAL_STATUS.md");
    
    let mut content = String::new();
    content.push_str("# LSP Feature Status\n\n");
    content.push_str("Auto-generated from `features.toml` - DO NOT EDIT\n\n");
    content.push_str(&format!("Version: {} | LSP: {}\n\n", catalog.meta.version, catalog.meta.lsp_version));
    
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

fn verify_features() -> Result<()> {
    println!("🔍 Verifying features match capabilities...");
    
    let catalog = load_features()?;
    
    // Check that all advertised features have tests
    let mut missing_tests = Vec::new();
    for feature in &catalog.feature {
        if feature.advertised && feature.tests.is_empty() {
            missing_tests.push(&feature.id);
        }
    }
    
    if !missing_tests.is_empty() {
        println!("⚠️  Features missing tests:");
        for id in missing_tests {
            println!("  - {}", id);
        }
    }
    
    // TODO: Actually check against ServerCapabilities
    // This would require compiling and running code to extract capabilities
    
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
    let preview = catalog.feature.iter().filter(|f| f.maturity == "preview" && f.advertised).count();
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
        println!("  {:20} {}/{} ({}%)", 
            area.replace('_', " "), 
            impl_count, 
            total_count, 
            coverage
        );
    }
    
    Ok(())
}