//! Tests for `.perl-lsp.toml` project configuration loading and merging.
//!
//! All 11 test cases correspond directly to the spec in issue #2053.

use perl_lsp_config::{ProjectConfig, ServerConfig, WorkspaceConfig, load_project_config};
use std::io::Write as _;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// Helper: create a temp dir, write a `.perl-lsp.toml` with the given content,
// and return the temp dir (which stays alive for the duration of the test).
fn write_toml(content: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(".perl-lsp.toml");
    let mut file = std::fs::File::create(&path)?;
    file.write_all(content.as_bytes())?;
    Ok(dir)
}

// ── load_project_config ───────────────────────────────────────────────────────

#[test]
fn project_config_missing_file_returns_ok_none() -> TestResult {
    let dir = tempfile::tempdir()?;
    // No .perl-lsp.toml written — file does not exist
    let result = load_project_config(dir.path())?;
    assert!(result.is_none(), "expected None when file absent");
    Ok(())
}

#[test]
fn project_config_empty_file_returns_default() -> TestResult {
    let dir = write_toml("")?;
    let cfg = load_project_config(dir.path())?.ok_or("expected Some for empty file")?;
    // All fields should be at default (empty/None)
    assert!(cfg.perl.include_paths.is_empty());
    assert!(cfg.perl.version.is_none());
    assert!(cfg.diagnostics.perlcritic.is_none());
    assert!(cfg.diagnostics.perlcritic_severity.is_none());
    assert!(cfg.features.inlay_hints.is_none());
    Ok(())
}

#[test]
fn project_config_full_config_parsed_correctly() -> TestResult {
    let dir = write_toml(
        r#"
[perl]
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
perlcritic = true
perlcritic_severity = 2

[features]
inlay_hints = false
"#,
    )?;
    let cfg = load_project_config(dir.path())?.ok_or("expected Some")?;

    assert_eq!(cfg.perl.include_paths, vec!["lib", "local/lib/perl5"]);
    assert_eq!(cfg.diagnostics.perlcritic, Some(true));
    assert_eq!(cfg.diagnostics.perlcritic_severity, Some(2));
    assert_eq!(cfg.features.inlay_hints, Some(false));
    Ok(())
}

#[test]
fn project_config_malformed_toml_returns_err() -> TestResult {
    let dir = write_toml("[invalid\ntoml = !!!")?;
    let result = load_project_config(dir.path());
    let msg = result
        .err()
        .ok_or("expected Err for malformed TOML, got Ok")?;
    assert!(
        msg.contains(".perl-lsp.toml"),
        "error message should reference the file: {msg}"
    );
    Ok(())
}

#[test]
fn project_config_unknown_keys_are_ignored() -> TestResult {
    let dir = write_toml(
        r#"
[perl]
unknown_future_key = "whatever"
include_paths = ["lib"]
"#,
    )?;
    let cfg = load_project_config(dir.path())
        .map_err(|e| format!("unknown keys must not cause an error: {e}"))?
        .ok_or("expected Some")?;
    assert_eq!(cfg.perl.include_paths, vec!["lib"]);
    Ok(())
}

/// Parsing the complete example from issue #2053 must succeed.
///
/// The issue spec includes `[formatting]` and `code_lens` in `[features]`, neither of
/// which is modelled in the current structs. This test proves they are silently ignored
/// (not treated as parse errors) so users who copy the spec verbatim are not broken.
#[test]
fn project_config_full_issue_spec_toml_parses_cleanly() -> TestResult {
    let dir = write_toml(
        r#"
[perl]
version = "5.38"
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
perlcritic = false
perlcritic_severity = 3

[formatting]
perltidy = true
perltidy_profile = ".perltidyrc"

[features]
inlay_hints = true
code_lens = true
"#,
    )?;
    // Must not error — unknown sections [formatting] and unknown key code_lens are silently ignored.
    let cfg = load_project_config(dir.path())
        .map_err(|e| format!("full issue-spec TOML must parse without error: {e}"))?
        .ok_or("expected Some for full issue-spec TOML")?;

    // Known fields are correctly parsed
    assert_eq!(cfg.perl.include_paths, vec!["lib", "local/lib/perl5"]);
    assert_eq!(cfg.perl.version.as_deref(), Some("5.38"));
    assert_eq!(cfg.diagnostics.perlcritic, Some(false));
    assert_eq!(cfg.diagnostics.perlcritic_severity, Some(3));
    assert_eq!(cfg.features.inlay_hints, Some(true));
    // Unknown sections/keys do not appear in any field
    Ok(())
}

