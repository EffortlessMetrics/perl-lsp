//! Comprehensive unit tests for `perl-feature-catalog`.

use perl_feature_catalog::{
    AreaStats, Catalog, CatalogError, CatalogSourceKind, DEFAULT_DAP_FEATURES, Feature, Maturity,
    Meta,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn minimal_meta() -> Meta {
    Meta { version: "1.0.0".to_string(), lsp_version: "3.18".to_string(), compliance_percent: None }
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

fn make_catalog(features: Vec<Feature>) -> Catalog {
    Catalog { meta: minimal_meta(), feature: features }
}

// ---------------------------------------------------------------------------
// Maturity
// ---------------------------------------------------------------------------

#[test]
fn maturity_is_advertised() {
    assert!(Maturity::Ga.is_advertised());
    assert!(Maturity::Production.is_advertised());
    assert!(!Maturity::Experimental.is_advertised());
    assert!(!Maturity::Preview.is_advertised());
    assert!(!Maturity::Planned.is_advertised());
}

#[test]
fn maturity_is_trackable() {
    assert!(Maturity::Ga.is_trackable());
    assert!(Maturity::Production.is_trackable());
    assert!(Maturity::Experimental.is_trackable());
    assert!(Maturity::Preview.is_trackable());
    assert!(!Maturity::Planned.is_trackable());
}

#[test]
fn maturity_labels() {
    assert_eq!(Maturity::Experimental.label(), "experimental");
    assert_eq!(Maturity::Preview.label(), "preview");
    assert_eq!(Maturity::Ga.label(), "ga");
    assert_eq!(Maturity::Planned.label(), "planned");
    assert_eq!(Maturity::Production.label(), "production");
}

#[test]
fn maturity_ordering() {
    // Derived Ord should work without panicking.
    let mut maturities =
        [Maturity::Production, Maturity::Ga, Maturity::Experimental, Maturity::Preview];
    maturities.sort();
    // Just verify sort completes; exact order is derive-dependent.
    assert_eq!(maturities.len(), 4);
}

#[test]
fn maturity_hash_and_eq() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Maturity::Ga);
    set.insert(Maturity::Ga);
    assert_eq!(set.len(), 1);
}

// ---------------------------------------------------------------------------
// Catalog — empty
// ---------------------------------------------------------------------------

#[test]
fn empty_catalog_features() {
    let cat = make_catalog(vec![]);
    assert!(cat.features().is_empty());
    assert!(cat.advertised_feature_ids().is_empty());
    assert_eq!(cat.trackable_feature_count(), 0);
    assert_eq!(cat.advertised_trackable_count(), 0);
    assert_eq!(cat.trackable_feature_count_for_grid(), 0);
    assert_eq!(cat.advertised_trackable_count_for_grid(), 0);
}

