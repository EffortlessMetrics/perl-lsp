//! Tests for capture variable context tracking in scope analysis.
//!
//! Covers:
//! - `$1`, `$2`, etc. used without a preceding regex match → warn
//! - `$1`, `$2`, etc. used after `=~` in the same scope → no warn
//! - `$1`, `$2`, etc. used inside an `if` block after `=~` → no warn
//! - Nested scopes inherit regex-match context from parent scope
//! - Standalone `m//` (matches `$_`) counts as regex match context
//! - `s///` substitution counts as regex match context
//! - Multi-capture group variables (`$1`, `$2`, `$3`) all covered

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scope_issues(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &[])
}

fn has_issue(issues: &[ScopeIssue], kind: IssueKind, var_name: &str) -> bool {
    issues.iter().any(|i| i.kind == kind && i.variable_name.contains(var_name))
}

fn count_of_kind(issues: &[ScopeIssue], kind: IssueKind) -> usize {
    issues.iter().filter(|i| i.kind == kind).count()
}

// ===========================================================================
// 1. Capture variable used WITHOUT any prior regex match → should warn
// ===========================================================================

#[test]
fn capture_var_no_regex_warns() -> Result<(), Box<dyn std::error::Error>> {
    // $1 used with no regex match anywhere in scope — should produce a diagnostic
    let code = r#"my $x = $1;"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Expected CaptureVarWithoutRegexMatch for $1 with no regex; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn capture_var_two_no_regex_warns() -> Result<(), Box<dyn std::error::Error>> {
    // $2 used with no regex match — should also warn
    let code = r#"my $y = $2;"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "2"),
        "Expected CaptureVarWithoutRegexMatch for $2; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 2. Capture variable used AFTER `=~` match → no warn
// ===========================================================================

#[test]
fn capture_var_after_match_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // $1 used after =~ match in same scope — should NOT warn
    let code = r#"
my $str = "hello world";
$str =~ /(\w+)/;
my $y = $1;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Should NOT warn about $1 when =~ match precedes it; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn capture_var_inside_if_after_match_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // $1 used inside an if block whose condition performs the match — no warn
    let code = r#"
my $str = "hello";
if ($str =~ /(\w+)/) {
    my $y = $1;
}
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Should NOT warn about $1 inside if-block with =~ match condition; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 3. Standalone `m//` (matches `$_`) counts as regex match context
// ===========================================================================

#[test]
fn capture_var_after_standalone_match_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // m// matches $_ and sets capture vars — no warn after
    let code = r#"
$_ = "hello";
m/(\w+)/;
my $z = $1;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Should NOT warn about $1 after standalone m//; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 4. `s///` substitution counts as regex match context
// ===========================================================================

#[test]
fn capture_var_after_substitution_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // s/// also sets $1 etc. — no warn after
    let code = r#"
my $str = "hello world";
$str =~ s/(\w+)/replaced/;
my $matched = $1;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Should NOT warn about $1 after s///; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 5. Multiple capture variables ($1, $2, $3)
// ===========================================================================

#[test]
fn multiple_capture_vars_no_regex_warns() -> Result<(), Box<dyn std::error::Error>> {
    // $1 and $2 used without any regex — both should warn
    let code = r#"
my $a = $1;
my $b = $2;
"#;
    let issues = scope_issues(code);
    let count = count_of_kind(&issues, IssueKind::CaptureVarWithoutRegexMatch);
    assert_eq!(
        count,
        2,
        "Expected 2 CaptureVarWithoutRegexMatch issues (one for $1, one for $2); issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn multiple_capture_vars_after_match_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // $1 and $2 used after =~ match — neither should warn
    let code = r#"
my $str = "hello world";
$str =~ /(\w+)\s+(\w+)/;
my $a = $1;
my $b = $2;
"#;
    let issues = scope_issues(code);
    let count = count_of_kind(&issues, IssueKind::CaptureVarWithoutRegexMatch);
    assert_eq!(
        count,
        0,
        "Should NOT warn about $1 or $2 after =~ match; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 6. Nested scope inherits regex-match context from parent
// ===========================================================================

#[test]
fn capture_var_in_nested_block_after_outer_match_no_warn() -> Result<(), Box<dyn std::error::Error>>
{
    // Match in outer scope, $1 used in nested block — should not warn
    let code = r#"
my $str = "hello";
$str =~ /(\w+)/;
{
    my $inner = $1;
}
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Nested block should inherit regex-match context from outer scope; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 7. Capture var before match in same scope → warns
// ===========================================================================

#[test]
fn capture_var_before_match_in_scope_warns() -> Result<(), Box<dyn std::error::Error>> {
    // $1 used BEFORE the regex match — should warn because match hasn't happened yet
    let code = r#"
my $str = "hello";
my $early = $1;
$str =~ /(\w+)/;
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Should warn about $1 used before the regex match in same scope; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 8. $0 is the program name, NOT a capture variable — never warn
// ===========================================================================

#[test]
fn dollar_zero_is_not_capture_var_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // $0 is the program name; it should never trigger a capture-var warning
    let code = r#"my $prog = $0;"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "0"),
        "$0 is program name, not a capture variable; should not warn; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 9. String interpolation edge case (Phase 1 limitation)
// ===========================================================================

#[test]
fn capture_var_in_double_quoted_string_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // Phase 1 limitation: $1 interpolated inside a double-quoted string is NOT visited as
    // a Variable node by the scope analyzer — string interpolation nodes are not walked for
    // capture-variable checks. No warning is produced in this case.
    // Future: string interpolation nodes could be walked to catch this pattern.
    let code = r#"my $str = "matched: $1";"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Phase 1 limitation: interpolated $1 in string not currently checked; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn capture_var_in_string_after_match_no_warn() -> Result<(), Box<dyn std::error::Error>> {
    // Even in a string, if a regex match has occurred in scope, no warn
    let code = r#"
my $str = "hello";
$str =~ /(\w+)/;
my $msg = "matched: $1";
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::CaptureVarWithoutRegexMatch, "1"),
        "Should NOT warn about $1 in string after regex match in scope; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}