// ── apply_to_server_config ────────────────────────────────────────────────────

#[test]
fn project_config_applies_perlcritic_to_server_config() -> TestResult {
    let mut config = ServerConfig::default();
    let mut project = ProjectConfig::default();
    project.diagnostics = perl_lsp_config::ProjectDiagnosticsConfig {
        perlcritic: Some(true),
        perlcritic_severity: Some(2),
    };
    project.apply_to_server_config(&mut config);
    assert!(config.perlcritic_enabled, "perlcritic should be enabled");
    assert_eq!(config.perlcritic_severity, 2, "severity should be 2");
    Ok(())
}

#[test]
fn project_config_perlcritic_severity_clamped() -> TestResult {
    let mut config = ServerConfig::default();

    // Severity 0 should clamp to 1
    let mut project_low = ProjectConfig::default();
    project_low.diagnostics = perl_lsp_config::ProjectDiagnosticsConfig {
        perlcritic: None,
        perlcritic_severity: Some(0),
    };
    project_low.apply_to_server_config(&mut config);
    assert_eq!(
        config.perlcritic_severity, 1,
        "severity 0 should clamp to 1"
    );

    // Severity 99 should clamp to 5
    let mut project_high = ProjectConfig::default();
    project_high.diagnostics = perl_lsp_config::ProjectDiagnosticsConfig {
        perlcritic: None,
        perlcritic_severity: Some(99),
    };
    project_high.apply_to_server_config(&mut config);
    assert_eq!(
        config.perlcritic_severity, 5,
        "severity 99 should clamp to 5"
    );
    Ok(())
}

#[test]
fn project_config_unset_fields_leave_server_config_defaults() -> TestResult {
    let mut config = ServerConfig::default();
    let default_inlay = config.inlay_hints_enabled;
    let default_perlcritic = config.perlcritic_enabled;
    let default_severity = config.perlcritic_severity;

    // Empty project config — nothing set
    let project = ProjectConfig::default();
    project.apply_to_server_config(&mut config);

    assert_eq!(
        config.inlay_hints_enabled, default_inlay,
        "inlay_hints unchanged"
    );
    assert_eq!(
        config.perlcritic_enabled, default_perlcritic,
        "perlcritic unchanged"
    );
    assert_eq!(
        config.perlcritic_severity, default_severity,
        "severity unchanged"
    );
    Ok(())
}

// ── apply_to_workspace_config ─────────────────────────────────────────────────

#[test]
fn project_config_applies_include_paths_to_workspace_config() -> TestResult {
    let mut config = WorkspaceConfig::default();
    let mut project = ProjectConfig::default();
    project.perl = perl_lsp_config::ProjectPerlConfig {
        include_paths: vec!["lib".to_string(), "local/lib/perl5".to_string()],
        version: None,
        use_perl5lib: None,
        perl5lib_precedence: None,
    };
    project.apply_to_workspace_config(&mut config);
    assert_eq!(config.include_paths, vec!["lib", "local/lib/perl5"]);
    Ok(())
}

