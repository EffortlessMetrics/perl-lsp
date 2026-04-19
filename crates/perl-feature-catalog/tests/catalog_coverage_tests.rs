//! Extended test coverage for `perl-feature-catalog`.
//!
//! Covers: real features.toml loading, feature lookup by ID, status querying
//! by maturity, compliance math edge cases, multi-area statistics, validation
//! corner cases, and render output structural checks.

use perl_feature_catalog::{AreaStats, Catalog, CatalogError, Feature, Maturity, Meta};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn minimal_meta() -> Meta {
    Meta {
        version: "1.0.0".to_string(),
        lsp_version: "3.18".to_string(),
        compliance_percent: None,
    }
}

fn make_feature(id: &str, maturity: Maturity, advertised: bool) -> Feature {
    Feature {
        id: id.to_string(),
        spec: String::new(),
        area: "text_document".to_string(),
        maturity,
        advertised,
        tests: Vec::new(),
        counts_in_coverage: true,
        description: String::new(),
    }
}

fn make_feature_in_area(id: &str, maturity: Maturity, advertised: bool, area: &str) -> Feature {
    Feature {
        id: id.to_string(),
        spec: String::new(),
        area: area.to_string(),
        maturity,
        advertised,
        tests: Vec::new(),
        counts_in_coverage: true,
        description: String::new(),
    }
}

fn make_catalog(features: Vec<Feature>) -> Catalog {
    Catalog {
        meta: minimal_meta(),
        feature: features,
    }
}

// ---------------------------------------------------------------------------
// Real features.toml loading
// ---------------------------------------------------------------------------

#[test]
fn load_real_features_toml() -> Result<(), Box<dyn std::error::Error>> {
    // Walk up from the crate directory to find the workspace root features.toml
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("could not find workspace root")?;
    let features_path = workspace_root.join("features.toml");
    if !features_path.exists() {
        // Skip gracefully in environments where features.toml is absent
        return Ok(());
    }
    let catalog = perl_feature_catalog::read_catalog(&features_path)?;

    // Basic structural assertions about the real catalog
    assert!(
        !catalog.features().is_empty(),
        "real features.toml should have features"
    );
    assert!(
        !catalog.meta.version.is_empty(),
        "meta.version should be non-empty"
    );
    assert!(
        !catalog.meta.lsp_version.is_empty(),
        "meta.lsp_version should be non-empty"
    );

    // Validation must pass on the real file
    catalog.validate()?;
    Ok(())
}

#[test]
fn real_features_toml_has_advertised_features() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("could not find workspace root")?;
    let features_path = workspace_root.join("features.toml");
    if !features_path.exists() {
        return Ok(());
    }
    let catalog = perl_feature_catalog::read_catalog(&features_path)?;
    let advertised = catalog.advertised_feature_ids();

    assert!(
        !advertised.is_empty(),
        "real catalog should have advertised features"
    );
    // Advertised IDs must be sorted
    let mut sorted = advertised.clone();
    sorted.sort_unstable();
    assert_eq!(advertised, sorted, "advertised IDs should be sorted");
    Ok(())
}

#[test]
fn real_features_toml_has_multiple_areas() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("could not find workspace root")?;
    let features_path = workspace_root.join("features.toml");
    if !features_path.exists() {
        return Ok(());
    }
    let catalog = perl_feature_catalog::read_catalog(&features_path)?;
    let stats = catalog.area_statistics();

    assert!(
        stats.len() >= 2,
        "real catalog should span at least 2 areas, found {}",
        stats.len()
    );
    Ok(())
}

