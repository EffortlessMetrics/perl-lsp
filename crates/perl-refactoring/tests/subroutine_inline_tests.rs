//! Tests for subroutine inlining refactoring operation.
//!
//! These tests validate the core subroutine inlining algorithm implemented in
//! `perl_refactoring::refactor::inline`. Each test exercises a distinct scenario
//! described in the spec (issue #3040).

use perl_refactoring::refactor::inline::{
    InlineAbility, InlineError, SubInliner, analyze_sub_for_inlining,
};

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
    assert!(result.is_ok(), "basic inlining should succeed: {:?}", result);
    let inlined = result.unwrap();
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
    assert!(result.is_ok(), "single-arg inlining should succeed: {:?}", result);
    let inlined = result.unwrap();
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
    assert!(result.is_ok(), "multi-arg inlining should succeed: {:?}", result);
    let inlined = result.unwrap();
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
    assert!(result.is_ok(), "side-effect sub should be inlinable (with warning): {:?}", result);
    let (_, warnings) = result.unwrap();
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
    assert!(result.is_ok(), "collision should be handled by renaming: {:?}", result);
    let inlined = result.unwrap();
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