#[test]
fn project_config_empty_include_paths_leaves_workspace_defaults() -> TestResult {
    let mut config = WorkspaceConfig::default();
    let default_paths = config.include_paths.clone();

    // Empty include_paths — should NOT override defaults
    let mut project = ProjectConfig::default();
    project.perl = perl_lsp_config::ProjectPerlConfig {
        include_paths: vec![], // explicitly empty
        version: None,
        use_perl5lib: None,
        perl5lib_precedence: None,
    };
    project.apply_to_workspace_config(&mut config);
    assert_eq!(
        config.include_paths, default_paths,
        "empty include_paths in TOML must leave workspace defaults unchanged"
    );
    Ok(())
}

// ── LSP override wins ─────────────────────────────────────────────────────────

#[test]
fn project_config_lsp_override_wins_over_toml_base() -> TestResult {
    let mut config = ServerConfig::default();

    // Step 1: apply TOML base layer — enables perlcritic
    let mut project = ProjectConfig::default();
    project.diagnostics = perl_lsp_config::ProjectDiagnosticsConfig {
        perlcritic: Some(true),
        perlcritic_severity: Some(4),
    };
    project.apply_to_server_config(&mut config);
    assert!(config.perlcritic_enabled, "TOML set perlcritic=true");
    assert_eq!(config.perlcritic_severity, 4, "TOML set severity=4");

    // Step 2: LSP didChangeConfiguration overrides with different values
    config.update_from_value(&serde_json::json!({
        "perlcritic": {
            "enabled": false,
            "severity": 1
        }
    }));

    // LSP values must win
    assert!(
        !config.perlcritic_enabled,
        "LSP override must disable perlcritic"
    );
    assert_eq!(
        config.perlcritic_severity, 1,
        "LSP override must set severity=1"
    );
    Ok(())
}

/// A `perlcritic_severity` value larger than 255 cannot be deserialized into `u8`.
/// The server treats this as a TOML parse error: warns the user and continues with defaults.
#[test]
fn project_config_severity_out_of_u8_range_returns_err() -> TestResult {
    let dir = write_toml("[diagnostics]\nperlcritic_severity = 256\n")?;
    let result = load_project_config(dir.path());
    assert!(
        result.is_err(),
        "severity = 256 exceeds u8 range and must be a parse error, got: {:?}",
        result
    );
    Ok(())
}

/// A negative `perlcritic_severity` must be rejected at the TOML layer.
#[test]
fn project_config_severity_negative_returns_err() -> TestResult {
    let dir = write_toml("[diagnostics]\nperlcritic_severity = -1\n")?;
    let result = load_project_config(dir.path());
    assert!(
        result.is_err(),
        "negative severity must be a parse error, got: {:?}",
        result
    );
    Ok(())
}

// ── parse_perl5lib ────────────────────────────────────────────────────────────

#[test]
fn parse_perl5lib_empty_string_returns_empty() {
    let result = perl_lsp_config::WorkspaceConfig::parse_perl5lib("");
    assert!(result.is_empty(), "empty string produces no paths");
}

#[test]
fn parse_perl5lib_single_path() {
    let result = perl_lsp_config::WorkspaceConfig::parse_perl5lib("/usr/lib/perl5");
    assert_eq!(result, vec!["/usr/lib/perl5"]);
}

#[test]
fn parse_perl5lib_multiple_paths() {
    #[cfg(windows)]
    let value = "/a/lib;/b/lib";
    #[cfg(not(windows))]
    let value = "/a/lib:/b/lib";

    let result = perl_lsp_config::WorkspaceConfig::parse_perl5lib(value);
    assert_eq!(result, vec!["/a/lib", "/b/lib"]);
}

#[test]
fn parse_perl5lib_drops_empty_components() {
    // Leading/trailing/consecutive separators produce empty components — they must be dropped.
    #[cfg(windows)]
    let value = ";/a/lib;;/b/lib;";
    #[cfg(not(windows))]
    let value = ":/a/lib::/b/lib:";

    let result = perl_lsp_config::WorkspaceConfig::parse_perl5lib(value);
    assert_eq!(result, vec!["/a/lib", "/b/lib"]);
}

// ── effective_include_paths ───────────────────────────────────────────────────