#[test]
fn real_features_toml_compliance_is_positive() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("could not find workspace root")?;
    let features_path = workspace_root.join("features.toml");
    if !features_path.exists() {
        return Ok(());
    }
    let catalog = perl_feature_catalog::read_catalog(&features_path)?;
    let pct = catalog.compliance_percent();

    assert!(
        pct > 0.0,
        "real catalog compliance should be positive, got {pct}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Feature lookup by ID (filtering pattern)
// ---------------------------------------------------------------------------

#[test]
fn lookup_feature_by_id_found() {
    let cat = make_catalog(vec![
        make_feature("lsp.completion", Maturity::Ga, true),
        make_feature("lsp.hover", Maturity::Production, true),
        make_feature("lsp.diagnostics", Maturity::Experimental, false),
    ]);

    let found = cat.features().iter().find(|f| f.id == "lsp.hover");
    assert!(found.is_some());
    let f = perl_tdd_support::must_some(found);
    assert_eq!(f.maturity, Maturity::Production);
    assert!(f.advertised);
}

#[test]
fn lookup_feature_by_id_not_found() {
    let cat = make_catalog(vec![make_feature("lsp.completion", Maturity::Ga, true)]);
    let found = cat.features().iter().find(|f| f.id == "lsp.nonexistent");
    assert!(found.is_none());
}

#[test]
fn lookup_feature_by_id_prefix_match() {
    let cat = make_catalog(vec![
        make_feature("lsp.completion", Maturity::Ga, true),
        make_feature("lsp.completion.snippet", Maturity::Preview, false),
        make_feature("dap.breakpoints", Maturity::Ga, true),
    ]);
    let lsp_features: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.id.starts_with("lsp."))
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(lsp_features.len(), 2);
    assert!(lsp_features.contains(&"lsp.completion"));
    assert!(lsp_features.contains(&"lsp.completion.snippet"));
}

// ---------------------------------------------------------------------------
// Feature status querying by maturity
// ---------------------------------------------------------------------------

fn multi_status_catalog() -> Catalog {
    make_catalog(vec![
        make_feature("f.ga1", Maturity::Ga, true),
        make_feature("f.ga2", Maturity::Ga, true),
        make_feature("f.prod1", Maturity::Production, true),
        make_feature("f.prev1", Maturity::Preview, false),
        make_feature("f.prev2", Maturity::Preview, true),
        make_feature("f.exp1", Maturity::Experimental, false),
        make_feature("f.plan1", Maturity::Planned, false),
        make_feature("f.plan2", Maturity::Planned, false),
    ])
}

#[test]
fn query_ga_features() {
    let cat = multi_status_catalog();
    let ga: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.maturity == Maturity::Ga)
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(ga, vec!["f.ga1", "f.ga2"]);
}

#[test]
fn query_production_features() {
    let cat = multi_status_catalog();
    let prod: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.maturity == Maturity::Production)
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(prod, vec!["f.prod1"]);
}

#[test]
fn query_preview_features() {
    let cat = multi_status_catalog();
    let preview: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.maturity == Maturity::Preview)
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(preview, vec!["f.prev1", "f.prev2"]);
}

#[test]
fn query_experimental_features() {
    let cat = multi_status_catalog();
    let experimental: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.maturity == Maturity::Experimental)
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(experimental, vec!["f.exp1"]);
}

#[test]
fn query_planned_features() {
    let cat = multi_status_catalog();
    let planned: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.maturity == Maturity::Planned)
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(planned, vec!["f.plan1", "f.plan2"]);
}

#[test]
fn query_stable_features_ga_or_production() {
    let cat = multi_status_catalog();
    let stable: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.maturity.is_advertised())
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(stable, vec!["f.ga1", "f.ga2", "f.prod1"]);
}

#[test]
fn query_trackable_features_excludes_planned() {
    let cat = multi_status_catalog();
    let trackable: Vec<&str> = cat
        .features()
        .iter()
        .filter(|f| f.maturity.is_trackable())
        .map(|f| f.id.as_str())
        .collect();
    // All except the 2 planned features
    assert_eq!(trackable.len(), 6);
    assert!(!trackable.contains(&"f.plan1"));
    assert!(!trackable.contains(&"f.plan2"));
}

#[test]
fn advertised_ids_requires_both_maturity_and_flag() {
    let cat = multi_status_catalog();
    let advertised = cat.advertised_feature_ids();
    // f.ga1, f.ga2 (GA + advertised=true), f.prod1 (Production + advertised=true)
    // f.prev2 has advertised=true but Preview maturity - excluded
    assert_eq!(advertised, vec!["f.ga1", "f.ga2", "f.prod1"]);
    assert!(!advertised.contains(&"f.prev2"));
}

