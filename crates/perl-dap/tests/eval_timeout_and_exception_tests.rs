//! Tests for timeout behaviour, exception handling patterns, large output,
//! side-effect expressions, and invalid syntax in perl-dap-eval.
//!
//! These tests validate the *validator* layer — the SafeEvaluator never actually
//! executes Perl code, so "timeout behaviour" and "large output handling" are
//! tested by verifying that the validator itself returns promptly and correctly
//! even on adversarial inputs, and that long/deeply-nested expressions are
//! classified correctly.

use perl_dap::eval::{SafeEvaluator, ValidationError};
use perl_tdd_support::must_err;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn eval() -> SafeEvaluator {
    SafeEvaluator::new()
}

fn ok(expr: &str) -> Result<(), ValidationError> {
    eval().validate(expr)
}

fn err(expr: &str) -> ValidationError {
    must_err(eval().validate(expr))
}

// ===========================================================================
// 1. Expression evaluation timeout behaviour
//
//    The SafeEvaluator is a pure validator (no Perl execution), so it must
//    never hang even on pathologically large or repetitive inputs.  We verify
//    that validation completes within a generous wall-clock budget.
// ===========================================================================

#[test]
fn validator_returns_promptly_on_very_long_safe_expression() -> Result<(), ValidationError> {
    // Build a long but safe expression: "$v0 + $v1 + ... + $v9999"
    let parts: Vec<String> = (0..10_000).map(|i| format!("$v{i}")).collect();
    let expr = parts.join(" + ");
    let start = std::time::Instant::now();
    ok(&expr)?;
    let elapsed = start.elapsed();
    // Should complete well under 2 seconds on any reasonable machine
    assert!(
        elapsed.as_secs() < 2,
        "Validator took too long on long safe expression: {elapsed:?}"
    );
    Ok(())
}

#[test]
fn validator_returns_promptly_on_very_long_dangerous_expression() {
    // Build a long expression that starts with a dangerous op
    let padding: String = " + $x".repeat(10_000);
    let expr = format!("system('ls'){padding}");
    let start = std::time::Instant::now();
    let e = err(&expr);
    let elapsed = start.elapsed();
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "system"));
    assert!(
        elapsed.as_secs() < 2,
        "Validator took too long on long dangerous expression: {elapsed:?}"
    );
}

#[test]
fn validator_handles_deeply_nested_parentheses() -> Result<(), ValidationError> {
    // (((((... $x ...)))))
    let depth = 500;
    let open: String = "(".repeat(depth);
    let close: String = ")".repeat(depth);
    let expr = format!("{open}$x{close}");
    ok(&expr)?;
    Ok(())
}

#[test]
fn validator_handles_repeated_backtick_detection_quickly() {
    // Many backticks — should fail immediately on the first one
    let expr = "`".repeat(50_000);
    let start = std::time::Instant::now();
    let e = err(&expr);
    let elapsed = start.elapsed();
    assert!(matches!(e, ValidationError::Backticks));
    assert!(
        elapsed.as_secs() < 2,
        "Backtick detection took too long: {elapsed:?}"
    );
}

#[test]
fn validator_handles_repeated_newline_detection_quickly() {
    let expr = "\n".repeat(50_000);
    let start = std::time::Instant::now();
    let e = err(&expr);
    let elapsed = start.elapsed();
    assert!(matches!(e, ValidationError::ContainsNewlines));
    assert!(
        elapsed.as_secs() < 2,
        "Newline detection took too long: {elapsed:?}"
    );
}

// ===========================================================================
// 2. die/eval nesting in evaluated expressions
//
//    Perl's die/eval is a common pattern.  The validator blocks `eval` (code
//    execution) but allows `die` and `warn` (these don't execute arbitrary
//    code and are useful for inspecting exception state during debugging).
// ===========================================================================

#[test]
fn eval_is_blocked_even_when_wrapping_die() {
    // eval { die "msg" } — the eval is still blocked
    let e = err("eval { die 'oops' }");
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "eval"));
}

#[test]
fn eval_block_form_is_blocked() {
    let e = err("eval { $x }");
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "eval"));
}