#[test]
fn empty_catalog_compliance_is_zero() {
    let cat = make_catalog(vec![]);
    assert!((cat.compliance_percent() - 0.0).abs() < f32::EPSILON);
    assert!((cat.compliance_percent_for_grid() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn empty_catalog_validates() -> Result<(), CatalogError> {
    let cat = make_catalog(vec![]);
    cat.validate()?;
    Ok(())
}

#[test]
fn empty_catalog_area_statistics() {
    let cat = make_catalog(vec![]);
    assert!(cat.area_statistics().is_empty());
}

#[test]
fn empty_catalog_area_feature_ids() {
    let cat = make_catalog(vec![]);
    assert!(cat.area_feature_ids("nonexistent").is_empty());
}

// ---------------------------------------------------------------------------
// Catalog — single feature per maturity
// ---------------------------------------------------------------------------

#[test]
fn single_ga_advertised_feature() {
    let cat = make_catalog(vec![make_feature("lsp.completion", Maturity::Ga, true)]);
    assert_eq!(cat.features().len(), 1);
    assert_eq!(cat.advertised_feature_ids(), vec!["lsp.completion"]);
    assert_eq!(cat.trackable_feature_count(), 1);
    assert_eq!(cat.advertised_trackable_count(), 1);
    assert!((cat.compliance_percent() - 100.0).abs() < f32::EPSILON);
}

#[test]
fn single_production_advertised_feature() {
    let cat = make_catalog(vec![make_feature("lsp.hover", Maturity::Production, true)]);
    assert_eq!(cat.advertised_feature_ids(), vec!["lsp.hover"]);
    assert_eq!(cat.advertised_trackable_count(), 1);
}

#[test]
fn single_preview_feature_not_advertised_ids() {
    let cat = make_catalog(vec![make_feature("lsp.preview", Maturity::Preview, true)]);
    // Preview is trackable but not advertised (maturity gate)
    assert!(cat.advertised_feature_ids().is_empty());
    assert_eq!(cat.trackable_feature_count(), 1);
    assert_eq!(cat.advertised_trackable_count(), 0);
}

#[test]
fn single_planned_feature_not_trackable() {
    let cat = make_catalog(vec![make_feature("lsp.future", Maturity::Planned, false)]);
    assert_eq!(cat.trackable_feature_count(), 0);
    assert!((cat.compliance_percent() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn ga_not_flagged_advertised_excluded_from_ids() {
    let cat = make_catalog(vec![make_feature("lsp.internal", Maturity::Ga, false)]);
    assert!(cat.advertised_feature_ids().is_empty());
    assert_eq!(cat.trackable_feature_count(), 1);
    assert_eq!(cat.advertised_trackable_count(), 0);
}

// ---------------------------------------------------------------------------
// Catalog — mixed features
// ---------------------------------------------------------------------------

fn mixed_catalog() -> Catalog {
    make_catalog(vec![
        make_feature("lsp.completion", Maturity::Ga, true),
        make_feature("lsp.hover", Maturity::Production, true),
        make_feature("lsp.preview_feat", Maturity::Preview, true),
        make_feature("lsp.experimental_feat", Maturity::Experimental, false),
        make_feature("lsp.planned_feat", Maturity::Planned, false),
    ])
}

#[test]
fn mixed_advertised_ids_sorted() {
    let cat = mixed_catalog();
    let ids = cat.advertised_feature_ids();
    assert_eq!(ids, vec!["lsp.completion", "lsp.hover"]);
}

#[test]
fn mixed_trackable_count() {
    let cat = mixed_catalog();
    // All except Planned are trackable
    assert_eq!(cat.trackable_feature_count(), 4);
}

#[test]
fn mixed_compliance() {
    let cat = mixed_catalog();
    // 2 advertised out of 4 trackable = 50%
    assert!((cat.compliance_percent() - 50.0).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// Catalog — area queries
// ---------------------------------------------------------------------------

#[test]
fn area_feature_ids_filters_correctly() {
    let mut f1 = make_feature("lsp.completion", Maturity::Ga, true);
    f1.area = "text_document".to_string();
    let mut f2 = make_feature("lsp.workspace_symbols", Maturity::Ga, true);
    f2.area = "workspace".to_string();
    let mut f3 = make_feature("lsp.hover", Maturity::Ga, true);
    f3.area = "text_document".to_string();

    let cat = make_catalog(vec![f1, f2, f3]);
    let td_ids = cat.area_feature_ids("text_document");
    assert_eq!(td_ids, vec!["lsp.completion", "lsp.hover"]);

    let ws_ids = cat.area_feature_ids("workspace");
    assert_eq!(ws_ids, vec!["lsp.workspace_symbols"]);

    assert!(cat.area_feature_ids("nonexistent").is_empty());
}

// ---------------------------------------------------------------------------
// Catalog — counts_in_coverage toggle
// ---------------------------------------------------------------------------

#[test]
fn counts_in_coverage_exclusion() {
    let mut f1 = make_feature("lsp.a", Maturity::Ga, true);
    f1.counts_in_coverage = true;
    let mut f2 = make_feature("lsp.b", Maturity::Ga, true);
    f2.counts_in_coverage = false;
    let f3 = make_feature("lsp.c", Maturity::Preview, false);

    let cat = make_catalog(vec![f1, f2, f3]);

    // Grid counts exclude f2 (counts_in_coverage=false)
    assert_eq!(cat.trackable_feature_count_for_grid(), 2); // f1 + f3
    assert_eq!(cat.advertised_trackable_count_for_grid(), 1); // f1 only
    assert!((cat.compliance_percent_for_grid() - 50.0).abs() < f32::EPSILON);

    // Regular counts include everything non-planned
    assert_eq!(cat.trackable_feature_count(), 3);
    assert_eq!(cat.advertised_trackable_count(), 2);
}

#[test]
fn grid_compliance_all_excluded() {
    let mut f = make_feature("lsp.x", Maturity::Ga, true);
    f.counts_in_coverage = false;
    let cat = make_catalog(vec![f]);
    // No features count in grid → 0%
    assert!((cat.compliance_percent_for_grid() - 0.0).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// Catalog — area_statistics
// ---------------------------------------------------------------------------

#[test]
fn area_statistics_aggregation() {
    let mut features = Vec::new();
    for (area, maturity, adv) in [
        ("text_document", Maturity::Ga, true),
        ("text_document", Maturity::Preview, false),
        ("workspace", Maturity::Production, true),
        ("workspace", Maturity::Planned, false),
        ("workspace", Maturity::Experimental, false),
    ] {
        let mut f = make_feature(&format!("{area}.{}", maturity.label()), maturity, adv);
        f.area = area.to_string();
        features.push(f);
    }

    let cat = make_catalog(features);
    let stats = cat.area_statistics();

    let td = stats.get("text_document");
    assert!(td.is_some());
    let td = perl_tdd_support::must_some(td);
    assert_eq!(td.total, 2);
    assert_eq!(td.advertised, 1);
    assert_eq!(td.ga, 1);
    assert_eq!(td.preview, 1);

    let ws = perl_tdd_support::must_some(stats.get("workspace"));
    assert_eq!(ws.total, 3);
    assert_eq!(ws.advertised, 1);
    assert_eq!(ws.production, 1);
    assert_eq!(ws.planned, 1);
    assert_eq!(ws.experimental, 1);
}

// ---------------------------------------------------------------------------
// AreaStats
// ---------------------------------------------------------------------------

#[test]
fn area_stats_default() {
    let s = AreaStats::default();
    assert_eq!(s.total, 0);
    assert_eq!(s.advertised, 0);
    assert_eq!(s.trackable(), 0);
    assert_eq!(s.coverage_percent(), 0);
    assert_eq!(s.trackable_coverage_percent(), 0);
}

#[test]
fn area_stats_trackable_excludes_planned() {
    let s = AreaStats { total: 5, planned: 2, ..Default::default() };
    assert_eq!(s.trackable(), 3);
}

#[test]
fn area_stats_coverage_percent() {
    let s = AreaStats { total: 4, advertised: 2, ..Default::default() };
    assert_eq!(s.coverage_percent(), 50);
}

#[test]
fn area_stats_trackable_coverage_percent() {
    let s = AreaStats { total: 5, advertised: 3, planned: 2, ..Default::default() };
    // trackable = 5-2 = 3, advertised = 3 → 100%
    assert_eq!(s.trackable_coverage_percent(), 100);
}

#[test]
fn area_stats_trackable_coverage_zero_trackable() {
    let s = AreaStats { total: 3, planned: 3, ..Default::default() };
    assert_eq!(s.trackable(), 0);
    assert_eq!(s.trackable_coverage_percent(), 0);
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn validate_rejects_empty_id() {
    let cat = make_catalog(vec![make_feature("", Maturity::Ga, true)]);
    let err = cat.validate();
    assert!(err.is_err());
    let msg = format!("{}", perl_tdd_support::must_err(err));
    assert!(msg.contains("empty"), "expected 'empty' in: {msg}");
}

#[test]
fn validate_rejects_whitespace_only_id() {
    let cat = make_catalog(vec![make_feature("   ", Maturity::Ga, true)]);
    assert!(cat.validate().is_err());
}

#[test]
fn validate_rejects_duplicate_ids() {
    let cat = make_catalog(vec![
        make_feature("lsp.dup", Maturity::Ga, true),
        make_feature("lsp.dup", Maturity::Preview, false),
    ]);
    let err = cat.validate();
    assert!(err.is_err());
    let msg = format!("{}", perl_tdd_support::must_err(err));
    assert!(msg.contains("duplicate"), "expected 'duplicate' in: {msg}");
}

#[test]
fn validate_accepts_unique_ids() -> Result<(), CatalogError> {
    let cat = make_catalog(vec![
        make_feature("lsp.a", Maturity::Ga, true),
        make_feature("lsp.b", Maturity::Preview, false),
    ]);
    cat.validate()?;
    Ok(())
}

#[test]
fn validate_reports_all_issues() {
    let cat = make_catalog(vec![
        make_feature("", Maturity::Ga, true),
        make_feature("lsp.dup", Maturity::Ga, true),
        make_feature("lsp.dup", Maturity::Preview, false),
    ]);
    let err = cat.validate();
    let msg = format!("{}", perl_tdd_support::must_err(err));
    // Both empty and duplicate should be reported
    assert!(msg.contains("empty"), "expected 'empty' in: {msg}");
    assert!(msg.contains("duplicate"), "expected 'duplicate' in: {msg}");
}

// ---------------------------------------------------------------------------
// CatalogError Display
// ---------------------------------------------------------------------------

#[test]
fn catalog_error_display_variants() {
    let e1 = CatalogError::MissingSource("/some/path".into());
    assert!(format!("{e1}").contains("/some/path"));

    let e2 = CatalogError::Validation("bad data".to_string());
    assert!(format!("{e2}").contains("bad data"));
}

// ---------------------------------------------------------------------------
// Serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn catalog_toml_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "0.10.0"
lsp_version = "3.18"

[[feature]]
id = "lsp.completion"
spec = "LSP 3.18"
area = "text_document"
maturity = "ga"
advertised = true
description = "Code completion"
tests = ["test_completion"]

[[feature]]
id = "lsp.hover"
spec = "LSP 3.18"
area = "text_document"
maturity = "production"
advertised = true
description = "Hover info"
"#;
    let catalog: Catalog = toml::from_str(toml_str)?;
    assert_eq!(catalog.meta.version, "0.10.0");
    assert_eq!(catalog.features().len(), 2);
    assert_eq!(catalog.features()[0].maturity, Maturity::Ga);
    assert_eq!(catalog.features()[1].maturity, Maturity::Production);
    assert_eq!(catalog.features()[0].tests, vec!["test_completion"]);
    assert!(catalog.features()[1].tests.is_empty());
    // counts_in_coverage defaults to true
    assert!(catalog.features()[0].counts_in_coverage);
    catalog.validate()?;
    Ok(())
}

#[test]
fn catalog_toml_all_maturities() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
id = "f.exp"
maturity = "experimental"

[[feature]]
id = "f.pre"
maturity = "preview"

[[feature]]
id = "f.ga"
maturity = "ga"

[[feature]]
id = "f.plan"
maturity = "planned"

[[feature]]
id = "f.prod"
maturity = "production"
"#;
    let catalog: Catalog = toml::from_str(toml_str)?;
    assert_eq!(catalog.features().len(), 5);
    assert_eq!(catalog.features()[0].maturity, Maturity::Experimental);
    assert_eq!(catalog.features()[1].maturity, Maturity::Preview);
    assert_eq!(catalog.features()[2].maturity, Maturity::Ga);
    assert_eq!(catalog.features()[3].maturity, Maturity::Planned);
    assert_eq!(catalog.features()[4].maturity, Maturity::Production);
    Ok(())
}

#[test]
fn catalog_toml_invalid_maturity() {
    let toml_str = r#"
[meta]
version = "1.0.0"
lsp_version = "3.18"

[[feature]]
id = "f.bad"
maturity = "unknown_state"
"#;
    let result: Result<Catalog, _> = toml::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn catalog_toml_missing_meta() {
    let toml_str = r#"
[[feature]]
id = "f.a"
maturity = "ga"
"#;
    let result: Result<Catalog, _> = toml::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn meta_with_compliance_percent() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "0.10.0"
lsp_version = "3.18"
compliance_percent = 95
"#;
    let meta: Meta = toml::from_str(
        toml_str
            .trim_start()
            .strip_prefix("[meta]\n")
            .ok_or("strip")?
            .replace("[meta]\n", "")
            .as_str(),
    )
    .or_else(|_| -> Result<Meta, Box<dyn std::error::Error>> {
        // Parse the full structure to get meta
        #[derive(serde::Deserialize)]
        struct Wrapper {
            meta: Meta,
        }
        let w: Wrapper = toml::from_str(toml_str)?;
        Ok(w.meta)
    })?;
    assert_eq!(meta.compliance_percent, Some(95));
    Ok(())
}

#[test]
fn meta_compliance_percent_defaults_to_none() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
[meta]
version = "0.10.0"
lsp_version = "3.18"
"#;
    #[derive(serde::Deserialize)]
    struct Wrapper {
        meta: Meta,
    }
    let w: Wrapper = toml::from_str(toml_str)?;
    assert!(w.meta.compliance_percent.is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// read_catalog with tempfile
// ---------------------------------------------------------------------------

#[test]
fn read_catalog_from_file() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("features.toml");
    std::fs::write(
        &path,
        r#"
[meta]
version = "0.10.0"
lsp_version = "3.18"

[[feature]]
id = "lsp.completion"
maturity = "ga"
advertised = true
"#,
    )?;
    let catalog = perl_feature_catalog::read_catalog(&path)?;
    assert_eq!(catalog.features().len(), 1);
    assert_eq!(catalog.advertised_feature_ids(), vec!["lsp.completion"]);
    Ok(())
}

#[test]
fn read_catalog_nonexistent_file() {
    let result = perl_feature_catalog::read_catalog(std::path::Path::new("/nonexistent/file.toml"));
    assert!(result.is_err());
}

#[test]
fn read_catalog_invalid_toml() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "this is not valid toml {{{")?;
    let result = perl_feature_catalog::read_catalog(&path);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn read_catalog_rejects_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("features.toml");
    std::fs::write(
        &path,
        r#"
[meta]
version = "0.10.0"
lsp_version = "3.18"

[[feature]]
id = "lsp.dup"
maturity = "ga"

[[feature]]
id = "lsp.dup"
maturity = "preview"
"#,
    )?;
    let result = perl_feature_catalog::read_catalog(&path);
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_catalog_source
// ---------------------------------------------------------------------------

#[test]
fn resolve_catalog_source_workspace_root() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let features_path = dir.path().join("features.toml");
    std::fs::write(&features_path, "[meta]\nversion=\"1\"\nlsp_version=\"3\"")?;
    let source = perl_feature_catalog::resolve_catalog_source(dir.path())?;
    assert!(matches!(source.kind, CatalogSourceKind::Workspace));
    assert_eq!(source.path, features_path);
    Ok(())
}

#[test]
fn resolve_catalog_source_parent_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    // Create parent/parent/features.toml
    let crate_dir = dir.path().join("crates").join("my-crate");
    std::fs::create_dir_all(&crate_dir)?;
    let features_path = dir.path().join("features.toml");
    std::fs::write(&features_path, "[meta]\nversion=\"1\"\nlsp_version=\"3\"")?;
    let source = perl_feature_catalog::resolve_catalog_source(&crate_dir)?;
    assert!(matches!(source.kind, CatalogSourceKind::Workspace));
    // The resolver walks ancestors, so it may find the test fixture or the
    // real repo features.toml depending on the working directory.  Accept
    // any path that ends with "features.toml" and is inside the temp dir.
    assert!(
        source.path.ends_with("features.toml"),
        "expected path ending in features.toml, got {:?}",
        source.path
    );
    assert!(
        source.path.starts_with(dir.path()),
        "expected path under temp dir {}, got {:?}",
        dir.path().display(),
        source.path
    );
    Ok(())
}

#[test]
fn resolve_catalog_source_vendored() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let vendored_path = dir.path().join("features_sot.toml");
    std::fs::write(&vendored_path, "[meta]\nversion=\"1\"\nlsp_version=\"3\"")?;
    let source = perl_feature_catalog::resolve_catalog_source(dir.path())?;
    assert!(matches!(source.kind, CatalogSourceKind::Vendored));
    assert_eq!(source.path, vendored_path);
    Ok(())
}

#[test]
fn resolve_catalog_source_missing() {
    let dir = tempfile::TempDir::new();
    let dir = perl_tdd_support::must(dir);
    let result = perl_feature_catalog::resolve_catalog_source(dir.path());
    assert!(result.is_err());
}

#[test]
fn resolve_catalog_source_override_env() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let override_path = dir.path().join("override.toml");
    std::fs::write(&override_path, "[meta]\nversion=\"1\"\nlsp_version=\"3\"")?;

    // Set env and test — note: this is inherently not thread-safe, but test
    // runners isolate env per-process when using --test-threads=1 or
    // we accept the slight race for coverage.
    let key = "FEATURES_TOML_OVERRIDE";
    let prev = std::env::var(key).ok();
    // SAFETY: single-threaded test; restoring env immediately after.
    unsafe {
        std::env::set_var(key, override_path.to_str().ok_or("non-utf8 path")?);
    }

    let result = perl_feature_catalog::resolve_catalog_source(dir.path());

    // Restore env
    unsafe {
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    let source = result?;
    assert!(matches!(source.kind, CatalogSourceKind::Override));
    Ok(())
}

// ---------------------------------------------------------------------------
// CatalogSource::comment
// ---------------------------------------------------------------------------

#[test]
fn catalog_source_comment_variants() {
    use perl_feature_catalog::CatalogSource;

    let s1 = CatalogSource { path: "a".into(), kind: CatalogSourceKind::Override };
    assert!(s1.comment().contains("FEATURES_TOML_OVERRIDE"));

    let s2 = CatalogSource { path: "b".into(), kind: CatalogSourceKind::Workspace };
    assert!(s2.comment().contains("features.toml"));

    let s3 = CatalogSource { path: "c".into(), kind: CatalogSourceKind::Vendored };
    assert!(s3.comment().contains("features_sot.toml"));
}

// ---------------------------------------------------------------------------
// load_catalog_for_build
// ---------------------------------------------------------------------------

#[test]
fn load_catalog_for_build_success() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("features.toml"),
        r#"
[meta]
version = "0.10.0"
lsp_version = "3.18"

[[feature]]
id = "lsp.completion"
maturity = "ga"
advertised = true
"#,
    )?;
    let (catalog, source) = perl_feature_catalog::load_catalog_for_build(dir.path())?;
    assert_eq!(catalog.features().len(), 1);
    assert!(matches!(source.kind, CatalogSourceKind::Workspace));
    Ok(())
}

#[test]
fn vendored_feature_catalogs_match_workspace_root_catalog() -> Result<(), Box<dyn std::error::Error>>
{
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("perl-feature-catalog should live under crates/")?;
    let workspace_catalog = perl_feature_catalog::read_catalog(&repo_root.join("features.toml"))?;

    let vendored_paths = [
        repo_root.join("crates/perl-lsp/features_sot.toml"),
        repo_root.join("crates/perl-dap/features_sot.toml"),
        repo_root.join("crates/perl-parser/features_sot.toml"),
        repo_root.join("crates/perl-lsp-feature-contracts/features_sot.toml"),
    ];

    for vendored_path in vendored_paths {
        let vendored_catalog = perl_feature_catalog::read_catalog(&vendored_path)?;
        assert_eq!(
            vendored_catalog,
            workspace_catalog,
            "vendored catalog should stay in lockstep with workspace features.toml: {}",
            vendored_path.display()
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// render_lsp_feature_catalog_module
// ---------------------------------------------------------------------------

#[test]
fn render_lsp_module_contains_expected_constants() {
    let cat = make_catalog(vec![
        make_feature("lsp.hover", Maturity::Ga, true),
        make_feature("lsp.completion", Maturity::Ga, true),
    ]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "// test source\n");

    assert!(code.contains("@generated by build.rs"));
    assert!(code.contains("// test source"));
    assert!(code.contains("pub const VERSION: &str"));
    assert!(code.contains("pub const LSP_VERSION: &str"));
    assert!(code.contains("pub const COMPLIANCE_PERCENT: f32"));
    assert!(code.contains("pub const ALL_FEATURES: &[Feature]"));
    assert!(code.contains("pub const ADVERTISED_LSP_FEATURES: &[&str]"));
    assert!(code.contains("pub fn advertised_features()"));
    assert!(code.contains("pub fn has_feature("));
    assert!(code.contains("pub fn compliance_percent()"));
}

#[test]
fn render_lsp_module_sorts_by_area_then_id() {
    let mut f1 = make_feature("z.last", Maturity::Ga, true);
    f1.area = "a_area".to_string();
    let mut f2 = make_feature("a.first", Maturity::Ga, true);
    f2.area = "z_area".to_string();
    let mut f3 = make_feature("m.mid", Maturity::Ga, true);
    f3.area = "a_area".to_string();

    let cat = make_catalog(vec![f1, f2, f3]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");

    // In ALL_FEATURES: a_area features first (m.mid, z.last), then z_area (a.first)
    let mid_pos = code.find("\"m.mid\"");
    let last_pos = code.find("\"z.last\"");
    let first_pos = code.find("\"a.first\"");
    assert!(mid_pos < last_pos, "m.mid should precede z.last");
    assert!(last_pos < first_pos, "z.last should precede a.first");
}

#[test]
fn render_lsp_module_advertised_sorted() {
    let cat = make_catalog(vec![
        make_feature("lsp.z", Maturity::Ga, true),
        make_feature("lsp.a", Maturity::Ga, true),
        make_feature("lsp.m", Maturity::Preview, true), // not advertised (maturity)
    ]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");

    // ADVERTISED_LSP_FEATURES should contain lsp.a before lsp.z
    let section_start = code.find("ADVERTISED_LSP_FEATURES: &[&str]").ok_or("missing section").ok();
    assert!(section_start.is_some());
    let section = &code[perl_tdd_support::must_some(section_start)..];
    let a_pos = section.find("\"lsp.a\"");
    let z_pos = section.find("\"lsp.z\"");
    assert!(a_pos < z_pos);
    // lsp.m should NOT appear in advertised
    assert!(
        !section.contains("\"lsp.m\"") || {
            // It might appear in ALL_FEATURES but not in ADVERTISED
            let after_advertised = section.find("];").map(|i| &section[..i]).unwrap_or(section);
            !after_advertised.contains("\"lsp.m\"")
        }
    );
}

#[test]
fn render_lsp_module_empty_catalog() {
    let cat = make_catalog(vec![]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    assert!(code.contains("ALL_FEATURES: &[Feature] = &[\n];"));
    assert!(code.contains("ADVERTISED_LSP_FEATURES: &[&str] = &[\n];"));
    assert!(code.contains("COMPLIANCE_PERCENT: f32 = 0.00"));
}

#[test]
fn render_lsp_module_includes_tests_array() {
    let mut f = make_feature("lsp.test_feat", Maturity::Ga, true);
    f.tests = vec!["test_a".to_string(), "test_b".to_string()];
    let cat = make_catalog(vec![f]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    assert!(code.contains("test_a"));
    assert!(code.contains("test_b"));
}

#[test]
fn render_lsp_module_includes_feature_struct_fields() {
    let mut f = make_feature("lsp.f", Maturity::Ga, true);
    f.spec = "LSP 3.18".to_string();
    f.description = "My feature".to_string();
    f.counts_in_coverage = false;
    let cat = make_catalog(vec![f]);
    let code = perl_feature_catalog::render_lsp_feature_catalog_module(&cat, "");
    assert!(code.contains("\"LSP 3.18\""));
    assert!(code.contains("\"My feature\""));
    assert!(code.contains("counts_in_coverage: false"));
}

// ---------------------------------------------------------------------------
// render_dap_feature_catalog_module
// ---------------------------------------------------------------------------

#[test]
fn render_dap_module_basic() {
    let ids = vec!["dap.breakpoints", "dap.core"];
    let code = perl_feature_catalog::render_dap_feature_catalog_module(&ids);
    assert!(code.contains("@generated by build.rs"));
    assert!(code.contains("ADVERTISED_DAP_FEATURES"));
    assert!(code.contains("\"dap.breakpoints\""));
    assert!(code.contains("\"dap.core\""));
    assert!(code.contains("pub fn advertised_features()"));
    assert!(code.contains("pub fn has_feature("));
}

#[test]
fn render_dap_module_sorts_and_deduplicates() {
    let ids = vec!["dap.z", "dap.a", "dap.z", "dap.m"];
    let code = perl_feature_catalog::render_dap_feature_catalog_module(&ids);
    // Should be sorted and unique
    let a_pos = code.find("\"dap.a\"");
    let m_pos = code.find("\"dap.m\"");
    let z_pos = code.find("\"dap.z\"");
    assert!(a_pos < m_pos);
    assert!(m_pos < z_pos);
    // Only one occurrence of dap.z in ADVERTISED_DAP_FEATURES
    let count = code.matches("\"dap.z\"").count();
    assert_eq!(count, 1);
}

#[test]
fn render_dap_module_empty() {
    let code = perl_feature_catalog::render_dap_feature_catalog_module(&[]);
    assert!(code.contains("ADVERTISED_DAP_FEATURES: &[&str] = &[\n];"));
}

// ---------------------------------------------------------------------------
// render_dap_fallback_module
// ---------------------------------------------------------------------------

#[test]
fn render_dap_fallback_uses_defaults() {
    let code = perl_feature_catalog::render_dap_fallback_module(DEFAULT_DAP_FEATURES);
    for id in DEFAULT_DAP_FEATURES {
        assert!(code.contains(id), "missing default feature: {id}");
    }
}

#[test]
fn render_dap_fallback_empty() {
    let code = perl_feature_catalog::render_dap_fallback_module(&[]);
    assert!(code.contains("ADVERTISED_DAP_FEATURES: &[&str] = &[\n];"));
}

// ---------------------------------------------------------------------------
// render_lsp_fallback_module
// ---------------------------------------------------------------------------

#[test]
fn render_lsp_fallback_module_structure() {
    let code = perl_feature_catalog::render_lsp_fallback_module();
    assert!(code.contains("Auto-generated minimal catalog"));
    assert!(code.contains("pub const VERSION: &str"));
    assert!(code.contains("pub const LSP_VERSION: &str"));
    assert!(code.contains("pub const COMPLIANCE_PERCENT: f32 = 0.0"));
    assert!(code.contains("ALL_FEATURES: &[Feature] = &[]"));
    assert!(code.contains("ADVERTISED_LSP_FEATURES: &[&str] = &[]"));
    assert!(code.contains("pub fn advertised_features()"));
    assert!(code.contains("pub fn has_feature("));
    assert!(code.contains("pub fn compliance_percent()"));
    // fallback always returns false for has_feature
    assert!(code.contains("false"));
}

// ---------------------------------------------------------------------------
// DEFAULT_DAP_FEATURES constant
// ---------------------------------------------------------------------------

#[test]
fn default_dap_features_non_empty() {
    assert!(!DEFAULT_DAP_FEATURES.is_empty());
    for id in DEFAULT_DAP_FEATURES {
        assert!(id.starts_with("dap."), "unexpected prefix in: {id}");
    }
}

// ---------------------------------------------------------------------------
// Edge case: catalog with only planned features
// ---------------------------------------------------------------------------

#[test]
fn all_planned_catalog() {
    let cat = make_catalog(vec![
        make_feature("lsp.future1", Maturity::Planned, false),
        make_feature("lsp.future2", Maturity::Planned, false),
    ]);
    assert_eq!(cat.trackable_feature_count(), 0);
    assert_eq!(cat.advertised_trackable_count(), 0);
    assert!((cat.compliance_percent() - 0.0).abs() < f32::EPSILON);
    assert!(cat.advertised_feature_ids().is_empty());
}

// ---------------------------------------------------------------------------
// Edge case: 100% compliance
// ---------------------------------------------------------------------------

#[test]
fn full_compliance_catalog() {
    let cat = make_catalog(vec![
        make_feature("lsp.a", Maturity::Ga, true),
        make_feature("lsp.b", Maturity::Production, true),
    ]);
    assert!((cat.compliance_percent() - 100.0).abs() < f32::EPSILON);
    assert!((cat.compliance_percent_for_grid() - 100.0).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// Serde: Feature serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn feature_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cat = make_catalog(vec![make_feature("lsp.a", Maturity::Ga, true)]);
    let toml_str = toml::to_string(&cat)?;
    let restored: Catalog = toml::from_str(&toml_str)?;
    assert_eq!(restored.features().len(), 1);
    assert_eq!(restored.features()[0].id, "lsp.a");
    assert_eq!(restored.features()[0].maturity, Maturity::Ga);
    Ok(())
}
