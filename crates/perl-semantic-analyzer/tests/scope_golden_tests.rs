//! Golden file tests for scope analysis — common Perl patterns that must produce zero false
//! positives.
//!
//! Each test represents a real-world Perl pattern and asserts that the scope analyzer emits
//! **no** `UndeclaredVariable`, `UnusedVariable`, or `UninitializedVariable` diagnostics for
//! that pattern.  These become the authoritative "does the scope analyzer understand real Perl"
//! regression suite.
//!
//! Patterns covered:
//!  1. Nested closures with captured variables
//!  2. For/foreach loop variable binding
//!  3. While/until loop conditions
//!  4. Complex conditionals (if/elsif/else)
//!  5. Ternary operator
//!  6. Multi-line string operations
//!  7. Array and hash operations
//!  8. Regular expressions with capture variables
//!  9. Subroutine with multiple return paths
//! 10. eval/die error handling

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scope_issues_strict(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let pragma_map = PragmaTracker::build(&ast);
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &pragma_map)
}

/// Return only the diagnostic kinds that constitute false positives on well-formed code.
fn false_positive_issues(issues: &[ScopeIssue]) -> Vec<&ScopeIssue> {
    issues
        .iter()
        .filter(|i| {
            matches!(
                i.kind,
                IssueKind::UndeclaredVariable
                    | IssueKind::UnusedVariable
                    | IssueKind::UninitializedVariable
            )
        })
        .collect()
}

// ===========================================================================
// Pattern 1: Nested closures with captured variables
// ===========================================================================