#[test]
fn eval_string_form_is_blocked() {
    let e = err("eval('$x + 1')");
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "eval"));
}

#[test]
fn nested_eval_blocked() {
    // eval { eval { ... } } — first eval caught
    let e = err("eval { eval { 1 } }");
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "eval"));
}

#[test]
fn die_alone_is_safe() -> Result<(), ValidationError> {
    ok("die('error message')")?;
    ok("die 'fatal'")?;
    ok("die $err")?;
    Ok(())
}

#[test]
fn warn_alone_is_safe() -> Result<(), ValidationError> {
    ok("warn('something happened')")?;
    ok("warn $msg")?;
    Ok(())
}

#[test]
fn die_with_object_is_safe() -> Result<(), ValidationError> {
    // die My::Exception->new(...)  — the ->new is a method call, safe for validation
    ok("die My::Exception->new('msg')")?;
    Ok(())
}

#[test]
fn dollar_at_is_safe() -> Result<(), ValidationError> {
    // $@ is the Perl exception variable — should be inspectable
    ok("$@")?;
    ok("ref($@)")?;
    ok("defined($@)")?;
    ok("$@ ? $@ : 'no error'")?;
    Ok(())
}

#[test]
fn sigil_prefixed_eval_variable_is_safe() -> Result<(), ValidationError> {
    ok("$eval")?;
    ok("$eval_result")?;
    ok("@eval_errors")?;
    ok("%eval_cache")?;
    Ok(())
}

#[test]
fn core_eval_is_blocked() {
    let e = err("CORE::eval('code')");
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "eval"));
}

#[test]
fn package_qualified_eval_is_safe() -> Result<(), ValidationError> {
    // Foo::eval is a user-defined method, not the built-in
    ok("Foo::eval($x)")?;
    ok("My::Module::eval()")?;
    Ok(())
}

// ===========================================================================
// 3. Large expression output handling
//
//    Verify that the validator correctly classifies very long expressions
//    without truncation or misclassification.  The validator does not produce
//    "output" per se, but it must correctly identify dangerous patterns even
//    when buried in large amounts of surrounding safe text.
// ===========================================================================

#[test]
fn dangerous_op_buried_in_large_safe_expression() {
    // Safe preamble + dangerous op in the middle + safe suffix
    let prefix: String = (0..5_000).map(|i| format!("$v{i} + ")).collect();
    let suffix: String = (0..5_000).map(|i| format!(" + $w{i}")).collect();
    let expr = format!("{prefix}system('ls'){suffix}");
    let e = err(&expr);
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "system"));
}

#[test]
fn dangerous_op_at_very_end_of_large_expression() {
    let prefix: String = (0..5_000).map(|i| format!("$v{i} + ")).collect();
    let expr = format!("{prefix}eval('code')");
    let e = err(&expr);
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "eval"));
}

#[test]
fn safe_expression_with_many_hash_accesses() -> Result<(), ValidationError> {
    // Many hash lookups: $h{k0} . $h{k1} . ... . $h{k999}
    let parts: Vec<String> = (0..1_000).map(|i| format!("$h{{k{i}}}")).collect();
    let expr = parts.join(" . ");
    ok(&expr)?;
    Ok(())
}

#[test]
fn safe_expression_with_many_nested_derefs() -> Result<(), ValidationError> {
    // $ref->{a}->{b}->{c}->...  deep chain
    let chain: String = (0..200).map(|i| format!("->{{k{i}}}")).collect();
    let expr = format!("$ref{chain}");
    ok(&expr)?;
    Ok(())
}

#[test]
fn error_message_includes_op_name_regardless_of_expression_length() {
    let padding = "$x + ".repeat(1_000);
    let expr = format!("{padding}fork()");
    let e = err(&expr);
    let msg = format!("{e}");
    assert!(
        msg.contains("fork"),
        "Error message should contain the operation name 'fork'"
    );
    assert!(
        msg.contains("allowSideEffects"),
        "Error message should mention allowSideEffects"
    );
}

// ===========================================================================
// 4. Expressions with side effects
//
//    Verify that all categories of side-effect-producing expressions are
//    correctly rejected.
// ===========================================================================

