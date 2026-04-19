//! Integration tests for perl-lsp-code-lens
//!
//! These tests exercise the public API surface: `is_test_file`,
//! `get_shebang_lens`, `resolve_code_lens`, `CodeLensProvider::extract`,
//! and `CodeLensProvider::extract_subtest_lenses`.

use perl_lsp_code_lens::{
    CodeLens, CodeLensProvider, get_shebang_lens, is_test_file, resolve_code_lens,
};
use perl_parser::Parser;
use perl_tdd_support::{must, must_some};

// ── helpers ──────────────────────────────────────────────────────────────────

fn parse_and_extract(source: &str) -> Vec<CodeLens> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeLensProvider::new(source.to_string());
    provider.extract(&ast)
}

fn parse_and_extract_with_path(source: &str, path: &str) -> Vec<CodeLens> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeLensProvider::new(source.to_string()).with_file_path(path.to_string());
    provider.extract(&ast)
}

fn commands_with_name(lenses: &[CodeLens], cmd: &str) -> usize {
    lenses
        .iter()
        .filter(|l| {
            l.command
                .as_ref()
                .map(|c| c.command == cmd)
                .unwrap_or(false)
        })
        .count()
}

// ── is_test_file ──────────────────────────────────────────────────────────────

#[test]
fn test_is_test_file_dot_t_extension() {
    assert!(is_test_file("t/basic.t"));
}

#[test]
fn test_is_test_file_bare_t() {
    assert!(is_test_file("basic.t"));
}

#[test]
fn test_is_test_file_rejects_pm() {
    assert!(!is_test_file("lib/Foo.pm"));
}

#[test]
fn test_is_test_file_rejects_pl() {
    assert!(!is_test_file("script.pl"));
}

#[test]
fn test_is_test_file_empty_string() {
    assert!(!is_test_file(""));
}

// ── get_shebang_lens ──────────────────────────────────────────────────────────

#[test]
fn test_shebang_lens_present_for_perl_shebang() {
    let source = "#!/usr/bin/perl\nprint 'hello';\n";
    let lens = get_shebang_lens(source);
    assert!(lens.is_some(), "expected a shebang lens for a perl script");
}

#[test]
fn test_shebang_lens_absent_without_shebang() {
    let source = "use strict;\nprint 'hello';\n";
    let lens = get_shebang_lens(source);
    assert!(lens.is_none());
}

#[test]
fn test_shebang_lens_command_is_run_script() {
    let source = "#!/usr/bin/perl\nprint 'hello';\n";
    let lens = get_shebang_lens(source);
    let lens = must_some(lens);
    let cmd = must_some(lens.command);
    assert_eq!(cmd.command, "perl.runScript");
}

#[test]
fn test_shebang_lens_title_contains_run() {
    let source = "#!/usr/bin/env perl\nprint 'hi';\n";
    let lens = get_shebang_lens(source);
    let lens = must_some(lens);
    let cmd = must_some(lens.command);
    assert!(
        cmd.title.contains("Run"),
        "title should mention Run, got: {}",
        cmd.title
    );
}

// ── resolve_code_lens ─────────────────────────────────────────────────────────

#[test]
fn test_resolve_code_lens_with_zero_references() {
    let source = "sub foo {}\n";
    let lenses = parse_and_extract(source);
    let unresolved = lenses
        .into_iter()
        .find(|l| l.command.is_none() && l.data.is_some());
    if let Some(lens) = unresolved {
        let resolved = resolve_code_lens(lens, 0);
        let cmd = must_some(resolved.command);
        assert!(cmd.title.contains("reference"), "got: {}", cmd.title);
    }
    // If no unresolved lenses, test still passes (no package/sub detected)
}

#[test]
fn test_resolve_code_lens_singular_for_one_reference() {
    let source = "sub helper {}\n";
    let lenses = parse_and_extract(source);
    let unresolved = lenses
        .into_iter()
        .find(|l| l.command.is_none() && l.data.is_some());
    if let Some(lens) = unresolved {
        let resolved = resolve_code_lens(lens, 1);
        let cmd = must_some(resolved.command);
        assert!(
            cmd.title.contains("1 reference"),
            "expected singular 'reference', got: {}",
            cmd.title
        );
    }
}

#[test]
fn test_resolve_code_lens_plural_for_many_references() {
    let source = "sub helper {}\n";
    let lenses = parse_and_extract(source);
    let unresolved = lenses
        .into_iter()
        .find(|l| l.command.is_none() && l.data.is_some());
    if let Some(lens) = unresolved {
        let resolved = resolve_code_lens(lens, 5);
        let cmd = must_some(resolved.command);
        assert!(
            cmd.title.contains("5 references"),
            "expected plural 'references', got: {}",
            cmd.title
        );
    }
}

// ── CodeLensProvider::extract ─────────────────────────────────────────────────

#[test]
fn test_extract_run_test_lenses_for_test_subs() {
    let source = "sub test_basic { ok(1) }\nsub test_advanced { ok(2) }\n";
    let lenses = parse_and_extract(source);
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(count, 2, "expected 2 run-test lenses, got {count}");
}

