use perl_lsp_perltidy::{BuiltInFormatter, PerlTidyConfig, PerlTidyFormatter};
use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::must;
use std::sync::Arc;

// --- Configuration flag generation tests ---

#[test]
fn config_to_args_includes_core_flags() {
    let args = PerlTidyConfig::default().to_args();
    assert!(args.contains(&"--maximum-line-length=80".to_string()));
    assert!(args.contains(&"--indent-columns=4".to_string()));
    assert!(args.contains(&"--notabs".to_string()));
}

#[test]
fn pbp_preset_sets_best_practices_flag() {
    let args = PerlTidyConfig::pbp().to_args();
    assert!(args.contains(&"--perl-best-practices".to_string()));
    assert!(args.contains(&"--maximum-line-length=78".to_string()));
}

#[test]
fn config_with_profile_uses_only_profile_flag() {
    let config = PerlTidyConfig {
        profile: Some("/home/user/.perltidyrc".to_string()),
        ..PerlTidyConfig::default()
    };
    let args = config.to_args();

    assert_eq!(args.len(), 1);
    assert_eq!(args[0], "--profile=/home/user/.perltidyrc");
}

#[test]
fn config_with_profile_ignores_other_settings() {
    let config = PerlTidyConfig {
        maximum_line_length: Some(120),
        indent_columns: Some(8),
        tabs: Some(true),
        profile: Some(".perltidyrc".to_string()),
        ..PerlTidyConfig::default()
    };
    let args = config.to_args();

    // Only profile flag should be present; all others suppressed
    assert_eq!(args.len(), 1);
    assert!(args[0].starts_with("--profile="));
}

#[test]
fn gnu_preset_sets_gnu_style_flag() {
    let args = PerlTidyConfig::gnu().to_args();
    assert!(args.contains(&"--gnu-style".to_string()));
}

#[test]
fn gnu_preset_uses_two_space_indent() {
    let args = PerlTidyConfig::gnu().to_args();
    assert!(args.contains(&"--indent-columns=2".to_string()));
}

#[test]
fn gnu_preset_opens_brace_on_new_line() {
    let args = PerlTidyConfig::gnu().to_args();
    assert!(args.contains(&"--opening-brace-on-new-line".to_string()));
}

#[test]
fn config_tabs_true_generates_tabs_flag() {
    let config = PerlTidyConfig { tabs: Some(true), ..PerlTidyConfig::default() };
    let args = config.to_args();
    assert!(args.contains(&"--tabs".to_string()));
    assert!(!args.contains(&"--notabs".to_string()));
}

#[test]
fn config_cuddled_else_false_generates_nocuddled() {
    let config = PerlTidyConfig { cuddled_else: Some(false), ..PerlTidyConfig::default() };
    let args = config.to_args();
    assert!(args.contains(&"--nocuddled-else".to_string()));
}

#[test]
fn config_space_after_keyword_false() {
    let config = PerlTidyConfig { space_after_keyword: Some(false), ..PerlTidyConfig::default() };
    let args = config.to_args();
    assert!(args.contains(&"--nospace-after-keyword".to_string()));
}

#[test]
fn config_add_trailing_commas_true() {
    let config = PerlTidyConfig { add_trailing_commas: Some(true), ..PerlTidyConfig::default() };
    let args = config.to_args();
    assert!(args.contains(&"--add-trailing-commas".to_string()));
}

#[test]
fn config_vertical_alignment_false() {
    let config = PerlTidyConfig { vertical_alignment: Some(false), ..PerlTidyConfig::default() };
    let args = config.to_args();
    assert!(args.contains(&"--no-vertical-alignment".to_string()));
}

#[test]
fn config_extra_args_appended() {
    let config = PerlTidyConfig {
        extra_args: vec!["--custom-flag".to_string(), "--another".to_string()],
        ..PerlTidyConfig::default()
    };
    let args = config.to_args();
    assert!(args.contains(&"--custom-flag".to_string()));
    assert!(args.contains(&"--another".to_string()));
}