#[test]
fn state_mutation_side_effects_blocked() {
    // Array/hash mutation
    assert!(eval().validate("push(@arr, $x)").is_err());
    assert!(eval().validate("pop(@arr)").is_err());
    assert!(eval().validate("shift(@arr)").is_err());
    assert!(eval().validate("unshift(@arr, $x)").is_err());
    assert!(eval().validate("splice(@arr, 0, 1)").is_err());
    assert!(eval().validate("delete $hash{key}").is_err());
}

#[test]
fn process_side_effects_blocked() {
    assert!(eval().validate("fork()").is_err());
    assert!(eval().validate("kill(9, $pid)").is_err());
    assert!(eval().validate("alarm(10)").is_err());
    assert!(eval().validate("sleep(1)").is_err());
}

#[test]
fn io_side_effects_blocked() {
    assert!(eval().validate("print STDERR 'msg'").is_err());
    assert!(eval().validate("say 'hello'").is_err());
    assert!(eval().validate("printf '%s', $x").is_err());
    assert!(eval().validate("write()").is_err());
}

#[test]
fn filesystem_side_effects_blocked() {
    assert!(eval().validate("mkdir('/tmp/test')").is_err());
    assert!(eval().validate("rmdir('/tmp/test')").is_err());
    assert!(eval().validate("unlink('/tmp/file')").is_err());
    assert!(eval().validate("rename('a', 'b')").is_err());
    assert!(eval().validate("chmod(0o755, 'file')").is_err());
    assert!(eval().validate("chown(0, 0, 'file')").is_err());
}

#[test]
fn network_side_effects_blocked() {
    assert!(eval().validate("socket(SOCK, 2, 1, 0)").is_err());
    assert!(eval().validate("connect(SOCK, $addr)").is_err());
    assert!(eval().validate("bind(SOCK, $addr)").is_err());
    assert!(eval().validate("send(SOCK, $data, 0)").is_err());
}

#[test]
fn tie_side_effects_blocked() {
    assert!(eval().validate("tie(%hash, 'DB_File', 'file.db')").is_err());
    assert!(eval().validate("untie(%hash)").is_err());
}

#[test]
fn bless_side_effect_blocked() {
    assert!(eval().validate("bless($ref, 'MyClass')").is_err());
}

#[test]
fn combined_side_effects_first_match_wins() {
    // Assignment + dangerous op: assignment is checked first
    let e = err("$x = system('ls')");
    assert!(matches!(e, ValidationError::AssignmentOperator(_)));
}

#[test]
fn chained_method_with_side_effect_op() {
    // Direct dangerous op even in method-call-looking context
    assert!(eval().validate("system($cmd)").is_err());
    assert!(eval().validate("exec($cmd)").is_err());
}

#[test]
fn regex_substitution_is_a_side_effect() {
    let e = err("s/old/new/g");
    assert!(matches!(e, ValidationError::RegexMutation(_)));
}

#[test]
fn transliteration_is_a_side_effect() {
    let e = err("tr/a-z/A-Z/");
    assert!(matches!(e, ValidationError::RegexMutation(_)));
}

#[test]
fn increment_is_a_side_effect() {
    let e = err("$counter++");
    assert!(matches!(e, ValidationError::IncrementDecrement));
}

#[test]
fn compound_assignment_is_a_side_effect() {
    let e = err("$total += $amount");
    // The validator checks `=` first (substring match), so `+=` matches as `=`
    assert!(matches!(e, ValidationError::AssignmentOperator(_)));
}

// ===========================================================================
// 5. Invalid expression syntax
//
//    The SafeEvaluator does not parse Perl — it only pattern-matches for
//    dangerous constructs.  Syntactically invalid expressions that don't
//    contain dangerous patterns should pass validation (Perl will reject
//    them at eval time).  Syntactically invalid expressions that DO contain
//    dangerous patterns should still be caught.
// ===========================================================================

#[test]
fn unbalanced_parens_safe_if_no_dangerous_ops() -> Result<(), ValidationError> {
    ok("((($x")?;
    ok("$x)))")?;
    ok("$x + ($y")?;
    Ok(())
}

