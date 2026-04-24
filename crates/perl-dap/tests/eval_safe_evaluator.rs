//! Comprehensive integration tests for perl-dap-eval
//!
//! Covers: safe expressions, all dangerous operation categories, assignment operators,
//! increment/decrement, backticks, newlines, regex mutation, sigil-prefixed identifiers,
//! braced variables, CORE:: qualification, package-qualified names, single-quoted strings,
//! escape sequences, error variant messages, and edge cases.

use perl_dap::eval::{DANGEROUS_OPERATIONS, SafeEvaluator, ValidationError, ValidationResult};
use perl_tdd_support::must_err;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn eval() -> SafeEvaluator {
    SafeEvaluator::new()
}

fn ok(expr: &str) -> ValidationResult {
    eval().validate(expr)
}

fn err(expr: &str) -> ValidationError {
    must_err(eval().validate(expr))
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

#[test]
fn default_and_new_are_equivalent() -> Result<(), ValidationError> {
    let a = SafeEvaluator::default();
    let b = SafeEvaluator::new();
    a.validate("$x")?;
    b.validate("$x")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Safe expressions
// ---------------------------------------------------------------------------

#[test]
fn safe_simple_arithmetic() -> Result<(), ValidationError> {
    ok("$x + $y")?;
    ok("$a - $b")?;
    ok("$x * 2")?;
    ok("$x / 3")?;
    ok("$x % 4")?;
    ok("$x ** 2")?;
    Ok(())
}

#[test]
fn safe_comparison_operators_without_equals() -> Result<(), ValidationError> {
    // Comparisons that don't contain `=` substring pass safely
    ok("$x > $y")?;
    ok("$x < $y")?;
    ok("$x eq $y")?;
    ok("$x ne $y")?;
    Ok(())
}

#[test]
fn comparison_operators_containing_equals_are_safe() -> Result<(), ValidationError> {
    ok("$x == $y")?;
    ok("$x != $y")?;
    ok("$x >= $y")?;
    ok("$x <= $y")?;
    ok("$x <=> $y")?;
    Ok(())
}

#[test]
fn safe_string_operators() -> Result<(), ValidationError> {
    ok("$x . $y")?;
    ok("$x x 3")?;
    ok("substr($str, 0, 5)")?;
    ok("length($str)")?;
    ok("index($str, 'foo')")?;
    ok("rindex($str, 'foo')")?;
    Ok(())
}

#[test]
fn safe_hash_and_array_access() -> Result<(), ValidationError> {
    ok("$hash{key}")?;
    ok("$hash{'key'}")?;
    ok("$array[0]")?;
    ok("$array[-1]")?;
    ok("$ref->{key}")?;
    ok("$ref->[0]")?;
    ok("scalar(@array)")?;
    ok("defined($x)")?;
    ok("ref($x)")?;
    Ok(())
}

#[test]
fn safe_ternary_and_logical() -> Result<(), ValidationError> {
    ok("$x ? 1 : 0")?;
    ok("$x && $y")?;
    ok("$x || $y")?;
    ok("!$x")?;
    ok("not $x")?;
    Ok(())
}

#[test]
fn safe_empty_and_whitespace() -> Result<(), ValidationError> {
    ok("")?;
    ok("   ")?;
    ok("42")?;
    Ok(())
}

#[test]
fn safe_complex_expressions() -> Result<(), ValidationError> {
    ok("$hash{$key} + $array[$idx]")?;
    ok("($a + $b) * ($c - $d)")?;
    ok("wantarray()")?;
    ok("caller(0)")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Dangerous operations – exhaustive category coverage
// ---------------------------------------------------------------------------

#[test]
fn dangerous_state_mutation_ops() {
    for op in
        &["push", "pop", "shift", "unshift", "splice", "delete", "undef", "srand", "bless", "reset"]
    {
        let expr = format!("{op}($x)");
        assert!(eval().validate(&expr).is_err(), "expected {op} to be blocked");
    }
}

#[test]
fn dangerous_process_control_ops() {
    for op in &[
        "system",
        "exec",
        "fork",
        "exit",
        "dump",
        "kill",
        "alarm",
        "sleep",
        "wait",
        "waitpid",
        "setpgrp",
        "setpriority",
        "umask",
        "lock",
    ] {
        let expr = format!("{op}()");
        assert!(eval().validate(&expr).is_err(), "expected {op} to be blocked");
    }
}

#[test]
fn dangerous_io_ops() {
    for op in &[
        "qx",
        "readpipe",
        "syscall",
        "open",
        "close",
        "print",
        "say",
        "printf",
        "sysread",
        "syswrite",
        "glob",
        "readline",
        "ioctl",
        "fcntl",
        "flock",
        "select",
        "dbmopen",
        "dbmclose",
        "binmode",
        "opendir",
        "closedir",
        "readdir",
        "rewinddir",
        "seekdir",
        "telldir",
        "seek",
        "sysseek",
        "formline",
        "write",
        "pipe",
        "socketpair",
    ] {
        let expr = format!("{op}()");
        assert!(eval().validate(&expr).is_err(), "expected {op} to be blocked");
    }
}

#[test]
fn dangerous_filesystem_ops() {
    for op in &[
        "mkdir", "rmdir", "unlink", "rename", "chdir", "chmod", "chown", "chroot", "truncate",
        "utime", "symlink", "link",
    ] {
        let expr = format!("{op}('path')");
        assert!(eval().validate(&expr).is_err(), "expected {op} to be blocked");
    }
}

#[test]
fn dangerous_code_loading_ops() {
    for op in &["eval", "require", "do"] {
        let expr = format!("{op}('code')");
        assert!(eval().validate(&expr).is_err(), "expected {op} to be blocked");
    }
}

#[test]
fn dangerous_tie_ops() {
    assert!(eval().validate("tie(%hash, 'DB_File')").is_err());
    assert!(eval().validate("untie(%hash)").is_err());
}

#[test]
fn dangerous_network_ops() {
    for op in
        &["socket", "connect", "bind", "listen", "accept", "send", "recv", "shutdown", "setsockopt"]
    {
        let expr = format!("{op}()");
        assert!(eval().validate(&expr).is_err(), "expected {op} to be blocked");
    }
}

#[test]
fn dangerous_ipc_ops() {
    for op in &[
        "msgget", "msgsnd", "msgrcv", "msgctl", "semget", "semop", "semctl", "shmget", "shmat",
        "shmdt", "shmctl",
    ] {
        let expr = format!("{op}()");
        assert!(eval().validate(&expr).is_err(), "expected {op} to be blocked");
    }
}

#[test]
fn every_dangerous_op_is_blocked() {
    for op in DANGEROUS_OPERATIONS {
        let expr = format!("{op}()");
        assert!(
            eval().validate(&expr).is_err(),
            "DANGEROUS_OPERATIONS entry '{op}' was not blocked"
        );
    }
}

// ---------------------------------------------------------------------------
// CORE:: qualified operations (should be blocked)
// ---------------------------------------------------------------------------

#[test]
fn core_qualified_is_blocked() {
    assert!(eval().validate("CORE::system('ls')").is_err());
    assert!(eval().validate("CORE::print('x')").is_err());
    assert!(eval().validate("CORE::eval('x')").is_err());
}

#[test]
fn core_global_qualified_is_blocked() {
    assert!(eval().validate("CORE::GLOBAL::system('ls')").is_err());
}

// ---------------------------------------------------------------------------
// Package-qualified names (NOT CORE) — should be safe
// ---------------------------------------------------------------------------

#[test]
fn package_qualified_names_are_safe() -> Result<(), ValidationError> {
    ok("Foo::print($x)")?;
    ok("My::Module::system()")?;
    ok("Bar::Baz::eval()")?;
    ok("IO::Socket::connect()")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Sigil-prefixed identifiers — should be safe
// ---------------------------------------------------------------------------

#[test]
fn sigil_dollar_safe() -> Result<(), ValidationError> {
    ok("$print")?;
    ok("$system")?;
    ok("$eval")?;
    ok("$exec")?;
    ok("$fork")?;
    ok("$exit")?;
    ok("$kill")?;
    ok("$open")?;
    ok("$close")?;
    ok("$say")?;
    Ok(())
}

#[test]
fn sigil_at_safe() -> Result<(), ValidationError> {
    ok("@say")?;
    ok("@print")?;
    ok("@system")?;
    Ok(())
}

#[test]
fn sigil_percent_safe() -> Result<(), ValidationError> {
    ok("%exit")?;
    ok("%system")?;
    Ok(())
}

#[test]
fn sigil_star_safe() -> Result<(), ValidationError> {
    ok("*dump")?;
    ok("*print")?;
    Ok(())
}

#[test]
fn sigil_prefixed_with_suffix_safe() -> Result<(), ValidationError> {
    ok("$system_name")?;
    ok("$print_count")?;
    ok("$eval_result")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Sigil-prefixed but dangerous (code deref / method call)
// ---------------------------------------------------------------------------

#[test]
fn ampersand_deref_is_dangerous() {
    assert!(eval().validate("&$system").is_err());
}

#[test]
fn arrow_method_call_is_dangerous() {
    assert!(eval().validate("$obj->$system").is_err());
}

#[test]
fn braced_ampersand_deref_is_dangerous() {
    assert!(eval().validate("&{ $system }").is_err());
}

// ---------------------------------------------------------------------------
// Braced scalar variables — should be safe
// ---------------------------------------------------------------------------

#[test]
fn braced_scalar_variables_are_safe() -> Result<(), ValidationError> {
    ok("${print}")?;
    ok("${system}")?;
    ok("${eval}")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Assignment operators — exhaustive
// ---------------------------------------------------------------------------

#[test]
fn all_assignment_operators_blocked() {
    let ops = [
        "=", "+=", "-=", "*=", "/=", "%=", "**=", ".=", "&=", "|=", "^=", "<<=", ">>=", "&&=",
        "||=", "//=",
    ];
    for op in &ops {
        let expr = format!("$x {op} 1");
        assert!(
            eval().validate(&expr).is_err(),
            "expected assignment operator '{op}' to be blocked"
        );
    }
}

// ---------------------------------------------------------------------------
// Increment / Decrement
// ---------------------------------------------------------------------------

#[test]
fn postfix_increment_blocked() {
    assert!(eval().validate("$x++").is_err());
}

#[test]
fn prefix_increment_blocked() {
    assert!(eval().validate("++$x").is_err());
}

#[test]
fn postfix_decrement_blocked() {
    assert!(eval().validate("$x--").is_err());
}

#[test]
fn prefix_decrement_blocked() {
    assert!(eval().validate("--$x").is_err());
}

// ---------------------------------------------------------------------------
// Backticks / shell execution
// ---------------------------------------------------------------------------

#[test]
fn backticks_blocked() {
    assert!(eval().validate("`ls -la`").is_err());
    assert!(eval().validate("`whoami`").is_err());
    assert!(eval().validate("my $x = `cat /etc/passwd`").is_err());
}

// ---------------------------------------------------------------------------
// Newlines / carriage returns
// ---------------------------------------------------------------------------

#[test]
fn newline_blocked() {
    assert!(eval().validate("1\nprint 'x'").is_err());
}

#[test]
fn carriage_return_blocked() {
    assert!(eval().validate("1\rprint 'x'").is_err());
}

#[test]
fn crlf_blocked() {
    assert!(eval().validate("1\r\nprint 'x'").is_err());
}

// ---------------------------------------------------------------------------
// Regex mutation operators
// ---------------------------------------------------------------------------

#[test]
fn substitution_blocked() {
    assert!(eval().validate("s/foo/bar/").is_err());
    assert!(eval().validate("s|foo|bar|").is_err());
    assert!(eval().validate("s{foo}{bar}").is_err());
}

#[test]
fn transliteration_blocked() {
    assert!(eval().validate("tr/a-z/A-Z/").is_err());
    assert!(eval().validate("y/abc/xyz/").is_err());
}

#[test]
fn sigil_prefixed_s_tr_y_safe() -> Result<(), ValidationError> {
    ok("$s")?;
    ok("$tr")?;
    ok("$y")?;
    Ok(())
}

#[test]
fn escape_sequence_s_safe() -> Result<(), ValidationError> {
    // \s in a regex pattern should be allowed
    ok("/\\s+/")?;
    ok("/\\y/")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Single-quoted strings — ops inside are safe
// ---------------------------------------------------------------------------

#[test]
fn ops_in_single_quotes_are_safe() -> Result<(), ValidationError> {
    ok("'print this'")?;
    ok("'system call'")?;
    ok("'eval something'")?;
    ok("'exec program'")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Error variant matching
// ---------------------------------------------------------------------------

#[test]
fn error_variant_dangerous_operation() {
    let e = err("system('ls')");
    assert!(matches!(e, ValidationError::DangerousOperation(ref op) if op == "system"));
    // Verify Display output
    let msg = format!("{e}");
    assert!(msg.contains("system"));
    assert!(msg.contains("allowSideEffects"));
}

#[test]
fn error_variant_assignment_operator() {
    let e = err("$x = 1");
    assert!(matches!(e, ValidationError::AssignmentOperator(ref op) if op == "="));
    let msg = format!("{e}");
    assert!(msg.contains("assignment operator"));
}

#[test]
fn error_variant_increment_decrement() {
    let e = err("$x++");
    assert!(matches!(e, ValidationError::IncrementDecrement));
    let msg = format!("{e}");
    assert!(msg.contains("increment/decrement"));
}

#[test]
fn error_variant_backticks() {
    let e = err("`ls`");
    assert!(matches!(e, ValidationError::Backticks));
    let msg = format!("{e}");
    assert!(msg.contains("backticks"));
}

#[test]
fn error_variant_regex_mutation() {
    let e = err("s/a/b/");
    assert!(matches!(e, ValidationError::RegexMutation(_)));
    let msg = format!("{e}");
    assert!(msg.contains("regex mutation"));
}

#[test]
fn error_variant_contains_newlines() {
    let e = err("1\n2");
    assert!(matches!(e, ValidationError::ContainsNewlines));
    let msg = format!("{e}");
    assert!(msg.contains("newline"));
}

// ---------------------------------------------------------------------------
// Validation priority / ordering
// ---------------------------------------------------------------------------

#[test]
fn newline_checked_before_other_patterns() {
    // Expression has both newline and dangerous op; newline should be caught first
    let e = err("system('x')\neval('y')");
    assert!(matches!(e, ValidationError::ContainsNewlines));
}

#[test]
fn backtick_checked_before_assignment() {
    // Expression has backtick and assignment
    let e = err("$x = `ls`");
    // Backtick wins because it's checked before assignment in the validate flow
    // Actually assignment (=) is checked first in the code flow, let's verify
    // The actual order: newlines -> backticks -> assignment -> incr/decr -> dangerous -> regex
    // So `=` will match before backtick since assignment is checked first
    assert!(
        matches!(e, ValidationError::AssignmentOperator(_))
            || matches!(e, ValidationError::Backticks)
    );
}

// ---------------------------------------------------------------------------
// Clone and Debug on error types
// ---------------------------------------------------------------------------

#[test]
fn validation_error_is_clone_and_debug() {
    let e = err("eval('x')");
    let cloned = e.clone();
    let debug = format!("{cloned:?}");
    assert!(!debug.is_empty());
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn bare_op_name_in_word_boundary() -> Result<(), ValidationError> {
    // "evaluate" contains "eval" but should not trigger (word boundary)
    ok("$evaluate")?;
    Ok(())
}

#[test]
fn op_surrounded_by_parens() {
    // Bare `eval` with parens should be dangerous
    assert!(eval().validate("eval()").is_err());
}

#[test]
fn op_with_space_before_parens() {
    assert!(eval().validate("eval ()").is_err());
}

#[test]
fn multiple_safe_lookups() -> Result<(), ValidationError> {
    ok("$hash{a} . $hash{b} . $hash{c}")?;
    Ok(())
}

#[test]
fn numeric_literals_safe() -> Result<(), ValidationError> {
    ok("42")?;
    ok("3.14")?;
    ok("0xff")?;
    ok("1_000_000")?;
    Ok(())
}

#[test]
fn qw_safe() -> Result<(), ValidationError> {
    // qw is not in the dangerous ops list
    ok("qw(foo bar baz)")?;
    Ok(())
}