// ---------------------------------------------------------------------------
// Compliance math edge cases
// ---------------------------------------------------------------------------

#[test]
fn compliance_rounding_one_third() {
    // 1 advertised out of 3 trackable = 33.333... -> rounds to 33.0
    let cat = make_catalog(vec![
        make_feature("f.a", Maturity::Ga, true),
        make_feature("f.b", Maturity::Preview, false),
        make_feature("f.c", Maturity::Experimental, false),
    ]);
    let pct = cat.compliance_percent();
    assert!(
        (pct - 33.0).abs() < f32::EPSILON,
        "expected 33.0, got {pct}"
    );
}

#[test]
fn compliance_rounding_two_thirds() {
    // 2 advertised out of 3 trackable = 66.667 -> rounds to 67.0
    let cat = make_catalog(vec![
        make_feature("f.a", Maturity::Ga, true),
        make_feature("f.b", Maturity::Production, true),
        make_feature("f.c", Maturity::Experimental, false),
    ]);
    let pct = cat.compliance_percent();
    assert!(
        (pct - 67.0).abs() < f32::EPSILON,
        "expected 67.0, got {pct}"
    );
}

#[test]
fn compliance_large_catalog() {
    // Build a catalog with many features to test larger scale
    let mut features = Vec::new();
    for i in 0..100 {
        let maturity = if i < 80 {
            Maturity::Ga
        } else {
            Maturity::Preview
        };
        let advertised = i < 80;
        features.push(make_feature(&format!("f.{i}"), maturity, advertised));
    }
    let cat = make_catalog(features);
    assert_eq!(cat.trackable_feature_count(), 100);
    assert_eq!(cat.advertised_trackable_count(), 80);
    assert!((cat.compliance_percent() - 80.0).abs() < f32::EPSILON);
}