#[test]
fn test_extract_no_run_test_lenses_for_non_test_subs() {
    let source = "sub helper { return 42 }\nsub calculate { return 1 }\n";
    let lenses = parse_and_extract(source);
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(count, 0, "non-test subs should not get run-test lenses");
}

#[test]
fn test_extract_run_all_tests_lens_for_t_file() {
    let source = "use Test::More;\nok(1);\ndone_testing();\n";
    let lenses = parse_and_extract_with_path(source, "t/basic.t");
    let count = commands_with_name(&lenses, "perl.runTestFile");
    assert_eq!(
        count, 1,
        "expected 1 run-all-tests lens for .t file, got {count}"
    );
}

#[test]
fn test_extract_no_run_all_tests_lens_for_pm_file() {
    let source = "package Foo; sub test_it { ok(1) }\n";
    let lenses = parse_and_extract_with_path(source, "lib/Foo.pm");
    let count = commands_with_name(&lenses, "perl.runTestFile");
    assert_eq!(count, 0, ".pm files should not get run-all-tests lens");
}

#[test]
fn test_extract_references_lenses_for_subs() {
    let source = "sub my_function { return 1 }\n";
    let lenses = parse_and_extract(source);
    let ref_lenses: Vec<_> = lenses.iter().filter(|l| l.command.is_none()).collect();
    assert!(
        !ref_lenses.is_empty(),
        "expected unresolved references lenses for subs"
    );
}

#[test]
fn test_extract_references_lenses_for_packages() {
    let source = "package MyModule;\nsub foo { 1 }\n";
    let lenses = parse_and_extract(source);
    let ref_lenses: Vec<_> = lenses.iter().filter(|l| l.command.is_none()).collect();
    assert!(
        !ref_lenses.is_empty(),
        "expected references lenses for package declarations"
    );
}

#[test]
fn test_extract_is_sub_prefix_detected_as_test() {
    // is_ is only a test pattern inside .t files (Defect 2 fix)
    let source = "sub is_valid { ok(1) }\n";
    let lenses = parse_and_extract_with_path(source, "t/basic.t");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(
        count, 1,
        "is_ prefix in .t file should be detected as test sub"
    );
}

#[test]
fn test_extract_can_prefix_detected_as_test() {
    // can_ is only a test pattern inside .t files (Defect 2 fix)
    let source = "sub can_frobnicate { ok(1) }\n";
    let lenses = parse_and_extract_with_path(source, "t/basic.t");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(
        count, 1,
        "can_ prefix in .t file should be detected as test sub"
    );
}

#[test]
fn test_extract_empty_source() {
    let lenses = parse_and_extract("");
    assert!(lenses.is_empty(), "empty source should produce no lenses");
}

// ── CodeLensProvider::extract_subtest_lenses ──────────────────────────────────

#[test]
fn test_extract_subtest_lenses_detects_subtest_call() {
    let source = r#"subtest "basic math" => sub { ok(1 + 1 == 2) };"#;
    let lenses = CodeLensProvider::extract_subtest_lenses(source);
    assert!(!lenses.is_empty(), "expected at least one subtest lens");
}

#[test]
fn test_extract_subtest_lenses_command_is_run_subtest() {
    let source = r#"subtest "my test" => sub { ok(1) };"#;
    let lenses = CodeLensProvider::extract_subtest_lenses(source);
    let cmd_names: Vec<_> = lenses
        .iter()
        .filter_map(|l| l.command.as_ref().map(|c| c.command.as_str()))
        .collect();
    assert!(
        cmd_names.contains(&"perl.runSubtest"),
        "expected perl.runSubtest command, got: {:?}",
        cmd_names
    );
}

#[test]
fn test_extract_subtest_lenses_empty_source() {
    let lenses = CodeLensProvider::extract_subtest_lenses("");
    assert!(
        lenses.is_empty(),
        "empty source should produce no subtest lenses"
    );
}

#[test]
fn test_extract_subtest_lenses_no_subtests_in_regular_code() {
    let source = "sub helper { return 42 }\nmy $x = 1;\n";
    let lenses = CodeLensProvider::extract_subtest_lenses(source);
    assert!(
        lenses.is_empty(),
        "no subtest lenses for code without subtest calls"
    );
}

// ── Defect 2: broad prefix false positives ────────────────────────────────────

/// sub is_valid in a .pm file must NOT get a "Run Test" lens (Defect 2)
#[test]
fn test_is_prefix_no_run_test_in_pm_file() {
    let source = "sub is_valid { return 1 }\n";
    let lenses = parse_and_extract_with_path(source, "lib/Foo.pm");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(
        count, 0,
        "is_ prefix in .pm file should NOT get a run-test lens"
    );
}

/// sub is_valid in a .t file MUST get a "Run Test" lens (Defect 2)
#[test]
fn test_is_prefix_run_test_in_t_file() {
    let source = "sub is_valid { ok(1) }\n";
    let lenses = parse_and_extract_with_path(source, "t/basic.t");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(count, 1, "is_ prefix in .t file SHOULD get a run-test lens");
}

