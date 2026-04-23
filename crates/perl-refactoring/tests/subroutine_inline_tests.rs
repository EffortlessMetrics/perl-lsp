//! Tests for subroutine inlining refactoring operation.
//!
//! These tests validate the core subroutine inlining algorithm implemented in
//! `perl_refactoring::refactor::inline`. Each test exercises a distinct scenario
//! described in the spec (issue #3040).

use perl_refactoring::refactor::inline::{
    InlineAbility, InlineError, SubInliner, analyze_sub_for_inlining,
};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Basic inlining
// ---------------------------------------------------------------------------

#[test]
fn test_basic_sub_inlining_replaces_call_with_body() {
    // sub calculate_tax { my ($price, $rate) = @_; return $price * $rate; }
    // my $total = calculate_tax(100, 0.15);
    // => my $total = (100 * 0.15);
    let source = r#"sub calculate_tax {
    my ($price, $rate) = @_;
    return $price * $rate;
}

my $total = calculate_tax(100, 0.15);
"#;

    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("calculate_tax", "calculate_tax(100, 0.15)");
    let inlined = must(result);
    assert!(
        inlined.contains("100 * 0.15") || inlined.contains("(100 * 0.15)"),
        "inlined result should contain the substituted body, got: {inlined}"
    );
}

#[test]
fn test_inlining_substitutes_parameters_with_arguments() {
    let source = r#"sub double {
    my ($x) = @_;
    return $x * 2;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("double", "double(5)");
    let inlined = must(result);
    assert!(
        inlined.contains("5 * 2") || inlined.contains("(5 * 2)"),
        "parameter $x should be replaced with 5, got: {inlined}"
    );
}