#[test]
fn compliance_single_planned_only() {
    let cat = make_catalog(vec![make_feature("f.x", Maturity::Planned, false)]);
    // 0 trackable -> 0% (avoid division by zero)
    assert!((cat.compliance_percent() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn grid_compliance_excludes_non_coverage_features() {
    let mut f1 = make_feature("f.a", Maturity::Ga, true);
    f1.counts_in_coverage = true;
    let mut f2 = make_feature("f.b", Maturity::Ga, true);
    f2.counts_in_coverage = false;
    let mut f3 = make_feature("f.c", Maturity::Ga, true);
    f3.counts_in_coverage = true;
    let f4 = make_feature("f.d", Maturity::Preview, false);

    let cat = make_catalog(vec![f1, f2, f3, f4]);

    // Grid: trackable with counts_in_coverage = f.a + f.c + f.d = 3
    // Grid: advertised+trackable with counts_in_coverage = f.a + f.c = 2
    assert_eq!(cat.trackable_feature_count_for_grid(), 3);
    assert_eq!(cat.advertised_trackable_count_for_grid(), 2);
    let pct = cat.compliance_percent_for_grid();
    assert!(
        (pct - 67.0).abs() < f32::EPSILON,
        "expected 67.0, got {pct}"
    );
}

// ---------------------------------------------------------------------------
// Multi-area statistics
// ---------------------------------------------------------------------------

#[test]
fn area_statistics_multiple_areas_coverage() {
    let cat = make_catalog(vec![
        make_feature_in_area("td.a", Maturity::Ga, true, "text_document"),
        make_feature_in_area("td.b", Maturity::Preview, false, "text_document"),
        make_feature_in_area("ws.a", Maturity::Production, true, "workspace"),
        make_feature_in_area("ws.b", Maturity::Planned, false, "workspace"),
        make_feature_in_area("dbg.a", Maturity::Experimental, false, "debug"),
        make_feature_in_area("dbg.b", Maturity::Ga, true, "debug"),
        make_feature_in_area("dbg.c", Maturity::Ga, false, "debug"),
    ]);
    let stats = cat.area_statistics();

    assert_eq!(stats.len(), 3);

    let td = perl_tdd_support::must_some(stats.get("text_document"));
    assert_eq!(td.total, 2);
    assert_eq!(td.advertised, 1);
    assert_eq!(td.ga, 1);
    assert_eq!(td.preview, 1);
    assert_eq!(td.coverage_percent(), 50);

    let ws = perl_tdd_support::must_some(stats.get("workspace"));
    assert_eq!(ws.total, 2);
    assert_eq!(ws.advertised, 1);
    assert_eq!(ws.production, 1);
    assert_eq!(ws.planned, 1);
    assert_eq!(ws.trackable(), 1);
    assert_eq!(ws.trackable_coverage_percent(), 100);

    let dbg = perl_tdd_support::must_some(stats.get("debug"));
    assert_eq!(dbg.total, 3);
    assert_eq!(dbg.advertised, 1);
    assert_eq!(dbg.ga, 2);
    assert_eq!(dbg.experimental, 1);
}

#[test]
fn area_statistics_single_area_all_maturities() {
    let cat = make_catalog(vec![
        make_feature_in_area("a.1", Maturity::Experimental, false, "area_a"),
        make_feature_in_area("a.2", Maturity::Preview, false, "area_a"),
        make_feature_in_area("a.3", Maturity::Ga, true, "area_a"),
        make_feature_in_area("a.4", Maturity::Production, true, "area_a"),
        make_feature_in_area("a.5", Maturity::Planned, false, "area_a"),
    ]);
    let stats = cat.area_statistics();
    assert_eq!(stats.len(), 1);

    let a = perl_tdd_support::must_some(stats.get("area_a"));
    assert_eq!(a.total, 5);
    assert_eq!(a.experimental, 1);
    assert_eq!(a.preview, 1);
    assert_eq!(a.ga, 1);
    assert_eq!(a.production, 1);
    assert_eq!(a.planned, 1);
    assert_eq!(a.advertised, 2);
    assert_eq!(a.trackable(), 4);
}

#[test]
fn area_feature_ids_returns_sorted() {
    let cat = make_catalog(vec![
        make_feature_in_area("z.feat", Maturity::Ga, true, "alpha"),
        make_feature_in_area("a.feat", Maturity::Ga, true, "alpha"),
        make_feature_in_area("m.feat", Maturity::Ga, true, "alpha"),
    ]);
    let ids = cat.area_feature_ids("alpha");
    assert_eq!(ids, vec!["a.feat", "m.feat", "z.feat"]);
}

#[test]
fn area_feature_ids_empty_area_name() {
    let mut f = make_feature("f.empty_area", Maturity::Ga, true);
    f.area = String::new();
    let cat = make_catalog(vec![f]);
    let ids = cat.area_feature_ids("");
    assert_eq!(ids, vec!["f.empty_area"]);
}

// ---------------------------------------------------------------------------
// AreaStats edge cases
// ---------------------------------------------------------------------------

#[test]
fn area_stats_coverage_percent_all_advertised() {
    let s = AreaStats {
        total: 10,
        advertised: 10,
        ..Default::default()
    };
    assert_eq!(s.coverage_percent(), 100);
}

#[test]
fn area_stats_coverage_percent_none_advertised() {
    let s = AreaStats {
        total: 5,
        advertised: 0,
        ..Default::default()
    };
    assert_eq!(s.coverage_percent(), 0);
}

#[test]
fn area_stats_trackable_coverage_rounding() {
    // 2 advertised out of 3 trackable = 66.667 -> rounds to 67
    let s = AreaStats {
        total: 4,
        advertised: 2,
        planned: 1,
        ..Default::default()
    };
    assert_eq!(s.trackable(), 3);
    assert_eq!(s.trackable_coverage_percent(), 67);
}

#[test]
fn area_stats_clone_and_copy() {
    let s = AreaStats {
        total: 3,
        advertised: 1,
        ga: 1,
        preview: 1,
        experimental: 1,
        ..Default::default()
    };
    let s2 = s;
    assert_eq!(s2.total, s.total);
    assert_eq!(s2.advertised, s.advertised);
}

// ---------------------------------------------------------------------------
// Validation corner cases
// ---------------------------------------------------------------------------

#[test]
fn validate_multiple_empty_ids() {
    let cat = make_catalog(vec![
        make_feature("", Maturity::Ga, true),
        make_feature("   ", Maturity::Preview, false),
        make_feature("\t", Maturity::Experimental, false),
    ]);
    let err = cat.validate();
    assert!(err.is_err());
    let msg = format!("{}", perl_tdd_support::must_err(err));
    assert!(msg.contains("empty"), "expected 'empty' in: {msg}");
}

#[test]
fn validate_many_duplicates() {
    let cat = make_catalog(vec![
        make_feature("lsp.a", Maturity::Ga, true),
        make_feature("lsp.a", Maturity::Preview, false),
        make_feature("lsp.a", Maturity::Experimental, false),
        make_feature("lsp.b", Maturity::Ga, true),
        make_feature("lsp.b", Maturity::Planned, false),
    ]);
    let err = cat.validate();
    assert!(err.is_err());
    let msg = format!("{}", perl_tdd_support::must_err(err));
    // Should report duplicates for both lsp.a and lsp.b
    assert!(msg.contains("lsp.a"), "expected 'lsp.a' in: {msg}");
    assert!(msg.contains("lsp.b"), "expected 'lsp.b' in: {msg}");
}

#[test]
fn validate_single_valid_feature() -> Result<(), CatalogError> {
    let cat = make_catalog(vec![make_feature("only.one", Maturity::Ga, true)]);
    cat.validate()?;
    Ok(())
}

#[test]
fn validate_many_unique_features() -> Result<(), CatalogError> {
    let features: Vec<Feature> = (0..50)
        .map(|i| make_feature(&format!("f.{i}"), Maturity::Ga, true))
        .collect();
    let cat = make_catalog(features);
    cat.validate()?;
    assert_eq!(cat.features().len(), 50);
    Ok(())
}

// ---------------------------------------------------------------------------
// TOML parsing edge cases
// ---------------------------------------------------------------------------

#[test]
fn toml_parse_counts_in_coverage_false() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
id = "lsp.internal"
maturity = "ga"
advertised = false
counts_in_coverage = false
"#;
    let catalog: Catalog = toml::from_str(toml_str)?;
    assert!(!catalog.features()[0].counts_in_coverage);
    Ok(())
}

#[test]
fn toml_parse_minimal_feature_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
id = "f.minimal"
maturity = "ga"
"#;
    let catalog: Catalog = toml::from_str(toml_str)?;
    let f = &catalog.features()[0];
    // Verify defaults
    assert_eq!(f.spec, "");
    assert_eq!(f.area, "");
    assert!(!f.advertised);
    assert!(f.tests.is_empty());
    assert!(f.counts_in_coverage); // defaults to true
    assert_eq!(f.description, "");
    Ok(())
}

#[test]
fn toml_parse_feature_with_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "0.12.0"
lsp_version = "3.18"
compliance_percent = 95

[[feature]]
id = "lsp.completion"
spec = "LSP 3.0"
area = "text_document"
maturity = "ga"
advertised = true
tests = ["test_basic_completion", "test_snippet_completion"]
counts_in_coverage = true
description = "Full code completion with 150+ built-in functions"
"#;
    let catalog: Catalog = toml::from_str(toml_str)?;
    assert_eq!(catalog.meta.version, "0.12.0");
    assert_eq!(catalog.meta.compliance_percent, Some(95));

    let f = &catalog.features()[0];
    assert_eq!(f.id, "lsp.completion");
    assert_eq!(f.spec, "LSP 3.0");
    assert_eq!(f.area, "text_document");
    assert_eq!(f.maturity, Maturity::Ga);
    assert!(f.advertised);
    assert_eq!(f.tests.len(), 2);
    assert!(f.counts_in_coverage);
    assert!(f.description.contains("completion"));
    Ok(())
}