#[test]
fn effective_include_paths_no_perl5lib_returns_include_paths() {
    let config = WorkspaceConfig::default();
    let result = config.effective_include_paths(&[]);
    assert_eq!(
        result, config.include_paths,
        "no PERL5LIB paths returns include_paths unchanged"
    );
}

#[test]
fn effective_include_paths_prepend_puts_perl5lib_first() {
    use perl_lsp_config::Perl5LibPrecedence;
    let mut config = WorkspaceConfig::default();
    config.perl5lib_precedence = Perl5LibPrecedence::Prepend;
    let p5l = vec!["/p5l/lib".to_string()];
    let result = config.effective_include_paths(&p5l);
    assert_eq!(result.first().map(String::as_str), Some("/p5l/lib"));
    assert!(result.contains(&"lib".to_string()));
}

#[test]
fn effective_include_paths_append_puts_perl5lib_last() {
    use perl_lsp_config::Perl5LibPrecedence;
    let mut config = WorkspaceConfig::default();
    config.perl5lib_precedence = Perl5LibPrecedence::Append;
    let p5l = vec!["/p5l/lib".to_string()];
    let result = config.effective_include_paths(&p5l);
    assert_eq!(result.last().map(String::as_str), Some("/p5l/lib"));
    assert_eq!(result.first().map(String::as_str), Some("lib"));
}

#[test]
fn effective_include_paths_disabled_ignores_perl5lib() {
    let mut config = WorkspaceConfig::default();
    config.use_perl5lib = false;
    let p5l = vec!["/p5l/lib".to_string()];
    let result = config.effective_include_paths(&p5l);
    assert_eq!(
        result, config.include_paths,
        "use_perl5lib=false ignores PERL5LIB paths"
    );
    assert!(!result.contains(&"/p5l/lib".to_string()));
}

// ── TOML [perl] section ───────────────────────────────────────────────────────

#[test]
fn project_config_use_perl5lib_defaults_to_none() -> TestResult {
    let dir = write_toml("[perl]\ninclude_paths = [\"lib\"]\n")?;
    let cfg = load_project_config(dir.path())?.ok_or("expected Some")?;
    assert!(
        cfg.perl.use_perl5lib.is_none(),
        "use_perl5lib absent in TOML must be None"
    );
    Ok(())
}

#[test]
fn project_config_use_perl5lib_false_parsed() -> TestResult {
    let dir = write_toml("[perl]\nuse_perl5lib = false\n")?;
    let cfg = load_project_config(dir.path())?.ok_or("expected Some")?;
    assert_eq!(cfg.perl.use_perl5lib, Some(false));
    Ok(())
}

#[test]
fn project_config_perl5lib_precedence_append_parsed() -> TestResult {
    use perl_lsp_config::Perl5LibPrecedence;
    let dir = write_toml("[perl]\nperl5lib_precedence = \"append\"\n")?;
    let cfg = load_project_config(dir.path())?.ok_or("expected Some")?;
    assert_eq!(
        cfg.perl.perl5lib_precedence,
        Some(Perl5LibPrecedence::Append)
    );
    Ok(())
}

// ── apply_to_workspace_config propagates new fields ──────────────────────────

#[test]
fn project_config_applies_use_perl5lib_false_to_workspace_config() -> TestResult {
    let mut config = WorkspaceConfig::default();
    assert!(config.use_perl5lib, "use_perl5lib is true by default");

    let mut project = ProjectConfig::default();
    project.perl = perl_lsp_config::ProjectPerlConfig {
        include_paths: vec![],
        version: None,
        use_perl5lib: Some(false),
        perl5lib_precedence: None,
    };
    project.apply_to_workspace_config(&mut config);
    assert!(
        !config.use_perl5lib,
        "apply_to_workspace_config must propagate use_perl5lib=false"
    );
    Ok(())
}

// ── WorkspaceConfig::update_from_value — PERL5LIB fields ─────────────────────

