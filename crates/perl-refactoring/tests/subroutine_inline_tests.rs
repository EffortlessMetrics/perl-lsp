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

#[test]
fn test_inline_sub_with_signature_parens() {
    // Perl signatures can appear between the sub name and body braces.
    // The inliner should still locate and inline this sub.
    let source = r#"sub combine ($left, $right) {
    my ($left, $right) = @_;
    return $left . $right;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("combine", r#"combine("a", "b")"#);
    let inlined = must(result);
    assert!(
        inlined.contains("\"a\" . \"b\"") || inlined.contains("(\"a\" . \"b\")"),
        "signature-style sub should inline normally; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Edge case: call argument containing a closing paren inside a string
// ---------------------------------------------------------------------------

#[test]
fn test_inline_call_with_paren_in_string_argument() {
    // The first argument is a string that contains ')' — the call-site parser
    // must correctly identify the real closing paren, not the one inside the string.
    let source = r#"sub greet {
    my ($prefix, $name) = @_;
    return $prefix . $name;
}
"#;
    let inliner = SubInliner::new(source);
    // First argument "Hi)" contains ')' — naive paren-matching would split here
    let result = inliner.inline_call("greet", r#"greet("Hi)", "World")"#);
    let inlined = must(result);
    assert!(
        inlined.contains(r#""Hi)" . "World""#),
        "call with ')' inside a string argument must preserve both arguments; got: {inlined}"
    );
}

#[test]
fn test_sub_body_with_brace_in_string_literal_inlines_full_body() {
    // A closing brace inside a string literal is not the end of the sub body.
    // The body parser must continue to the real brace so the return statement
    // remains available for inlining.
    let source = r#"sub describe_brace {
    my ($value) = @_;
    my $template = "literal } brace";
    return $template . $value;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("describe_brace", r#"describe_brace("tail")"#);
    let inlined = must(result);
    assert!(
        inlined.contains(r#"$template . "tail""#),
        "brace inside a string literal must not truncate the parsed sub body; got: {inlined}"
    );
}

#[test]
fn test_sub_body_with_brace_in_line_comment_inlines_full_body() {
    let source = r#"sub describe_commented_brace {
    # A closing brace in a comment: }
    return "ok";
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("describe_commented_brace", "describe_commented_brace()");
    let inlined = must(result);
    assert!(
        inlined.contains(r#""ok""#),
        "brace inside a line comment must not truncate the parsed sub body; got: {inlined}"
    );
}

#[test]
fn test_sub_body_with_hash_in_regex_on_one_line_inlines_full_body() {
    let source = r#"sub hash_regex { return qr/#/; }"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("hash_regex", "hash_regex()");
    let inlined = must(result);
    assert!(
        inlined.contains("qr/#/"),
        "hash inside a regex literal must not be mistaken for a line comment; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// InlineError Display formatting — covers all five variants
// ---------------------------------------------------------------------------

#[test]
fn test_display_sub_not_found_includes_name() {
    let err = InlineError::SubNotFound { name: "missing".to_string() };
    let msg = format!("{err}");
    assert!(
        msg.contains("missing") && msg.contains("not found"),
        "SubNotFound Display should mention the name and 'not found'; got: {msg}"
    );
}

#[test]
fn test_display_recursive_includes_name() {
    let err = InlineError::Recursive { name: "rec".to_string() };
    let msg = format!("{err}");
    assert!(
        msg.contains("rec") && msg.contains("recursive"),
        "Recursive Display should mention the name and 'recursive'; got: {msg}"
    );
}

#[test]
fn test_display_too_large_includes_line_count() {
    let err = InlineError::TooLarge { name: "big".to_string(), line_count: 123 };
    let msg = format!("{err}");
    assert!(
        msg.contains("big") && msg.contains("123"),
        "TooLarge Display should include sub name and line count; got: {msg}"
    );
}

#[test]
fn test_display_multiple_returns_includes_count() {
    let err = InlineError::MultipleReturns { name: "branch".to_string(), count: 3 };
    let msg = format!("{err}");
    assert!(
        msg.contains("branch") && msg.contains("3"),
        "MultipleReturns Display should include sub name and count; got: {msg}"
    );
}

#[test]
fn test_display_call_site_parse_failed_includes_message() {
    let err = InlineError::CallSiteParseFailed { message: "boom".to_string() };
    let msg = format!("{err}");
    assert!(
        msg.contains("boom") && msg.contains("call site"),
        "CallSiteParseFailed Display should include the diagnostic; got: {msg}"
    );
}

#[test]
fn test_inline_error_is_std_error() {
    // The std::error::Error impl is intentionally trait-only; this test confirms
    // the bound holds so callers can use the error in `Box<dyn Error>` chains.
    fn assert_error<E: std::error::Error>(_: &E) {}
    let err = InlineError::SubNotFound { name: "x".to_string() };
    assert_error(&err);
}

// ---------------------------------------------------------------------------
// CallSiteParseFailed paths — both failure modes
// ---------------------------------------------------------------------------

#[test]
fn test_call_expression_missing_sub_name_returns_parse_failed() {
    // The call expression does not contain the sub name at all — extract_call_args
    // must return CallSiteParseFailed rather than panicking or silently succeeding.
    let source = r#"sub add {
    my ($a, $b) = @_;
    return $a + $b;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("add", "wrong_name(1, 2)");
    assert!(
        matches!(result, Err(InlineError::CallSiteParseFailed { .. })),
        "call expr missing sub name should return CallSiteParseFailed; got: {:?}",
        result
    );
}

#[test]
fn test_call_expression_unmatched_paren_returns_parse_failed() {
    // The call expression has '(' but no closing ')' — find_matching_paren returns
    // None and extract_call_args surfaces CallSiteParseFailed.
    let source = r#"sub add {
    my ($a, $b) = @_;
    return $a + $b;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("add", "add(1, 2");
    assert!(
        matches!(result, Err(InlineError::CallSiteParseFailed { .. })),
        "unmatched paren should return CallSiteParseFailed; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Edge case: reference dereferences must not be corrupted
// ---------------------------------------------------------------------------

#[test]
fn test_inlining_does_not_corrupt_scalar_deref() {
    let source = r#"sub first_elem {
    my ($ref) = @_;
    return $$ref;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("first_elem", "first_elem($value_ref)");
    let inlined = must(result);
    assert!(
        inlined.contains("${$value_ref}"),
        "dereference parameter should be braced during substitution; got: {inlined}"
    );
    assert!(
        !inlined.contains("$$value_ref"),
        "replacing $ref in $$ref must not produce unbraced $$value_ref; got: {inlined}"
    );
    assert!(
        !inlined.contains("$$ref"),
        "inlined output must not leave the original parameter reference behind; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Call argument shapes — bare call, empty parens, nested parens, quoted commas
// ---------------------------------------------------------------------------

#[test]
fn test_bare_call_without_parens_treated_as_no_args() {
    // A bare call expression without parens (e.g. `now`) should be treated as
    // a zero-argument call rather than a parse error.
    let source = r#"sub now {
    return time();
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("now", "now");
    assert!(
        result.is_ok(),
        "bare call without parens should be treated as zero-arg call; got: {:?}",
        result
    );
}

#[test]
fn test_empty_parens_call_treated_as_no_args() {
    // `foo()` should produce no args without crashing or returning extra empty args.
    let source = r#"sub greet {
    return "hi";
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("greet", "greet()");
    let inlined = must(result);
    assert!(
        inlined.contains("\"hi\""),
        "empty-parens call should inline the return expression; got: {inlined}"
    );
}

#[test]
fn test_nested_paren_argument_is_kept_intact() {
    // Argument contains nested parens — split_args must track paren depth so the
    // single argument is not split at the inner comma.
    let source = r#"sub passthrough {
    my ($x) = @_;
    return $x;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("passthrough", "passthrough(foo(1, 2))");
    let inlined = must(result);
    assert!(
        inlined.contains("foo(1, 2)"),
        "nested-paren argument must be preserved as a single argument; got: {inlined}"
    );
}

#[test]
fn test_argument_with_comma_in_double_quoted_string() {
    // Commas inside double-quoted strings must not split the argument list.
    let source = r#"sub describe {
    my ($a, $b) = @_;
    return $a . $b;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("describe", r#"describe("one, two", "three")"#);
    let inlined = must(result);
    assert!(
        inlined.contains("\"one, two\""),
        "comma inside double-quoted argument must not be treated as a separator; got: {inlined}"
    );
}

#[test]
fn test_argument_with_comma_in_single_quoted_string() {
    // Same protection must apply for single-quoted strings.
    let source = r#"sub describe {
    my ($a, $b) = @_;
    return $a . $b;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("describe", "describe('one, two', 'three')");
    let inlined = must(result);
    assert!(
        inlined.contains("'one, two'"),
        "comma inside single-quoted argument must not be treated as a separator; got: {inlined}"
    );
}

#[test]
fn test_argument_with_escaped_quote_in_string() {
    // A backslash-escaped quote inside an argument must not terminate the string,
    // so split_args still sees the comma as inside the quoted region.
    let source = r#"sub describe {
    my ($a, $b) = @_;
    return $a . $b;
}
"#;
    let inliner = SubInliner::new(source);
    let result =
        inliner.inline_call("describe", r#"describe("with \" quote, still inside", "tail")"#);
    let inlined = must(result);
    assert!(
        inlined.contains("\"with \\\" quote, still inside\""),
        "escaped quote inside an argument must not end the string; got: {inlined}"
    );
}

// ---------------------------------------------------------------------------
// Sub-body shapes — no param line, no return, side-effect keyword variants
// ---------------------------------------------------------------------------

#[test]
fn test_sub_without_param_line_inlines_zero_args() {
    // A sub with no `my (...) = @_;` line should still be inlineable when the
    // call has no arguments — extract_params_line returns an empty Vec.
    let source = r#"sub literal {
    return 42;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("literal", "literal()");
    let inlined = must(result);
    assert!(
        inlined.contains("42"),
        "param-less sub should inline its return expression; got: {inlined}"
    );
}

#[test]
fn test_sub_without_return_uses_trimmed_body_as_expression() {
    // If the body has no `return` statement, extract_return_expr falls through to
    // the trimmed body. The result should reflect the body's expression directly.
    let source = r#"sub greet {
    "static";
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call("greet", "greet()");
    let inlined = must(result);
    assert!(
        inlined.contains("\"static\""),
        "body without an explicit `return` should still produce inlined text; got: {inlined}"
    );
}

#[test]
fn test_warn_keyword_triggers_side_effect_warning() {
    let source = r#"sub log_msg {
    my ($msg) = @_;
    warn "[debug] $msg";
    return 1;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call_with_warnings("log_msg", "log_msg(\"hi\")");
    let (_, warnings) = must(result);
    assert!(!warnings.is_empty(), "warn keyword should produce a side-effect warning");
}

#[test]
fn test_die_keyword_triggers_side_effect_warning() {
    let source = r#"sub assert_ok {
    my ($cond) = @_;
    die "fail" unless $cond;
    return 1;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call_with_warnings("assert_ok", "assert_ok(1)");
    let (_, warnings) = must(result);
    assert!(!warnings.is_empty(), "die keyword should produce a side-effect warning");
}

#[test]
fn test_open_keyword_triggers_side_effect_warning() {
    let source = r#"sub read_file {
    my ($path) = @_;
    open my $fh, '<', $path;
    return 1;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call_with_warnings("read_file", "read_file(\"/tmp/x\")");
    let (_, warnings) = must(result);
    assert!(!warnings.is_empty(), "open keyword should produce a side-effect warning");
}

#[test]
fn test_pure_sub_produces_no_warnings() {
    // A sub that performs no side-effect operations should yield zero warnings,
    // exercising the !has_side_effects branch in inline_call_inner.
    let source = r#"sub square {
    my ($n) = @_;
    return $n * $n;
}
"#;
    let inliner = SubInliner::new(source);
    let result = inliner.inline_call_with_warnings("square", "square(4)");
    let (_, warnings) = must(result);
    assert!(warnings.is_empty(), "pure sub should not produce warnings; got: {:?}", warnings);
}

// ---------------------------------------------------------------------------
// analyze_sub_for_inlining — direct exposure of the analysis result shape
// ---------------------------------------------------------------------------

#[test]
fn test_analyze_exposes_params_and_body() {
    // analyze_sub_for_inlining should report the parameter list and a body that
    // has the parameter line stripped.
    let source = r#"sub multiply {
    my ($x, $y) = @_;
    return $x * $y;
}
"#;
    let analysis = must(analyze_sub_for_inlining(source, "multiply"));
    let InlineAbility::Ok { params, body, has_side_effects } = analysis;
    assert_eq!(params, vec!["x".to_string(), "y".to_string()]);
    assert!(!body.contains("= @_"), "param line should be stripped from body; got: {body}");
    assert!(!has_side_effects, "pure multiply should not flag side effects");
}

#[test]
fn test_analyze_flags_side_effects_for_print() {
    let source = r#"sub trace {
    my ($x) = @_;
    print "trace: $x\n";
    return $x;
}
"#;
    let analysis = must(analyze_sub_for_inlining(source, "trace"));
    let InlineAbility::Ok { has_side_effects, .. } = analysis;
    assert!(has_side_effects, "analyze should flag print as a side effect");
}