#[test]
fn toml_parse_multiple_features_mixed_maturities() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
id = "f.stable"
maturity = "ga"
advertised = true

[[feature]]
id = "f.wip"
maturity = "experimental"

[[feature]]
id = "f.upcoming"
maturity = "planned"

[[feature]]
id = "f.beta"
maturity = "preview"
advertised = true

[[feature]]
id = "f.live"
maturity = "production"
advertised = true
"#;
    let catalog: Catalog = toml::from_str(toml_str)?;
    catalog.validate()?;
    assert_eq!(catalog.features().len(), 5);

    // Only GA + Production with advertised=true appear in advertised IDs
    let advertised = catalog.advertised_feature_ids();
    assert_eq!(advertised, vec!["f.live", "f.stable"]);

    // Trackable = everything except planned = 4
    assert_eq!(catalog.trackable_feature_count(), 4);
    Ok(())
}

#[test]
fn toml_parse_missing_feature_id() {
    let toml_str = r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
maturity = "ga"
"#;
    let result: Result<Catalog, _> = toml::from_str(toml_str);
    assert!(
        result.is_err(),
        "missing id field should fail deserialization"
    );
}

#[test]
fn toml_parse_missing_maturity() {
    let toml_str = r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
id = "f.no_maturity"
"#;
    let result: Result<Catalog, _> = toml::from_str(toml_str);
    assert!(result.is_err(), "missing maturity field should fail");
}