#[test]
fn unbalanced_braces_safe_if_no_dangerous_ops() -> Result<(), ValidationError> {
    ok("$hash{{")?;
    ok("}$x")?;
    Ok(())
}

#[test]
fn unbalanced_brackets_safe_if_no_dangerous_ops() -> Result<(), ValidationError> {
    ok("$arr[[")?;
    ok("]$x")?;
    Ok(())
}

#[test]
fn random_garbage_safe_if_no_patterns() -> Result<(), ValidationError> {
    ok("@#!^&*")?;
    ok("??? $$$ %%% &&& (((")?;
    Ok(())
}

#[test]
fn incomplete_string_literals_safe() -> Result<(), ValidationError> {
    ok("'unterminated")?;
    ok("\"also unterminated")?;
    Ok(())
}

#[test]
fn invalid_syntax_with_dangerous_op_still_blocked() {
    // Garbage surrounding a dangerous op — the op should still be caught
    assert!(eval().validate(")))system(((").is_err());
    assert!(eval().validate("???eval???").is_err());
    assert!(eval().validate("...fork...").is_err());
}

#[test]
fn invalid_syntax_with_assignment_still_blocked() {
    assert!(eval().validate("??? = ???").is_err());
    assert!(eval().validate("))) += (((").is_err());
}

#[test]
fn invalid_syntax_with_backtick_still_blocked() {
    assert!(eval().validate("???`???").is_err());
}

#[test]
fn invalid_syntax_with_newline_still_blocked() {
    assert!(eval().validate("???\n???").is_err());
}

#[test]
fn invalid_syntax_with_regex_mutation_still_blocked() {
    assert!(eval().validate(")))s/a/b/(((").is_err());
}

#[test]
fn only_operators_no_operands() -> Result<(), ValidationError> {
    // Pure operators without operands — syntactically invalid but not dangerous
    ok("+ - * /")?;
    ok("? : !")?;
    Ok(())
}

#[test]
fn mixed_valid_and_invalid_tokens() -> Result<(), ValidationError> {
    ok("$x + ??? + $y")?;
    ok("$hash{!!!}")?;
    Ok(())
}

#[test]
fn null_byte_in_expression_safe_if_no_dangerous_ops() -> Result<(), ValidationError> {
    // Null bytes are unusual but the validator should handle them gracefully
    ok("$x\0$y")?;
    Ok(())
}

#[test]
fn unicode_in_expression_safe() -> Result<(), ValidationError> {
    ok("$\u{00e9}l\u{00e8}ve")?; // $eleve with accents
    ok("$\u{1f600}")?; // emoji in variable name
    ok("'\u{4e16}\u{754c}'")?; // Chinese characters in string
    Ok(())
}

#[test]
fn very_long_single_token_safe() -> Result<(), ValidationError> {
    // A very long variable name
    let name = "a".repeat(100_000);
    let expr = format!("${name}");
    ok(&expr)?;
    Ok(())
}

#[test]
fn empty_quotes_safe() -> Result<(), ValidationError> {
    ok("''")?;
    ok("\"\"")?;
    Ok(())
}

// ===========================================================================
// Regression: ensure side-effect detection does not regress on
// previously-safe patterns after new tests are added
// ===========================================================================

#[test]
fn regression_safe_builtins_still_pass() -> Result<(), ValidationError> {
    ok("defined($x)")?;
    ok("ref($obj)")?;
    ok("length($str)")?;
    ok("scalar(@arr)")?;
    ok("exists($h{k})")?;
    ok("wantarray()")?;
    ok("caller()")?;
    ok("int($n)")?;
    ok("abs($n)")?;
    ok("sqrt($n)")?;
    Ok(())
}

#[test]
fn regression_sigil_prefixed_still_safe() -> Result<(), ValidationError> {
    ok("$system")?;
    ok("$eval")?;
    ok("$print")?;
    ok("$exec")?;
    ok("@open")?;
    ok("%close")?;
    ok("*fork")?;
    Ok(())
}

#[test]
fn regression_package_qualified_still_safe() -> Result<(), ValidationError> {
    ok("Foo::system()")?;
    ok("Bar::eval()")?;
    ok("Baz::Qux::exec()")?;
    Ok(())
}