#[test]
fn golden_nested_closure_captures_outer_variable() -> Result<(), Box<dyn std::error::Error>> {
    // A closure that reads a variable declared in the enclosing lexical scope should
    // not produce any false-positive diagnostics.
    let code = r#"
use strict;
my $outer = 1;
my $closure = sub { return $outer + 1; };
$closure->();
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "nested closure capturing outer variable should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn golden_doubly_nested_closure_captures_outer_variable() -> Result<(), Box<dyn std::error::Error>>
{
    // Two levels of closure nesting — the innermost sub should still see variables
    // declared in the outermost scope without false positives.
    let code = r#"
use strict;
my $base = 10;
my $middle = sub {
    my $increment = 5;
    my $inner = sub { return $base + $increment; };
    return $inner->();
};
print $middle->();
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "doubly nested closure should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 2: For/foreach loop variable binding
// ===========================================================================

#[test]
fn golden_for_my_loop_variable() -> Result<(), Box<dyn std::error::Error>> {
    // The loop variable in `for my $item (@list)` must be treated as declared and used.
    let code = r#"
use strict;
my @items = (1, 2, 3);
for my $item (@items) {
    print $item;
}
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "for-my loop variable should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn golden_foreach_my_loop_variable() -> Result<(), Box<dyn std::error::Error>> {
    // `foreach my $x (...)` should bind $x as declared and used inside the body.
    let code = r#"
use strict;
foreach my $x (1..10) {
    print $x;
}
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "foreach-my loop variable should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn golden_for_default_topic_variable() -> Result<(), Box<dyn std::error::Error>> {
    // `for (1..10) { print $_; }` uses the default topic variable $_.
    // The topic variable must never be flagged as undeclared.
    let code = r#"
use strict;
for (1..10) {
    print $_;
}
"#;
    let issues = scope_issues_strict(code);
    let undeclared_topic = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains('_'))
        .count();
    assert_eq!(
        undeclared_topic,
        0,
        "topic variable $_ inside for loop must not be undeclared; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 3: While/until loop conditions
// ===========================================================================

#[test]
fn golden_while_loop_condition_variable() -> Result<(), Box<dyn std::error::Error>> {
    // Variables used in a while-loop condition and body must not be false-positives.
    let code = r#"
use strict;
my $count = 0;
while ($count < 10) {
    $count++;
}
print $count;
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "while loop with declared condition variable should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn golden_until_loop_condition_variable() -> Result<(), Box<dyn std::error::Error>> {
    // Variables used in an until-loop condition and body must not be false-positives.
    let code = r#"
use strict;
my $count = 0;
until ($count >= 20) {
    $count++;
}
print $count;
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "until loop with declared condition variable should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 4: Complex conditionals (if/elsif/else)
// ===========================================================================

#[test]
fn golden_if_elsif_else_variables() -> Result<(), Box<dyn std::error::Error>> {
    // Variables used across if/elsif/else branches must not be false-positives.
    let code = r#"
use strict;
my $x = 1;
my $y = 2;
if ($x > 0) {
    print $x;
} elsif ($y > 0) {
    print $y;
} else {
    print "neither";
}
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "if/elsif/else with declared variables should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 5: Ternary operator
// ===========================================================================

#[test]
fn golden_ternary_operator_variables() -> Result<(), Box<dyn std::error::Error>> {
    // Variables in a ternary expression (condition, true-branch, false-branch) must
    // not produce false positives.
    let code = r#"
use strict;
my $val = 1;
my $result = $val > 0 ? "positive" : "non-positive";
print $result;
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "ternary expression variables should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 6: Multi-line string operations
// ===========================================================================

#[test]
fn golden_string_function_results_used_in_interpolation() -> Result<(), Box<dyn std::error::Error>>
{
    // Variables holding the result of uc/length and then used inside a double-quoted
    // string interpolation must not be false-positives.
    let code = r#"
use strict;
my $str = "hello";
my $upper = uc($str);
my $len = length($str);
print "$upper has $len chars";
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "string-operation variables used in interpolation should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 7: Array and hash operations
// ===========================================================================

#[test]
fn golden_array_push_shift_operations() -> Result<(), Box<dyn std::error::Error>> {
    // Variables used with push/shift array operations must not be false-positives.
    let code = r#"
use strict;
my @arr = (1, 2, 3);
push @arr, 4;
my $first = shift @arr;
print $first;
print @arr;
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "array push/shift operations should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn golden_hash_assignment_and_delete() -> Result<(), Box<dyn std::error::Error>> {
    // Variables used with hash element assignment and delete must not be false-positives.
    let code = r#"
use strict;
my %hash = (a => 1);
$hash{b} = 2;
delete $hash{a};
print %hash;
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "hash assignment and delete should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 8: Regular expressions with capture variables
// ===========================================================================

#[test]
fn golden_regex_capture_variables_in_if_block() -> Result<(), Box<dyn std::error::Error>> {
    // $1, $2, $3 populated by a regex match inside an if-condition must not be
    // undeclared inside the if-block body.
    let code = r#"
use strict;
my $text = "2024-01-15";
if ($text =~ /(\d{4})-(\d{2})-(\d{2})/) {
    my ($year, $month, $day) = ($1, $2, $3);
    print "$year/$month/$day";
}
"#;
    let issues = scope_issues_strict(code);
    // $1, $2, $3 are Perl magic variables — they must never be reported as undeclared.
    let undeclared_captures: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UndeclaredVariable
                && (i.variable_name == "$1"
                    || i.variable_name == "$2"
                    || i.variable_name == "$3"
                    || i.variable_name == "1"
                    || i.variable_name == "2"
                    || i.variable_name == "3")
        })
        .collect();
    assert!(
        undeclared_captures.is_empty(),
        "regex capture variables $1/$2/$3 must not be undeclared; got: {:?}",
        undeclared_captures
    );

    // $year, $month, $day should also not be false-positives.
    let fp_named: Vec<_> = issues
        .iter()
        .filter(|i| {
            matches!(i.kind, IssueKind::UnusedVariable | IssueKind::UninitializedVariable)
                && (i.variable_name.contains("year")
                    || i.variable_name.contains("month")
                    || i.variable_name.contains("day"))
        })
        .collect();
    assert!(
        fp_named.is_empty(),
        "named capture variables should not be false-positives; got: {:?}",
        fp_named
    );
    Ok(())
}

// ===========================================================================
// Pattern 9: Subroutine with multiple return paths
// ===========================================================================

#[test]
fn golden_sub_multiple_return_paths() -> Result<(), Box<dyn std::error::Error>> {
    // A subroutine with three early-return paths using `return X if COND;` syntax
    // should produce no false positives on the parameter or local variables.
    let code = r#"
use strict;
sub classify {
    my ($n) = @_;
    return "negative" if $n < 0;
    return "zero" if $n == 0;
    return "positive";
}
print classify(5);
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "sub with multiple return paths should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn golden_sub_parameter_destructuring() -> Result<(), Box<dyn std::error::Error>> {
    // `my ($a, $b) = @_` destructuring in a subroutine must not produce false positives.
    // Note: $a and $b are also Perl sort globals, but a lexical `my` declaration
    // creates a new binding that shadows the global and must be tracked as used.
    let code = r#"
use strict;
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}
print add(1, 2);
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "sub parameter destructuring should produce zero false positives; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Pattern 10: eval/die error handling
// ===========================================================================

#[test]
fn golden_eval_die_error_variable() -> Result<(), Box<dyn std::error::Error>> {
    // `eval { die ... }; if ($@) { ... }` is canonical Perl error handling.
    // $@ is a Perl magic variable — it must never be undeclared.
    let code = r#"
use strict;
eval { die "test error" };
if ($@) {
    warn "caught: $@";
}
"#;
    let issues = scope_issues_strict(code);
    let undeclared_eval_err: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UndeclaredVariable
                && (i.variable_name == "$@" || i.variable_name == "@")
        })
        .collect();
    assert!(
        undeclared_eval_err.is_empty(),
        "eval error variable $@ must not be undeclared; got: {:?}",
        undeclared_eval_err
    );
    Ok(())
}

#[test]
fn golden_eval_block_local_variable() -> Result<(), Box<dyn std::error::Error>> {
    // Variables declared inside an eval block must be accessible within it without
    // false positives.
    let code = r#"
use strict;
my $result;
eval {
    my $tmp = 42;
    $result = $tmp * 2;
};
print $result;
"#;
    let issues = scope_issues_strict(code);
    // $result is declared outside eval, assigned inside, used after — must not be false-positive.
    let fp_result: Vec<_> = issues
        .iter()
        .filter(|i| {
            matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UninitializedVariable)
                && i.variable_name.contains("result")
        })
        .collect();
    assert!(
        fp_result.is_empty(),
        "$result assigned inside eval must not be undeclared/uninitialized outside; got: {:?}",
        fp_result
    );
    Ok(())
}

// ===========================================================================
// Pattern: print with comma-separated argument list
// ===========================================================================

#[test]
fn golden_print_comma_separated_args_all_marked_used() -> Result<(), Box<dyn std::error::Error>> {
    // Variables that appear as comma-separated arguments to `print` must be marked as used.
    // Regression test for issue #3503: print $greeting, " ", $name, "\n" was producing
    // UnusedVariable diagnostics for $greeting and $name.
    let code = r#"
use strict;
my $name = "world";
my $greeting = "hello";
print $greeting, " ", $name, "\n";
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "variables in print comma-separated arg list must be marked as used; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn golden_say_comma_separated_args_all_marked_used() -> Result<(), Box<dyn std::error::Error>> {
    // Same as above but for `say` (adds newline automatically).
    let code = r#"
use strict;
my $a = "first";
my $b = "second";
my $c = "third";
say $a, $b, $c;
"#;
    let issues = scope_issues_strict(code);
    let fp = false_positive_issues(&issues);
    assert!(
        fp.is_empty(),
        "variables in say comma-separated arg list must be marked as used; got: {:?}",
        fp.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}