// ---------------------------------------------------------------------------
// read_catalog integration via tempfile
// ---------------------------------------------------------------------------

#[test]
fn read_catalog_validates_after_parse() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("features.toml");
    std::fs::write(
        &path,
        r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
id = "lsp.a"
maturity = "ga"
advertised = true

[[feature]]
id = "lsp.a"
maturity = "preview"
"#,
    )?;
    // read_catalog calls validate(), so duplicates should cause an error
    let result = perl_feature_catalog::read_catalog(&path);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn read_catalog_with_many_features() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("features.toml");

    let mut toml_content = String::from("[meta]\nversion = \"1.0.0\"\nlsp_version = \"3.18\"\n\n");
    for i in 0..20 {
        toml_content.push_str(&format!(
            "[[feature]]\nid = \"f.{i}\"\nmaturity = \"ga\"\nadvertised = true\n\n"
        ));
    }
    std::fs::write(&path, &toml_content)?;

    let catalog = perl_feature_catalog::read_catalog(&path)?;
    assert_eq!(catalog.features().len(), 20);
    assert_eq!(catalog.advertised_feature_ids().len(), 20);
    assert!((catalog.compliance_percent() - 100.0).abs() < f32::EPSILON);
    Ok(())
}

// ---------------------------------------------------------------------------
// render_lsp_feature_catalog_module structural checks
// ---------------------------------------------------------------------------

#[test]
fn render_lsp_module_version_strings() {
    let mut meta = minimal_meta();
    meta.version = "0.12.0".to_string();
    meta.lsp_version = "3.18".to_string();
    let cat = Catalog {
        meta,
        feature: vec![make_feature("lsp.a", Maturity::Ga, true)],
    };
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    assert!(
        code.contains("\"0.12.0\""),
        "version should appear in output"
    );
    assert!(
        code.contains("\"3.18\""),
        "lsp_version should appear in output"
    );
}

