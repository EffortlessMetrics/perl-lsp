//! Extended unit tests for perl-dap-eval
//!
//! Comprehensive test coverage including:
//! - Advanced expression scenarios
//! - Complex validation edge cases
#![allow(clippy::expect_used)]
//! - Nested expressions and combinations
//! - Context-sensitive pattern detection
//! - Comprehensive error message validation
//! - Boundary conditions and special characters
//! - Multi-line rejection scenarios

use perl_dap::eval::{SafeEvaluator, ValidationError, ValidationResult};

fn eval() -> SafeEvaluator {
    SafeEvaluator::new()
}

fn ok(expr: &str) -> ValidationResult {
    eval().validate(expr)
}

fn err(expr: &str) -> ValidationError {
    eval().validate(expr).expect_err("Expected validation error")
}

// ---------------------------------------------------------------------------
// Complex arithmetic expressions
// ---------------------------------------------------------------------------

#[test]
fn complex_arithmetic_expressions_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("($x + $y) * ($z - $w) / 2")?;
    ok("$a ** 2 + $b ** 2")?;
    ok("($x + $y) % 7")?;
    ok("int($x / $y)")?;
    ok("abs($x)")?;
    ok("sqrt($x)")?;
    Ok(())
}

#[test]
fn exponential_and_modulo_with_parens() -> Result<(), Box<dyn std::error::Error>> {
    ok("(($a + $b) ** ($c - $d)) % 256")?;
    ok("$x ** ($y ** $z)")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Complex nested structures
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_array_access() -> Result<(), Box<dyn std::error::Error>> {
    ok("$array[0][1][2]")?;
    ok("$array[$i][$j][$k]")?;
    ok("$ref->[0]->[1]->[2]")?;
    Ok(())
}

#[test]
fn deeply_nested_hash_access() -> Result<(), Box<dyn std::error::Error>> {
    ok("$hash{a}{b}{c}")?;
    ok("$hash{$key1}{$key2}{$key3}")?;
    ok("$ref->{a}->{b}->{c}")?;
    Ok(())
}

#[test]
fn mixed_array_and_hash_access() -> Result<(), Box<dyn std::error::Error>> {
    ok("$hash{key}[0]")?;
    ok("$array[0]{key}")?;
    ok("$ref->{arr}[0]{key}")?;
    ok("$data{users}[0]{name}")?;
    Ok(())
}

#[test]
fn complex_slice_expressions() -> Result<(), Box<dyn std::error::Error>> {
    ok("@array[0, 2, 4]")?;
    ok("@hash{'a', 'b', 'c'}")?;
    ok("@array[$start..$end]")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// String operations
// ---------------------------------------------------------------------------

#[test]
fn string_concatenation_chains() -> Result<(), Box<dyn std::error::Error>> {
    ok("$a . $b . $c . $d")?;
    ok("$name . ' ' . $surname")?;
    ok("'prefix:' . $value . ':suffix'")?;
    Ok(())
}

#[test]
fn string_repetition_expressions() -> Result<(), Box<dyn std::error::Error>> {
    ok("$char x 10")?;
    ok("'-' x 80")?;
    ok("'*' x ($width - 2)")?;
    Ok(())
}

#[test]
fn complex_string_functions() -> Result<(), Box<dyn std::error::Error>> {
    ok("substr($str, 0, 10)")?;
    ok("substr($str, -5)")?;
    ok("index($str, $pattern)")?;
    ok("rindex($str, $pattern)")?;
    ok("lc($str)")?;
    ok("uc($str)")?;
    ok("lcfirst($str)")?;
    ok("ucfirst($str)")?;
    ok("reverse($str)")?;
    Ok(())
}

#[test]
fn pack_and_unpack_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("pack('C*', @bytes)")?;
    ok("unpack('C*', $data)")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Array and hash functions
// ---------------------------------------------------------------------------

#[test]
fn safe_array_functions() -> Result<(), Box<dyn std::error::Error>> {
    ok("scalar(@array)")?;
    ok("reverse(@array)")?;
    ok("sort(@array)")?;
    ok("join(',', @array)")?;
    ok("grep { condition } @array")?;
    ok("map { expr } @array")?;
    Ok(())
}

#[test]
fn safe_hash_functions() -> Result<(), Box<dyn std::error::Error>> {
    ok("keys(%hash)")?;
    ok("values(%hash)")?;
    ok("each(%hash)")?;
    ok("exists($hash{key})")?;
    ok("scalar(keys(%hash))")?;
    Ok(())
}

#[test]
fn hash_and_array_constructor_expressions() -> Result<(), Box<dyn std::error::Error>> {
    ok("qw(a b c)")?;
    ok("[ $a, $b, $c ]")?;
    ok("( $x, $y, $z )")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Reference and dereference operations
// ---------------------------------------------------------------------------

#[test]
fn reference_creation_and_dereference() -> Result<(), Box<dyn std::error::Error>> {
    ok("\\$x")?;
    ok("\\@array")?;
    ok("\\%hash")?;
    ok("$$ref")?;
    ok("@$array_ref")?;
    ok("%$hash_ref")?;
    Ok(())
}

#[test]
fn method_calls_on_objects() -> Result<(), Box<dyn std::error::Error>> {
    ok("$obj->method()")?;
    ok("$obj->method($arg)")?;
    ok("$obj->method($arg1, $arg2)")?;
    Ok(())
}

#[test]
fn static_method_calls() -> Result<(), Box<dyn std::error::Error>> {
    ok("Package::method()")?;
    ok("Package::method($arg)")?;
    ok("Module::Submodule::method()")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Conditional expressions
// ---------------------------------------------------------------------------

#[test]
fn ternary_operator_nested() -> Result<(), Box<dyn std::error::Error>> {
    ok("$x ? 1 : 0")?;
    ok("$x ? ($y ? 1 : 2) : 3")?;
    ok("$a ? $b ? $c : $d : $e")?;
    Ok(())
}

#[test]
fn complex_logical_expressions() -> Result<(), Box<dyn std::error::Error>> {
    ok("$a && $b || $c")?;
    ok("($a && $b) || ($c && $d)")?;
    ok("$a || $b && $c")?;
    ok("!($a && $b)")?;
    ok("!($a || $b)")?;
    Ok(())
}

#[test]
fn defined_and_existence_checks() -> Result<(), Box<dyn std::error::Error>> {
    ok("defined($x)")?;
    ok("exists($hash{key})")?;
    ok("defined($hash{key})")?;
    ok("defined($array[0])")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Regular expressions (non-mutating)
// ---------------------------------------------------------------------------

#[test]
fn regex_match_patterns_safe() -> Result<(), Box<dyn std::error::Error>> {
    // Note: =~ and !~ contain = which is detected as assignment operator
    // This is a known limitation of substring matching for safety
    // So we test regex patterns themselves
    ok("/pattern/")?;
    ok("m/pattern/")?;
    ok("m{pattern}")?;
    ok("m[pattern]")?;
    Ok(())
}

#[test]
fn regex_with_capture_groups() -> Result<(), Box<dyn std::error::Error>> {
    ok("/(\\d+)/")?;
    ok("/(.+)@(.+)/")?;
    ok("/(a)(b)(c)/")?;
    Ok(())
}

#[test]
fn regex_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    ok("/pattern/i")?;
    ok("/pattern/g")?;
    ok("/pattern/x")?;
    ok("/pattern/igx")?;
    Ok(())
}

#[test]
fn split_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("split(/,/, $str)")?;
    ok("split(',', $str)")?;
    ok("split(' ', $str)")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Context-aware function calls
// ---------------------------------------------------------------------------

#[test]
fn built_in_functions_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("time()")?;
    ok("localtime()")?;
    ok("gmtime()")?;
    ok("rand()")?;
    ok("int($x)")?;
    ok("abs($x)")?;
    ok("sqrt($x)")?;
    ok("sin($x)")?;
    ok("cos($x)")?;
    ok("exp($x)")?;
    ok("log($x)")?;
    Ok(())
}

#[test]
fn reference_inspection_functions() -> Result<(), Box<dyn std::error::Error>> {
    ok("ref($x)")?;
    ok("ref($x) eq 'HASH'")?;
    ok("ref($x) eq 'ARRAY'")?;
    ok("wantarray()")?;
    ok("caller(0)")?;
    ok("caller()")?;
    Ok(())
}

#[test]
fn scalar_context_enforcement() -> Result<(), Box<dyn std::error::Error>> {
    ok("scalar(@array)")?;
    ok("scalar(%hash)")?;
    ok("scalar(reverse(@array))")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Special variables
// ---------------------------------------------------------------------------

#[test]
fn perl_special_variables_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("$_")?;
    ok("$1")?;
    ok("$2")?;
    ok("@")?;
    ok("$!")?;
    ok("$?")?;
    ok("$/")?;
    Ok(())
}

#[test]
fn package_variables_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("$Package::var")?;
    ok("$Module::Submodule::var")?;
    ok("@Package::array")?;
    ok("%Package::hash")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Sigil and context combinations
// ---------------------------------------------------------------------------

#[test]
fn multiple_sigil_prefixed_identifiers() -> Result<(), Box<dyn std::error::Error>> {
    ok("$system + $print")?;
    ok("@eval + @exec")?;
    ok("$exit . $kill")?;
    ok("( $open, $close )")?;
    Ok(())
}

#[test]
fn variable_with_underscores_and_numbers() -> Result<(), Box<dyn std::error::Error>> {
    ok("$system_call_1")?;
    ok("$eval_result_2")?;
    ok("@exit_codes_array")?;
    ok("%print_options")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Assignment operator edge cases
// ---------------------------------------------------------------------------

#[test]
fn all_compound_assignment_ops_blocked() {
    let ops = vec![
        ("+=", "addition assign"),
        ("-=", "subtraction assign"),
        ("*=", "multiplication assign"),
        ("/=", "division assign"),
        ("%=", "modulo assign"),
        ("**=", "exponentiation assign"),
        (".=", "string concat assign"),
        ("&=", "bitwise and assign"),
        ("|=", "bitwise or assign"),
        ("^=", "bitwise xor assign"),
        ("<<=", "left shift assign"),
        (">>=", "right shift assign"),
        ("&&=", "logical and assign"),
        ("||=", "logical or assign"),
        ("//=", "defined-or assign"),
    ];

    for (op, _desc) in ops {
        let expr = format!("$x {op} 1");
        assert!(eval().validate(&expr).is_err(), "op {op} should be blocked");
    }
}

#[test]
fn assignment_with_complex_rhs() {
    assert!(eval().validate("$x = $y + 1").is_err());
    assert!(eval().validate("$a = func($b)").is_err());
    assert!(eval().validate("$result = $a ? $b : $c").is_err());
}

// ---------------------------------------------------------------------------
// Backtick and shell execution edge cases
// ---------------------------------------------------------------------------

#[test]
fn backticks_in_various_positions() {
    assert!(eval().validate("`ls`").is_err());
    assert!(eval().validate("my $x = `date`").is_err());
    assert!(eval().validate("$output = `cat file.txt`").is_err());
    assert!(eval().validate("`echo $HOME`").is_err());
}

#[test]
fn nested_backticks() {
    assert!(eval().validate("`echo \\`date\\``").is_err());
}

// ---------------------------------------------------------------------------
// Newline injection vectors
// ---------------------------------------------------------------------------

#[test]
fn newline_vectors() {
    assert!(eval().validate("$x\n$y").is_err());
    assert!(eval().validate("1\n2\n3").is_err());
    assert!(eval().validate("func()\nprint 'x'").is_err());
}

#[test]
fn carriage_return_vectors() {
    assert!(eval().validate("$x\r$y").is_err());
    assert!(eval().validate("func()\rprint 'x'").is_err());
}

#[test]
fn crlf_combinations() {
    assert!(eval().validate("$x\r\n$y").is_err());
    assert!(eval().validate("1\r\n2").is_err());
}

// ---------------------------------------------------------------------------
// Regex mutation edge cases
// ---------------------------------------------------------------------------

#[test]
fn all_regex_mutation_delimiters() {
    // Test s, tr, y with various delimiters
    assert!(eval().validate("s/a/b/").is_err());
    assert!(eval().validate("s|a|b|").is_err());
    assert!(eval().validate("s{a}{b}").is_err());
    assert!(eval().validate("s[a][b]").is_err());
    assert!(eval().validate("tr/a/b/").is_err());
    assert!(eval().validate("tr|a|b|").is_err());
    assert!(eval().validate("y/a/b/").is_err());
}

#[test]
fn regex_mutation_with_modifiers() {
    assert!(eval().validate("s/a/b/g").is_err());
    assert!(eval().validate("s/a/b/ge").is_err());
    assert!(eval().validate("tr/a/b/c").is_err());
}

#[test]
fn regex_mutation_different_delimiters() {
    // Test various delimiter combinations
    assert!(eval().validate("s|a|b|").is_err());
    assert!(eval().validate("s{a}{b}").is_err());
    assert!(eval().validate("s[a][b]").is_err());
}

// ---------------------------------------------------------------------------
// Increment/decrement variations
// ---------------------------------------------------------------------------

#[test]
fn increment_in_expressions() {
    assert!(eval().validate("$x++ + $y").is_err());
    assert!(eval().validate("++$x").is_err());
    assert!(eval().validate("$arr[++$i]").is_err());
}

#[test]
fn decrement_in_expressions() {
    assert!(eval().validate("$x-- + $y").is_err());
    assert!(eval().validate("--$x").is_err());
    assert!(eval().validate("$arr[--$i]").is_err());
}

// ---------------------------------------------------------------------------
// Dangerous operations in complex contexts
// ---------------------------------------------------------------------------

#[test]
fn dangerous_ops_with_multiple_arguments() {
    assert!(eval().validate("system('cmd', $arg1, $arg2)").is_err());
    assert!(eval().validate("open(my $fh, '<', $file)").is_err());
}

#[test]
fn dangerous_ops_in_ternary() {
    assert!(eval().validate("$x ? system('ls') : 0").is_err());
    assert!(eval().validate("$x ? 1 : eval('code')").is_err());
}

#[test]
fn dangerous_ops_in_logical_expression() {
    assert!(eval().validate("$x && system('ls')").is_err());
    assert!(eval().validate("print('x') || $y").is_err());
}

// ---------------------------------------------------------------------------
// Safe expressions with dangerous-looking patterns
// ---------------------------------------------------------------------------

#[test]
fn package_method_safe_even_with_dangerous_name() -> Result<(), Box<dyn std::error::Error>> {
    ok("MyModule::print()")?;
    ok("MyModule::system()")?;
    ok("MyModule::eval()")?;
    ok("Foo::Bar::Baz::exec()")?;
    Ok(())
}

#[test]
fn sigil_variables_with_dangerous_names_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("$system_call")?;
    ok("$print_output")?;
    ok("$eval_code")?;
    ok("@exec_list")?;
    ok("%open_files")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Error message quality checks
// ---------------------------------------------------------------------------

#[test]
fn error_messages_mention_allow_side_effects() {
    let e = err("system('x')");
    let msg = format!("{e}");
    assert!(msg.contains("allowSideEffects"));
}

#[test]
fn error_includes_operation_name() {
    let e = err("system('x')");
    let msg = format!("{e}");
    assert!(msg.contains("system"));
}

#[test]
fn assignment_error_includes_operator() {
    let e = err("$x = 1");
    assert!(matches!(e, ValidationError::AssignmentOperator(ref op) if op == "="));
    let msg = format!("{e}");
    assert!(msg.contains("="));
}

#[test]
fn regex_mutation_error_includes_operator() {
    let e = err("s/a/b/");
    assert!(matches!(e, ValidationError::RegexMutation(_)));
    let msg = format!("{e}");
    assert!(msg.contains("regex mutation"));
}

// ---------------------------------------------------------------------------
// Combined validation scenarios
// ---------------------------------------------------------------------------

#[test]
fn multiple_dangerous_patterns_first_match_wins() {
    let e = err("system() && print('x')");
    // Should catch system first
    assert!(matches!(e, ValidationError::DangerousOperation(_)));
}

#[test]
fn newline_checked_first() {
    let e = err("system()\neval('x')");
    // Newline should be caught before dangerous ops
    assert!(matches!(e, ValidationError::ContainsNewlines));
}

#[test]
fn backtick_validation_order() {
    let e = err("`ls`");
    assert!(matches!(e, ValidationError::Backticks));
}

// ---------------------------------------------------------------------------
// Empty and whitespace edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_expression_valid() -> Result<(), Box<dyn std::error::Error>> {
    ok("")?;
    Ok(())
}

#[test]
fn whitespace_only_expressions_valid() -> Result<(), Box<dyn std::error::Error>> {
    ok("   ")?;
    ok("\t")?;
    ok("    \t   ")?;
    Ok(())
}

#[test]
fn single_character_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("$")?;
    ok("@")?;
    ok("1")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Numeric and string literal edge cases
// ---------------------------------------------------------------------------

#[test]
fn numeric_literals_all_formats() -> Result<(), Box<dyn std::error::Error>> {
    ok("0")?;
    ok("42")?;
    ok("3.14")?;
    ok("1e10")?;
    ok("0xff")?;
    ok("0o755")?;
    ok("0b1010")?;
    ok("1_000_000")?;
    Ok(())
}

#[test]
fn string_literal_quotes() -> Result<(), Box<dyn std::error::Error>> {
    ok("'string'")?;
    ok("\"string\"")?;
    ok("q(string)")?;
    ok("qq(string)")?;
    ok("qw(a b c)")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Context-dependent expression safety
// ---------------------------------------------------------------------------

#[test]
fn die_and_warn_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("die('msg')")?;
    ok("warn('msg')")?;
    Ok(())
}

#[test]
fn return_statement_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("return $x")?;
    ok("return ($a, $b)")?;
    Ok(())
}

#[test]
fn chomp_and_chop_safe() -> Result<(), Box<dyn std::error::Error>> {
    ok("chomp($str)")?;
    ok("chop($str)")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// SafeEvaluator object properties
// ---------------------------------------------------------------------------

#[test]
fn evaluator_clone_works() {
    let eval1 = SafeEvaluator::new();
    let eval2 = eval1.clone();
    assert!(eval1.validate("$x").is_ok());
    assert!(eval2.validate("$x").is_ok());
}

#[test]
fn evaluator_debug_impl() {
    let evaluator = SafeEvaluator::new();
    let debug_str = format!("{:?}", evaluator);
    assert!(!debug_str.is_empty());
}

#[test]
fn multiple_validations_same_evaluator() -> Result<(), Box<dyn std::error::Error>> {
    let eval = SafeEvaluator::new();
    eval.validate("$x")?;
    eval.validate("$y")?;
    eval.validate("$z")?;
    assert!(eval.validate("eval('x')").is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary conditions for string positions
// ---------------------------------------------------------------------------

#[test]
fn dangerous_op_at_start_of_string() {
    assert!(eval().validate("system('x')").is_err());
}

#[test]
fn dangerous_op_at_end_of_string() {
    assert!(eval().validate("$x = system").is_err());
}

#[test]
fn assignment_at_start() {
    assert!(eval().validate("= 1").is_err());
}

#[test]
fn assignment_at_end() {
    assert!(eval().validate("$x =").is_err());
}

// ---------------------------------------------------------------------------
// Complex nested sigil scenarios
// ---------------------------------------------------------------------------

#[test]
fn nested_sigil_operations() -> Result<(), Box<dyn std::error::Error>> {
    ok("$$ref")?;
    ok("@{$ref}")?;
    ok("%{$ref}")?;
    ok("${$ref}")?;
    ok("@{$ref->[0]}")?;
    Ok(())
}

#[test]
fn sigil_with_package_qualified_not_dangerous() -> Result<(), Box<dyn std::error::Error>> {
    ok("$Package::system")?;
    ok("@Module::print")?;
    ok("%Foo::eval")?;
    Ok(())
}