#[test]
fn workspace_config_update_from_value_sets_use_perl5lib_false() {
    let mut config = WorkspaceConfig::default();
    assert!(config.use_perl5lib, "use_perl5lib is true by default");
    config.update_from_value(&serde_json::json!({
        "workspace": { "usePerl5lib": false }
    }));
    assert!(
        !config.use_perl5lib,
        "update_from_value must set use_perl5lib=false"
    );
}

#[test]
fn workspace_config_update_from_value_sets_use_perl5lib_true() {
    let mut config = WorkspaceConfig::default();
    config.use_perl5lib = false;
    config.update_from_value(&serde_json::json!({
        "workspace": { "usePerl5lib": true }
    }));
    assert!(
        config.use_perl5lib,
        "update_from_value must set use_perl5lib=true"
    );
}

#[test]
fn workspace_config_update_from_value_sets_perl5lib_precedence_append() {
    use perl_lsp_config::Perl5LibPrecedence;
    let mut config = WorkspaceConfig::default();
    assert_eq!(config.perl5lib_precedence, Perl5LibPrecedence::Prepend);
    config.update_from_value(&serde_json::json!({
        "workspace": { "perl5libPrecedence": "append" }
    }));
    assert_eq!(
        config.perl5lib_precedence,
        Perl5LibPrecedence::Append,
        "update_from_value must set perl5lib_precedence to Append"
    );
}

#[test]
fn workspace_config_update_from_value_sets_perl5lib_precedence_prepend() {
    use perl_lsp_config::Perl5LibPrecedence;
    let mut config = WorkspaceConfig::default();
    config.perl5lib_precedence = Perl5LibPrecedence::Append;
    config.update_from_value(&serde_json::json!({
        "workspace": { "perl5libPrecedence": "prepend" }
    }));
    assert_eq!(
        config.perl5lib_precedence,
        Perl5LibPrecedence::Prepend,
        "update_from_value must set perl5lib_precedence to Prepend"
    );
}

#[test]
fn workspace_config_update_from_value_unknown_precedence_leaves_current_value() {
    use perl_lsp_config::Perl5LibPrecedence;
    let mut config = WorkspaceConfig::default();
    // Set to Append first, then send an unknown value — must leave Append intact.
    config.perl5lib_precedence = Perl5LibPrecedence::Append;
    config.update_from_value(&serde_json::json!({
        "workspace": { "perl5libPrecedence": "replace" }
    }));
    assert_eq!(
        config.perl5lib_precedence,
        Perl5LibPrecedence::Append,
        "unknown perl5libPrecedence value must NOT reset to Prepend"
    );
}

#[test]
fn effective_include_paths_prepend_preserves_full_order() {
    use perl_lsp_config::Perl5LibPrecedence;
    let mut config = WorkspaceConfig::default();
    config.perl5lib_precedence = Perl5LibPrecedence::Prepend;
    config.include_paths = vec!["lib".to_string(), "local/lib/perl5".to_string()];
    let p5l = vec!["/ext/a".to_string(), "/ext/b".to_string()];
    let result = config.effective_include_paths(&p5l);
    assert_eq!(
        result,
        vec!["/ext/a", "/ext/b", "lib", "local/lib/perl5"],
        "Prepend: PERL5LIB paths first (in order), then include_paths (in order)"
    );
}

#[test]
fn effective_include_paths_append_preserves_full_order() {
    use perl_lsp_config::Perl5LibPrecedence;
    let mut config = WorkspaceConfig::default();
    config.perl5lib_precedence = Perl5LibPrecedence::Append;
    config.include_paths = vec!["lib".to_string(), "local/lib/perl5".to_string()];
    let p5l = vec!["/ext/a".to_string(), "/ext/b".to_string()];
    let result = config.effective_include_paths(&p5l);
    assert_eq!(
        result,
        vec!["lib", "local/lib/perl5", "/ext/a", "/ext/b"],
        "Append: include_paths first (in order), then PERL5LIB paths (in order)"
    );
}