#[test]
fn config_none_fields_omit_flags() {
    let config = PerlTidyConfig {
        maximum_line_length: None,
        indent_columns: None,
        tabs: None,
        opening_brace_on_new_line: None,
        cuddled_else: None,
        space_after_keyword: None,
        add_trailing_commas: None,
        vertical_alignment: None,
        block_comment_indentation: None,
        profile: None,
        extra_args: Vec::new(),
        timeout_secs: 10,
    };
    let args = config.to_args();
    assert!(args.is_empty());
}

#[test]
fn config_serializes_and_deserializes() -> Result<(), Box<dyn std::error::Error>> {
    let config = PerlTidyConfig::default();
    let json = serde_json::to_string(&config)?;
    let restored: PerlTidyConfig = serde_json::from_str(&json)?;
    assert_eq!(restored.maximum_line_length, config.maximum_line_length);
    assert_eq!(restored.indent_columns, config.indent_columns);
    assert_eq!(restored.tabs, config.tabs);
    Ok(())
}

#[test]
fn config_profile_path_passed_to_perltidy() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"formatted\n".to_vec()));
    let config = PerlTidyConfig {
        profile: Some("/project/.perltidyrc".to_string()),
        ..PerlTidyConfig::default()
    };
    let mut formatter = PerlTidyFormatter::new(config, runtime.clone());

    let _ = must(formatter.format("code"));

    let invocations = runtime.invocations();
    assert!(invocations[0].args.contains(&"--profile=/project/.perltidyrc".to_string()));
}

// --- Built-in formatter tests ---

#[test]
fn builtin_formatter_indents_block_contents() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($x) {\nprint $x;\n}\n");
    assert!(formatted.contains("    print"));
}

#[test]
fn builtin_formatter_dedents_closing_braces() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("sub foo {\nreturn 1;\n}\n");

    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[0], "sub foo {");
    assert_eq!(lines[1], "    return 1;");
    assert_eq!(lines[2], "}");
}

#[test]
fn builtin_formatter_handles_nested_blocks() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($a) {\nif ($b) {\nprint 1;\n}\n}\n");

    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[0], "if ($a) {");
    assert_eq!(lines[1], "    if ($b) {");
    assert_eq!(lines[2], "        print 1;");
    assert_eq!(lines[3], "    }");
    assert_eq!(lines[4], "}");
}

#[test]
fn builtin_formatter_preserves_empty_lines() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("line1\n\nline2\n");

    assert!(formatted.contains("\n\n"));
}

#[test]
fn builtin_formatter_uses_tabs_when_configured() {
    let config = PerlTidyConfig { tabs: Some(true), ..PerlTidyConfig::default() };
    let formatter = BuiltInFormatter::new(config);
    let formatted = formatter.format("sub foo {\nreturn 1;\n}\n");

    assert!(formatted.contains("\treturn 1;"));
}

#[test]
fn builtin_formatter_respects_indent_columns() {
    let config = PerlTidyConfig { indent_columns: Some(2), ..PerlTidyConfig::default() };
    let formatter = BuiltInFormatter::new(config);
    let formatted = formatter.format("if (1) {\nprint;\n}\n");

    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[1], "  print;");
}

#[test]
fn builtin_formatter_handles_parens_and_brackets() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("my @arr = (\n1,\n2,\n);\n");

    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[1], "    1,");
    assert_eq!(lines[2], "    2,");
    assert_eq!(lines[3], ");");
}

#[test]
fn builtin_formatter_handles_empty_input() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("");
    assert_eq!(formatted, "");
}

#[test]
fn builtin_formatter_indents_after_opening_delimiter_before_comment() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($x) { # comment\nprint $x;\n}\n");

    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[0], "if ($x) { # comment");
    assert_eq!(lines[1], "    print $x;");
    assert_eq!(lines[2], "}");
}

#[test]
fn builtin_formatter_does_not_count_delimiters_inside_strings_or_comments() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($x) {\nprint \"}\"; # {\nprint q{literal};\n}\n");

    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[0], "if ($x) {");
    assert_eq!(lines[1], "    print \"}\"; # {");
    assert_eq!(lines[2], "    print q{literal};");
    assert_eq!(lines[3], "}");
}