#[test]
fn test_inlining_multiple_parameters() {
    let source = r#"sub add {
    my ($a, $b) = @_;
    return $a + $b;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("add", "add(3, 4)");
    let inlined = must(result);
    assert!(
        inlined.contains("3 + 4") || inlined.contains("(3 + 4)"),
        "both parameters should be substituted, got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Edge case: recursion rejection
// ---------------------------------------------------------------------------

#[test]
fn test_recursive_sub_is_rejected() {
    let source = r#"sub factorial {
    my ($n) = @_;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("factorial", "factorial(5)");
    assert!(
        matches!(result, Err(InlineError::Recursive { .. })),
        "recursive sub should be rejected with Recursive error, got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Edge case: large body rejection
// ---------------------------------------------------------------------------

#[test]
fn test_large_sub_is_rejected() {
    // Build a sub with >50 lines of body
    let mut body_lines = String::new();
    for i in 0..55 {
        body_lines.push_str(&format!("    my $v{i} = {i};\n"));
    }
    let source = format!("sub big_sub {{\n{body_lines}    return 1;\n}}\n");
    let inliner = SubInliner::new(&source);
    let result = inliner.inline_call("big_sub", "big_sub()");
    assert!(
        matches!(result, Err(InlineError::TooLarge { line_count, .. }) if line_count > 50),
        "large sub should be rejected with TooLarge error, got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Edge case: side effects warn but are allowed
// ---------------------------------------------------------------------------

#[test]
fn test_side_effect_sub_returns_warning() {
    let source = r#"sub greet {
    my ($name) = @_;
    print "Hello, $name!\n";
    return 1;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call_with_warnings("greet", "greet(\"World\")");
    let (_, warnings) = must(result);
    assert!(!warnings.is_empty(), "side-effect sub should produce warnings");
}

// ---------------------------------------------------------------------------
// Edge case: multiple return points
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_returns_rejected() {
    let source = r#"sub classify {
    my ($n) = @_;
    if ($n > 0) {
        return "positive";
    }
    return "non-positive";
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("classify", "classify(5)");
    assert!(
        matches!(result, Err(InlineError::MultipleReturns { count, .. }) if count > 1),
        "sub with multiple returns should be rejected, got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Edge case: variable name collision
// ---------------------------------------------------------------------------

#[test]
fn test_variable_collision_is_renamed() {
    // Both the outer scope and the sub use $result — collision must be resolved
    let source = r#"sub compute {
    my ($x) = @_;
    my $result = $x * 2;
    return $result;
}
"#;
    let inliner = SubInliner::new(source);
    // outer_vars simulates variables that exist in the call-site scope
    let result =
        inliner.inline_call_with_outer_vars("compute", "compute(7)", &["$result".to_string()]);
    let inlined = must(result);
    // The inline should NOT use $result verbatim if it collides
    assert!(
        !inlined.contains("my $result ="),
        "colliding $result should be renamed in inlined code, got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// analyze_sub_for_inlining helper
// ---------------------------------------------------------------------------

#[test]
fn test_analyze_simple_sub_is_inlineable() {
    let source = r#"sub add {
    my ($a, $b) = @_;
    return $a + $b;
}
"#;
    let analysis = analyze_sub_for_inlining(source, "add");
    assert!(
        matches!(analysis, Ok(InlineAbility::Ok { .. })),
        "simple sub should be inlineable, got: {:?}",
        analysis
    );
}

#[test]
fn test_analyze_sub_not_found_returns_error() {
    let source = "sub foo { return 1; }\n";
    let analysis = analyze_sub_for_inlining(source, "nonexistent");
    assert!(
        matches!(analysis, Err(InlineError::SubNotFound { .. })),
        "missing sub should return SubNotFound, got: {:?}",
        analysis
    );
}

// ---------------------------------------------------------------------------
// Edge case: partial variable name match in parameter substitution
// ---------------------------------------------------------------------------

#[test]
fn test_param_substitution_does_not_corrupt_longer_var_names() {
    // If the sub has param $n, and the body uses $name (a different variable
    // that starts with the same prefix), substituting $n -> arg must NOT
    // corrupt $name.  Previously this used naive str::replace which would
    // turn "$name" into "42ame".
    let source = r#"sub greet_n {
    my ($n, $name) = @_;
    return $name x $n;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("greet_n", "greet_n(3, \"hi\")");
    let inlined = must(result);
    // $name should NOT become "42ame" or corrupt form
    assert!(
        !inlined.contains("\"hi\"ame"),
        "param $n substitution must not corrupt variable $name; got: {inlined}"
    );
    // $n and $name should both be correctly substituted
    assert!(
        inlined.contains("\"hi\" x 3") || inlined.contains("(\"hi\" x 3)"),
        "both params must be substituted correctly; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Edge case: collision rename must not break subsequent param substitution
// ---------------------------------------------------------------------------

#[test]
fn test_collision_rename_does_not_break_param_substitution() {
    // Sub has param $x. Outer scope also has $x.
    // rename_collisions renames the local $x declaration to $x_inlined.
    // substitute_params must then correctly substitute the param $x with the
    // call-site argument WITHOUT corrupting $x_inlined (since the renamed
    // variable name now has the original as a prefix).
    //
    // This tests that replace_whole_var (word-boundary-aware) is used for
    // param substitution rather than naive str::replace.
    let source = r#"sub add_one {
    my ($x) = @_;
    return $x + 1;
}
"#;
    let inliner = SubInliner::new(source);
    // outer_vars includes $x, so rename_collisions will rename $x -> $x_inlined
    // in the body.  After renaming the body no longer contains param $x, so
    // substitute_params should find nothing to replace — the result reflects
    // the renamed form.  Crucially, it must NOT produce "7_inlined + 1".
    let result = inliner.inline_call_with_outer_vars("add_one", "add_one(7)", &["$x".to_string()]);
    let inlined = must(result);
    // The inlined result must NEVER contain "7_inlined" — that would mean the
    // word-boundary check failed and "$x_inlined" was corrupted by a naive
    // replace of "$x" -> "7".
    assert!(
        !inlined.contains("7_inlined"),
        "param substitution must not corrupt $x_inlined after collision rename; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Edge case: return keyword inside a string literal is not counted
// ---------------------------------------------------------------------------

#[test]
fn test_return_in_string_literal_not_counted_as_return_statement() {
    // The word "return" appears in a string, not as a control-flow statement.
    // This sub has only 1 real return statement.
    let source = r#"sub describe {
    my ($x) = @_;
    my $msg = "will return a value";
    return $x + 1;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("describe", "describe(3)");
    // Should succeed (not rejected as MultipleReturns)
    assert!(
        result.is_ok(),
        "sub with 'return' in string should not be rejected as MultipleReturns; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Edge case: collision rename declaration must not corrupt sibling variables
// ---------------------------------------------------------------------------

#[test]
fn test_collision_rename_decl_does_not_corrupt_sibling_variables() {
    // Sub body has BOTH $x and $x_count as local variables.
    // Outer scope has $x (a collision).
    // rename_collisions must rename only "my $x" to "my $x_inlined", and must
    // NOT mangle the unrelated "my $x_count" declaration.
    //
    // Previously, the declaration-replacement step used str::replace which is
    // not word-boundary-aware: replace("my $x", "my $x_inlined") on the text
    // "my $x_count = 3;\n    my $x = 5;" would produce
    // "my $x_inlined_count = 3;\n    my $x_inlined = 5;" — corrupting $x_count.
    let source = r#"sub compute {
    my ($a) = @_;
    my $x_count = 3;
    my $x = $a * 2;
    return $x * $x_count;
}
"#;
    let inliner = SubInliner::new(source);
    // $x collides with the outer scope
    let result = inliner.inline_call_with_outer_vars("compute", "compute(7)", &["$x".to_string()]);
    let inlined = must(result);
    // $x_count must NEVER be renamed to $x_inlined_count
    assert!(
        !inlined.contains("x_inlined_count"),
        "collision rename must not corrupt sibling variable $x_count; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Edge case: recursion detection must not fire on sub name in string literals
// ---------------------------------------------------------------------------

#[test]
fn test_recursion_detection_ignores_sub_name_in_string() {
    // The sub body contains the sub name inside a string literal.
    // body_calls_self must not treat this as actual recursion.
    let source = r#"sub add {
    my ($a, $b) = @_;
    my $msg = "add(a,b) adds two numbers";
    return $a + $b;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("add", "add(1, 2)");
    assert!(
        result.is_ok(),
        "sub name in a string literal must not trigger Recursive rejection; got: {:?}",
        result
    );
    let inlined = must(result);
    assert!(
        inlined.contains("1 + 2") || inlined.contains("(1 + 2)"),
        "inlined result should contain the substituted expression; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Edge case: call argument containing a closing paren inside a string
// ---------------------------------------------------------------------------

#[test]
fn test_inline_call_with_paren_in_string_argument() {
    // The second argument is a string that contains ')' — the call-site parser
    // must correctly identify the real closing paren, not the one inside the string.
    let source = r#"sub greet {
    my ($prefix, $name) = @_;
    return $prefix . $name;
}
"#;
    let inliner = SubInliner::new(source);
    // Second argument "hello)" contains ')' — naive paren-matching would split here
    let result = inliner.inline_call("greet", r#"greet("Hi)", "World")"#);
    assert!(
        result.is_ok(),
        "call with ')' inside a string argument must not fail; got: {:?}",
        result
    );
}