#[test]
fn render_lsp_module_maturity_labels_in_output() {
    let cat = make_catalog(vec![
        make_feature("f.exp", Maturity::Experimental, false),
        make_feature("f.pre", Maturity::Preview, false),
        make_feature("f.ga", Maturity::Ga, true),
        make_feature("f.prod", Maturity::Production, true),
        make_feature("f.plan", Maturity::Planned, false),
    ]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    assert!(code.contains("\"experimental\""));
    assert!(code.contains("\"preview\""));
    assert!(code.contains("\"ga\""));
    assert!(code.contains("\"production\""));
    assert!(code.contains("\"planned\""));
}

#[test]
fn render_lsp_module_compliance_value() {
    let cat = make_catalog(vec![
        make_feature("f.a", Maturity::Ga, true),
        make_feature("f.b", Maturity::Preview, false),
    ]);
    // 1 advertised out of 2 trackable = 50%
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    assert!(
        code.contains("COMPLIANCE_PERCENT: f32 = 50.00"),
        "expected 50.00 compliance in output"
    );
}

#[test]
fn render_lsp_module_description_in_output() {
    let mut f = make_feature("f.desc", Maturity::Ga, true);
    f.description = "A detailed description of the feature".to_string();
    let cat = make_catalog(vec![f]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    assert!(code.contains("A detailed description of the feature"));
}

#[test]
fn render_lsp_module_special_chars_in_description() {
    let mut f = make_feature("f.special", Maturity::Ga, true);
    f.description = "Feature with \"quotes\" and \\backslash".to_string();
    let cat = make_catalog(vec![f]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    // The description should be escaped in the output
    assert!(code.contains("quotes"));
    assert!(code.contains("backslash"));
}

// ---------------------------------------------------------------------------
// render_dap_feature_catalog_module structural checks
// ---------------------------------------------------------------------------

#[test]
fn render_dap_module_single_feature() {
    let code = perl_feature_catalog::render_dap_feature_catalog_module(&["dap.core"]);
    assert!(code.contains("\"dap.core\""));
    assert!(code.contains("ADVERTISED_DAP_FEATURES"));
}

#[test]
fn render_dap_module_preserves_all_unique() {
    let ids = vec!["dap.z", "dap.a", "dap.m", "dap.b"];
    let code = perl_feature_catalog::render_dap_feature_catalog_module(&ids);
    // All four should be present
    for id in &ids {
        assert!(code.contains(id), "missing {id} in output");
    }
}

// ---------------------------------------------------------------------------
// render_lsp_fallback_module structural checks
// ---------------------------------------------------------------------------

#[test]
fn lsp_fallback_module_has_all_required_declarations() {
    let code = perl_feature_catalog::render_lsp_fallback_module();
    // Must have the Feature struct
    assert!(code.contains("pub struct Feature"));
    assert!(code.contains("pub id: &'static str"));
    assert!(code.contains("pub spec: &'static str"));
    assert!(code.contains("pub area: &'static str"));
    assert!(code.contains("pub maturity: &'static str"));
    assert!(code.contains("pub advertised: bool"));
    assert!(code.contains("pub description: &'static str"));
    assert!(code.contains("pub counts_in_coverage: bool"));
    assert!(code.contains("pub tests: &'static [&'static str]"));
}

#[test]
fn lsp_fallback_module_compliance_is_zero() {
    let code = perl_feature_catalog::render_lsp_fallback_module();
    assert!(code.contains("COMPLIANCE_PERCENT: f32 = 0.0"));
}

// ---------------------------------------------------------------------------
// Serde: full catalog serialize/deserialize round trip
// ---------------------------------------------------------------------------

#[test]
fn full_catalog_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let mut f1 = make_feature("lsp.completion", Maturity::Ga, true);
    f1.spec = "LSP 3.0".to_string();
    f1.area = "text_document".to_string();
    f1.description = "Code completion".to_string();
    f1.tests = vec!["test_complete".to_string()];

    let mut f2 = make_feature("lsp.hover", Maturity::Production, true);
    f2.area = "text_document".to_string();
    f2.counts_in_coverage = false;

    let mut f3 = make_feature("lsp.future", Maturity::Planned, false);
    f3.area = "workspace".to_string();

    let cat = Catalog {
        meta: Meta {
            version: "0.12.0".to_string(),
            lsp_version: "3.18".to_string(),
            compliance_percent: Some(85),
        },
        feature: vec![f1, f2, f3],
    };

    let toml_str = toml::to_string(&cat)?;
    let restored: Catalog = toml::from_str(&toml_str)?;

    assert_eq!(restored.meta.version, "0.12.0");
    assert_eq!(restored.meta.lsp_version, "3.18");
    assert_eq!(restored.meta.compliance_percent, Some(85));
    assert_eq!(restored.features().len(), 3);

    assert_eq!(restored.features()[0].id, "lsp.completion");
    assert_eq!(restored.features()[0].maturity, Maturity::Ga);
    assert_eq!(restored.features()[0].spec, "LSP 3.0");
    assert!(restored.features()[0].advertised);
    assert_eq!(restored.features()[0].tests, vec!["test_complete"]);

    assert!(!restored.features()[1].counts_in_coverage);
    assert_eq!(restored.features()[2].maturity, Maturity::Planned);

    restored.validate()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// CatalogError variants and Debug
// ---------------------------------------------------------------------------

#[test]
fn catalog_error_io_variant() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let catalog_err = CatalogError::from(io_err);
    let msg = format!("{catalog_err}");
    assert!(msg.contains("file missing"));
}

#[test]
fn catalog_error_validation_variant() {
    let err = CatalogError::Validation("test validation error".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("test validation error"));
    let debug_msg = format!("{err:?}");
    assert!(debug_msg.contains("Validation"));
}

#[test]
fn catalog_error_missing_source_variant() {
    let err = CatalogError::MissingSource(std::path::PathBuf::from("/tmp/missing"));
    let msg = format!("{err}");
    assert!(msg.contains("/tmp/missing"));
}

// ---------------------------------------------------------------------------
// Meta serde
// ---------------------------------------------------------------------------

#[test]
fn meta_serialize_deserialize() -> Result<(), Box<dyn std::error::Error>> {
    let meta = Meta {
        version: "2.0.0".to_string(),
        lsp_version: "3.20".to_string(),
        compliance_percent: Some(42),
    };
    let toml_str = toml::to_string(&meta)?;
    let restored: Meta = toml::from_str(&toml_str)?;
    assert_eq!(restored.version, "2.0.0");
    assert_eq!(restored.lsp_version, "3.20");
    assert_eq!(restored.compliance_percent, Some(42));
    Ok(())
}

#[test]
fn meta_clone() {
    let meta = Meta {
        version: "1.0.0".to_string(),
        lsp_version: "3.18".to_string(),
        compliance_percent: Some(50),
    };
    let cloned = meta.clone();
    assert_eq!(cloned.version, meta.version);
    assert_eq!(cloned.compliance_percent, meta.compliance_percent);
}

// ---------------------------------------------------------------------------
// Feature clone and debug
// ---------------------------------------------------------------------------

#[test]
fn feature_clone() {
    let f = make_feature("lsp.a", Maturity::Ga, true);
    let cloned = f.clone();
    assert_eq!(cloned.id, f.id);
    assert_eq!(cloned.maturity, f.maturity);
    assert_eq!(cloned.advertised, f.advertised);
}

#[test]
fn feature_debug_output() {
    let f = make_feature("lsp.test", Maturity::Preview, false);
    let debug = format!("{f:?}");
    assert!(debug.contains("lsp.test"));
    assert!(debug.contains("Preview"));
}

// ---------------------------------------------------------------------------
// Catalog clone and debug
// ---------------------------------------------------------------------------

#[test]
fn catalog_clone() {
    let cat = make_catalog(vec![
        make_feature("f.a", Maturity::Ga, true),
        make_feature("f.b", Maturity::Preview, false),
    ]);
    let cloned = cat.clone();
    assert_eq!(cloned.features().len(), 2);
    assert_eq!(cloned.meta.version, cat.meta.version);
}

#[test]
fn catalog_debug_output() {
    let cat = make_catalog(vec![make_feature("f.a", Maturity::Ga, true)]);
    let debug = format!("{cat:?}");
    assert!(debug.contains("Catalog"));
    assert!(debug.contains("f.a"));
}

// ---------------------------------------------------------------------------
// Maturity Copy trait
// ---------------------------------------------------------------------------

#[test]
fn maturity_is_copy() {
    let m = Maturity::Ga;
    let m2 = m; // Copy, not move
    assert_eq!(m, m2);
    assert!(m.is_advertised());
    assert!(m2.is_advertised());
}

// ---------------------------------------------------------------------------
// load_catalog_for_build failure
// ---------------------------------------------------------------------------

#[test]
fn load_catalog_for_build_missing_dir() {
    let result =
        perl_feature_catalog::load_catalog_for_build(std::path::Path::new("/nonexistent/dir"));
    assert!(result.is_err());
}

#[test]
fn load_catalog_for_build_invalid_content() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("features.toml"), "not valid toml {{")?;
    let result = perl_feature_catalog::load_catalog_for_build(dir.path());
    assert!(result.is_err());
    Ok(())
}
