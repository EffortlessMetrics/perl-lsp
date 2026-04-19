//! Tests for implicit $_ handling in topic-variable builtins (issue #3458).
//!
//! Builtins like chomp, chop, length, lc, uc, etc. operate on $_ by default
//! when called with zero arguments. The scope analyzer must treat zero-arg
//! calls to these builtins as a use (and initialization) of $_.
//!
//! This prevents false "uninitialized variable" or "unused variable" diagnostics
//! when the idiomatic Perl pattern of implicit-$_ is used.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
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

fn scope_issues_strict(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let pragma_map = PragmaTracker::build(&ast);
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &pragma_map)
}

fn has_issue(issues: &[ScopeIssue], kind: IssueKind, var_name: &str) -> bool {
    issues
        .iter()
        .any(|i| i.kind == kind && i.variable_name.contains(var_name))
}

fn no_diagnostic_for(issues: &[ScopeIssue], var_name: &str) -> bool {
    !issues.iter().any(|i| i.variable_name.contains(var_name))
}

// ===========================================================================
// 1. chomp with no args — uses $_ implicitly
// ===========================================================================

/// `chomp;` with no args should not produce any diagnostic about $_.
/// The implicit topic variable is valid Perl and must not be flagged.
#[test]
fn test_chomp_no_args_no_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use warnings;

while (<STDIN>) {
    chomp;
    print $_;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        no_diagnostic_for(&issues, "_"),
        "chomp; should not produce any diagnostic about $_; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// `chomp;` inside a while loop should produce no issues.
#[test]
fn test_chomp_implicit_topic_in_while_loop() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
while (my $line = <STDIN>) {
    chomp;
    print length;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UninitializedVariable, "_"),
        "chomp; should not produce UninitializedVariable for $_; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 2. chop with no args — uses $_ implicitly
// ===========================================================================

/// `chop;` with no args should not produce any diagnostic about $_.
#[test]
fn test_chop_no_args_no_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
for (@ARGV) {
    chop;
    print;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        no_diagnostic_for(&issues, "_"),
        "chop; should not produce any diagnostic about $_; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 3. length with no args — uses $_ implicitly
// ===========================================================================

/// `length` with no args operates on $_ and must not produce a diagnostic.
#[test]
fn test_length_no_args_no_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
for (@ARGV) {
    my $n = length;
    print "$n\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        no_diagnostic_for(&issues, "_"),
        "length with no args should not produce any diagnostic about $_; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 4. lc/uc/lcfirst/ucfirst with no args — all use $_ implicitly
// ===========================================================================

/// `lc` and related string case functions with no args use $_ implicitly.
#[test]
fn test_lc_no_args_no_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
for (@ARGV) {
    my $lower = lc;
    my $upper = uc;
    print "$lower $upper\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        no_diagnostic_for(&issues, "_"),
        "lc/uc with no args should not produce any diagnostic about $_; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 5. Explicit $_ declaration — chomp; marks it as used
// ===========================================================================

/// When $_ is explicitly declared with `my $_ = ...`, a bare `chomp;`
/// should count as a use of $_, preventing UnusedVariable diagnostic.
/// The $_ variable is only "used" by chomp here — no explicit $_ reference follows.
#[test]
fn test_chomp_marks_explicit_topic_as_used() -> Result<(), Box<dyn std::error::Error>> {
    // The only use of $_ here is via chomp; — the implicit topic.
    // If chomp; doesn't mark $_ as used, UnusedVariable fires.
    let code = r#"
my $_ = "hello\n";
chomp;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "_"),
        "chomp; on explicit my $_ should mark it used (no explicit $_ reference); issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// chop; with explicit my $_ — bare chop should mark $_ as used.
#[test]
fn test_chop_marks_explicit_topic_as_used() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $_ = "hello";
chop;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "_"),
        "chop; on explicit my $_ should mark it used; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// When $_ is declared uninitialized and chomp; is called (zero args),
/// chomp reads/modifies $_, which counts as a use. After chomp, using $_
/// should not produce UninitializedVariable.
#[test]
fn test_chomp_zero_args_marks_topic_used_not_uninitialized()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $_;
chomp;
print $_;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::UninitializedVariable, "_"),
        "after chomp; $_ should not be reported as uninitialized; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 6. Builtins WITH explicit args still work normally
// ===========================================================================

/// `chomp $line;` with an explicit arg should still work fine.
/// Regression: explicit-arg form must not be broken by the fix.
#[test]
fn test_chomp_with_explicit_arg_still_works() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my $line = "hello\n";
chomp $line;
print $line;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "line"),
        "chomp $line should mark $line used; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    assert!(
        !has_issue(&issues, IssueKind::UninitializedVariable, "line"),
        "chomp $line should not report $line as uninitialized; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 7. Full idiomatic Perl IO loop
// ===========================================================================

/// Idiomatic Perl IO pattern: `while (<STDIN>) { chomp; ... }` must be clean.
#[test]
fn test_idiomatic_io_loop_no_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use warnings;

while (<STDIN>) {
    chomp;
    chop;
    my $n = length;
    my $lower = lc;
    my $upper = uc;
    print "$lower $upper $n\n";
}
"#;
    let issues = scope_issues_strict(code);
    let topic_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.variable_name.contains('_'))
        .collect();
    assert!(
        topic_issues.is_empty(),
        "idiomatic IO loop should produce no $_ diagnostics; found: {:?}",
        topic_issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 8. ord, chr, hex, oct, abs, int — all default to $_
// ===========================================================================

/// Other numeric/conversion builtins that default to $_ should also
/// produce no diagnostic when called without arguments.
#[test]
fn test_numeric_builtins_no_args_no_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
for (@ARGV) {
    my $o = ord;
    my $h = hex;
    my $oc = oct;
    my $a = abs;
    my $i = int;
    print "$o $h $oc $a $i\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        no_diagnostic_for(&issues, "_"),
        "numeric builtins with no args should not produce any diagnostic about $_; issues: {:?}",
        issues
            .iter()
            .map(|i| (&i.kind, &i.variable_name))
            .collect::<Vec<_>>()
    );
    Ok(())
}