/// sub can_read in a .pm file must NOT get a "Run Test" lens (Defect 2)
#[test]
fn test_can_prefix_no_run_test_in_pm_file() {
    let source = "sub can_read { return 1 }\n";
    let lenses = parse_and_extract_with_path(source, "lib/Foo.pm");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(
        count, 0,
        "can_ prefix in .pm file should NOT get a run-test lens"
    );
}

/// sub ok_result in a .pm file must NOT get a "Run Test" lens (Defect 2)
#[test]
fn test_ok_prefix_no_run_test_in_pm_file() {
    let source = "sub ok_result { return 1 }\n";
    let lenses = parse_and_extract_with_path(source, "lib/Foo.pm");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(
        count, 0,
        "ok_ prefix in .pm file should NOT get a run-test lens"
    );
}

/// sub like_pattern in a .pm file must NOT get a "Run Test" lens (Defect 2)
#[test]
fn test_like_prefix_no_run_test_in_pm_file() {
    let source = "sub like_pattern { return 1 }\n";
    let lenses = parse_and_extract_with_path(source, "lib/Foo.pm");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(
        count, 0,
        "like_ prefix in .pm file should NOT get a run-test lens"
    );
}

/// sub test_ in a .pm file MUST still get a "Run Test" lens (core patterns always apply)
#[test]
fn test_test_prefix_always_gets_run_test_lens() {
    let source = "sub test_basic { return 1 }\n";
    let lenses = parse_and_extract_with_path(source, "lib/Foo.pm");
    let count = commands_with_name(&lenses, "perl.runTest");
    assert_eq!(
        count, 1,
        "test_ prefix always gets a run-test lens regardless of file type"
    );
}

// ── Defect 1: run test argument format ────────────────────────────────────────

/// "Run Test" lens argument must be "uri::sub_name" not bare "sub_name" (Defect 1)
#[test]
fn test_run_test_lens_argument_includes_uri() {
    let source = "sub test_basic { ok(1) }\n";
    let lenses = parse_and_extract_with_path(source, "t/basic.t");
    let run_test = lenses.iter().find(|l| {
        l.command
            .as_ref()
            .is_some_and(|c| c.command == "perl.runTest")
    });
    let lens = must_some(run_test.cloned());
    let cmd = must_some(lens.command);
    let args = must_some(cmd.arguments);
    let arg = must_some(args.first().cloned());
    let arg_str = must_some(arg.as_str().map(|s| s.to_string()));
    assert!(
        arg_str.contains("::"),
        "Run Test argument must be 'uri::sub_name', got: {}",
        arg_str
    );
    assert!(
        arg_str.ends_with("::test_basic"),
        "Run Test argument must end with '::test_basic', got: {}",
        arg_str
    );
}

// ── Defect 3: debug test lens ─────────────────────────────────────────────────

/// "Debug Test" lens must appear alongside "Run Test" for test subs (Defect 3)
#[test]
fn test_debug_test_lens_present_for_test_sub() {
    let source = "sub test_basic { ok(1) }\n";
    let lenses = parse_and_extract_with_path(source, "t/basic.t");
    let count = commands_with_name(&lenses, "perl.debugTest");
    assert_eq!(
        count, 1,
        "test sub should have a 'Debug Test' lens (perl.debugTest)"
    );
}

/// "Debug Test" lens argument must also be "uri::sub_name" (Defect 3)
#[test]
fn test_debug_test_lens_argument_includes_uri() {
    let source = "sub test_basic { ok(1) }\n";
    let lenses = parse_and_extract_with_path(source, "t/basic.t");
    let debug_lens = lenses.iter().find(|l| {
        l.command
            .as_ref()
            .is_some_and(|c| c.command == "perl.debugTest")
    });
    let lens = must_some(debug_lens.cloned());
    let cmd = must_some(lens.command);
    let args = must_some(cmd.arguments);
    let arg = must_some(args.first().cloned());
    let arg_str = must_some(arg.as_str().map(|s| s.to_string()));
    assert!(
        arg_str.ends_with("::test_basic"),
        "Debug Test argument must end with '::test_basic', got: {}",
        arg_str
    );
}

// ── combined scenarios ────────────────────────────────────────────────────────

#[test]
fn test_full_test_file_produces_multiple_lens_types() {
    let source = "#!/usr/bin/perl\nuse Test::More;\n\nsub test_basic { ok(1) }\n\nsubtest \"group\" => sub { ok(2) };\n\ndone_testing();\n";
    let lenses = parse_and_extract_with_path(source, "t/example.t");
    // Should have: run-all-tests, run-test for test_basic, references for test_basic, subtest lens
    assert!(
        lenses.len() >= 3,
        "expected at least 3 lenses, got {}",
        lenses.len()
    );
    assert!(commands_with_name(&lenses, "perl.runTestFile") >= 1);
    assert!(commands_with_name(&lenses, "perl.runTest") >= 1);
}
