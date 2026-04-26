mod cpan_test_helpers;

use perl_parser_core::Parser;

/// Assert that parsing produces no diagnostics AND no error markers in sexp.
fn assert_no_diagnostics(source: &str) {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();

    // Check diagnostics
    assert!(
        output.diagnostics.is_empty(),
        "Expected no diagnostics for source:\n{}\n\nDiagnostics:\n{}",
        source,
        output.diagnostics.iter().map(|d| format!("  {}", d)).collect::<Vec<_>>().join("\n"),
    );

    // Also check sexp for error markers
    let sexp = output.ast.to_sexp();
    let error_markers = [
        "(error ",
        "(Error ",
        "(missing_expression",
        "(missing_statement",
        "(missing_identifier",
        "(missing_block",
        "MissingExpression",
        "MissingStatement",
        "MissingIdentifier",
        "MissingBlock",
    ];

    for marker in &error_markers {
        assert!(
            !sexp.contains(marker),
            "Clean-parse assertion failed: found '{}' in sexp for source:\n{}\n\nsexp:\n{}",
            marker,
            source,
            sexp,
        );
    }
}

fn assert_has_diagnostic(source: &str, needle: &str) {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let diagnostics =
        output.diagnostics.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n");
    assert!(
        diagnostics.contains(needle),
        "Expected diagnostic containing {:?} for source:\n{}\n\nDiagnostics:\n{}",
        needle,
        source,
        diagnostics,
    );
}

// ---- Tests for expected_left_brace bucket fix ----
//
// The core issue: the parser doesn't allow many Perl keywords as subroutine
// names after `sub`. In Perl, `sub <ANY_BAREWORD> { ... }` is valid.
// Similarly, `&<keyword>(...)` should work for calling subs named after keywords.

// == Sub definitions with keyword names (the main source of the bucket) ==

#[test]
fn test_sub_named_return() {
    // autodie/exception.pm: sub return { ... }
    assert_no_diagnostics(r#"sub return { return $_[0]->{return} }"#);
}

#[test]
fn test_sub_named_eval() {
    // perl5db.pl, DB.pm, Thread.pm: sub eval { ... }
    assert_no_diagnostics(r#"sub eval { die "not implemented" }"#);
}

#[test]
fn test_sub_named_last() {
    // Net/POP3.pm, Net/NNTP.pm: sub last { ... }
    assert_no_diagnostics(r#"sub last { $_[0]->{last} }"#);
}

#[test]
fn test_sub_named_next() {
    // Net/NNTP.pm, DB.pm, CPAN/Distroprefs.pm: sub next { ... }
    assert_no_diagnostics(r#"sub next { $_[0]->() }"#);
}

#[test]
fn test_sub_named_sub() {
    // DB.pm: sub sub { ... }
    assert_no_diagnostics(r#"sub sub { 1 }"#);
}

#[test]
fn test_sub_named_package() {
    // Test2/EventFacet/Trace.pm: sub package { ... }
    assert_no_diagnostics(r#"sub package { $_[0]->{frame}->[0] }"#);
}

#[test]
fn test_sub_named_state() {
    // Test2/API/InterceptResult.pm: sub state { ... }
    assert_no_diagnostics(r#"sub state { $_[0]->{state} }"#);
}

#[test]
fn test_sub_named_goto() {
    // CPAN/Distribution.pm: sub goto { ... }
    assert_no_diagnostics(r#"sub goto { my $self = shift; }"#);
}

#[test]
fn test_sub_named_cmp() {
    // IO/Compress/Base/Common.pm: sub cmp { ... }
    assert_no_diagnostics("sub cmp\n{\n  my $self = shift;\n}");
}

#[test]
fn test_sub_named_for() {
    assert_no_diagnostics(r#"sub for { 1 }"#);
}

#[test]
fn test_sub_named_foreach() {
    assert_no_diagnostics(r#"sub foreach { 1 }"#);
}

#[test]
fn test_sub_named_if() {
    assert_no_diagnostics(r#"sub if { 1 }"#);
}

#[test]
fn test_sub_named_unless() {
    assert_no_diagnostics(r#"sub unless { 1 }"#);
}

#[test]
fn test_sub_named_while() {
    assert_no_diagnostics(r#"sub while { 1 }"#);
}

#[test]
fn test_sub_named_until() {
    assert_no_diagnostics(r#"sub until { 1 }"#);
}

#[test]
fn test_sub_named_my() {
    assert_no_diagnostics(r#"sub my { 1 }"#);
}

#[test]
fn test_sub_named_our() {
    assert_no_diagnostics(r#"sub our { 1 }"#);
}

#[test]
fn test_sub_named_local() {
    assert_no_diagnostics(r#"sub local { 1 }"#);
}

#[test]
fn test_sub_named_no() {
    assert_no_diagnostics(r#"sub no { 1 }"#);
}

#[test]
fn test_sub_named_use() {
    assert_no_diagnostics(r#"sub use { 1 }"#);
}

#[test]
fn test_sub_named_do() {
    assert_no_diagnostics(r#"sub do { 1 }"#);
}

#[test]
fn test_sub_named_redo() {
    assert_no_diagnostics(r#"sub redo { 1 }"#);
}

#[test]
fn test_sub_named_begin() {
    // Benchmark.pm: sub BEGIN { ... }
    assert_no_diagnostics(r#"sub BEGIN { 1 }"#);
}

#[test]
fn test_sub_named_end() {
    assert_no_diagnostics(r#"sub END { 1 }"#);
}

// == Subroutine call via & with keyword names ==

#[test]
fn test_ampersand_call_try() {
    // Test2/Util.pm: &try($code)
    assert_no_diagnostics(r#"my ($ok, $err) = &try($code);"#);
}

#[test]
fn test_ampersand_call_next() {
    assert_no_diagnostics(r#"&next();"#);
}

#[test]
fn test_ampersand_call_last() {
    assert_no_diagnostics(r#"&last();"#);
}

#[test]
fn test_ampersand_call_goto() {
    assert_no_diagnostics(r#"&goto($label);"#);
}

#[test]
fn test_ampersand_call_state() {
    assert_no_diagnostics(r#"&state();"#);
}

#[test]
fn test_ampersand_call_redo() {
    assert_no_diagnostics(r#"&redo();"#);
}

#[test]
fn test_ampersand_call_begin() {
    assert_no_diagnostics(r#"&BEGIN();"#);
}

#[test]
fn test_try_catch_typed_with_block_is_valid() {
    assert_no_diagnostics(
        r#"
try { risky() }
catch Git::Error::Command with { my $e = shift; warn $e; };
"#,
    );
}

#[test]
fn test_try_catch_typed_without_with_still_errors() {
    assert_has_diagnostic(
        r#"
try { risky() }
catch Git::Error::Command { my $e = shift; warn $e; };
"#,
        "Expected 'with' before catch block",
    );
}
