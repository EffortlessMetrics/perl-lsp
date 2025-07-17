//! Unit tests for the pure Rust parser that match current grammar capabilities

#[cfg(test)]
mod tests {
    use crate::pure_rust_parser::{PureRustPerlParser, AstNode};

    // Helper function to parse and check success
    fn parse_successfully(input: &str) -> AstNode {
        let mut parser = PureRustPerlParser::new();
        match parser.parse(input) {
            Ok(ast) => ast,
            Err(e) => panic!("Parse failed for '{}': {}", input, e),
        }
    }

    // Helper function to check parse failure
    fn parse_fails(input: &str) {
        let mut parser = PureRustPerlParser::new();
        assert!(parser.parse(input).is_err(), "Expected parse to fail for: {}", input);
    }

    // Helper to check S-expression output contains pattern
    fn check_sexp_contains(input: &str, expected_pattern: &str) {
        let mut parser = PureRustPerlParser::new();
        let ast = parse_successfully(input);
        let sexp = parser.to_sexp(&ast);
        assert!(
            sexp.contains(expected_pattern),
            "S-expression '{}' does not contain '{}' for input '{}'",
            sexp,
            expected_pattern,
            input
        );
    }

    #[test]
    fn test_basic_numbers() {
        // Integer literals
        parse_successfully("42");
        parse_successfully("0");
        parse_successfully("123456789");
        
        // Float literals
        parse_successfully("3.14");
        parse_successfully("0.5");
        parse_successfully("123.456");
        
        // Hex numbers
        parse_successfully("0xFF");
        parse_successfully("0x1234");
        parse_successfully("0xABCDEF");
        
        // Binary numbers
        parse_successfully("0b1010");
        parse_successfully("0b11111111");
        
        // Octal numbers
        parse_successfully("0755");
        parse_successfully("0644");
    }

    #[test]
    fn test_basic_strings() {
        // Single quoted strings
        parse_successfully("'hello'");
        parse_successfully("'hello world'");
        parse_successfully(r#"'don\'t'"#);
        
        // Double quoted strings
        parse_successfully("\"hello\"");
        parse_successfully("\"hello world\"");
        parse_successfully(r#""hello\nworld""#);
        
        // q-style strings
        parse_successfully("q{hello}");
        parse_successfully("qq{hello world}");
    }

    #[test]
    fn test_variables() {
        // Scalar variables
        parse_successfully("$var");
        parse_successfully("$_");
        parse_successfully("$a");
        parse_successfully("$1");
        
        // Array variables
        parse_successfully("@array");
        parse_successfully("@_");
        parse_successfully("@ARGV");
        
        // Hash variables
        parse_successfully("%hash");
        parse_successfully("%ENV");
    }

    #[test]
    fn test_simple_expressions() {
        // Basic arithmetic
        parse_successfully("$a + $b");
        parse_successfully("$a - $b");
        parse_successfully("$a * $b");
        parse_successfully("$a / $b");
        parse_successfully("$a % $b");
        
        // Comparison
        parse_successfully("$a == $b");
        parse_successfully("$a != $b");
        parse_successfully("$a < $b");
        parse_successfully("$a > $b");
        parse_successfully("$a <= $b");
        parse_successfully("$a >= $b");
        
        // String operators
        parse_successfully("$a . $b");
        parse_successfully("$a eq $b");
        parse_successfully("$a ne $b");
        
        // Logical operators
        parse_successfully("$a && $b");
        parse_successfully("$a || $b");
        parse_successfully("!$a");
    }

    #[test]
    fn test_assignments() {
        // Simple scalar assignment
        parse_successfully("$x = 42");
        parse_successfully("$x = 'hello'");
        parse_successfully("$x = $y");
        
        // Array assignment
        parse_successfully("@array = (1, 2, 3)");
        parse_successfully("@array = ()");
        
        // Hash assignment  
        parse_successfully("%hash = (a => 1, b => 2)");
        parse_successfully("%hash = ()");
        
        // Augmented assignments
        parse_successfully("$x += 1");
        parse_successfully("$x -= 1");
        parse_successfully("$x *= 2");
        parse_successfully("$x /= 2");
        parse_successfully("$x .= 'suffix'");
    }

    #[test]
    fn test_lists() {
        // Empty list
        parse_successfully("()");
        
        // Simple lists
        parse_successfully("(1)");
        parse_successfully("(1, 2)");
        parse_successfully("(1, 2, 3)");
        
        // Lists with variables
        parse_successfully("($a)");
        parse_successfully("($a, $b)");
        parse_successfully("($a, $b, $c)");
        
        // Lists with fat comma
        parse_successfully("(a => 1)");
        parse_successfully("(a => 1, b => 2)");
        
        // Mixed lists
        parse_successfully("(1, 'hello', $var)");
    }

    #[test]
    fn test_basic_statements() {
        // Expression statements
        parse_successfully("42;");
        parse_successfully("$x = 42;");
        parse_successfully("$x + $y;");
        
        // Multiple statements
        parse_successfully("$x = 1; $y = 2;");
        parse_successfully("$x = 1; $y = 2; $z = 3;");
        
        // Empty statements
        parse_successfully(";");
        parse_successfully(";;");
    }

    #[test]
    fn test_control_flow() {
        // Basic if
        parse_successfully("if ($x) { $y = 1; }");
        parse_successfully("if ($x == 1) { $y = 2; }");
        
        // If-else
        parse_successfully("if ($x) { $y = 1; } else { $y = 2; }");
        
        // While loops
        parse_successfully("while ($x) { $x--; }");
        parse_successfully("while ($x > 0) { $x = $x - 1; }");
        
        // For loops
        parse_successfully("for ($i = 0; $i < 10; $i++) { $sum += $i; }");
        
        // Foreach
        parse_successfully("foreach $item (@list) { print $item; }");
        parse_successfully("for $x (@array) { $sum += $x; }");
    }

    #[test]
    fn test_subroutines() {
        // Basic subroutine
        parse_successfully("sub foo { }");
        parse_successfully("sub foo { return 42; }");
        parse_successfully("sub foo { my $x = shift; return $x; }");
        
        // Subroutine with statements
        parse_successfully("sub add { my ($a, $b) = @_; return $a + $b; }");
        
        // Anonymous subroutines
        parse_successfully("sub { }");
        parse_successfully("sub { return 42; }");
        parse_successfully("$ref = sub { return 42; };");
    }

    #[test]
    fn test_function_calls() {
        // No-argument functions
        parse_successfully("foo()");
        parse_successfully("foo");
        
        // With arguments
        parse_successfully("foo(1)");
        parse_successfully("foo(1, 2)");
        parse_successfully("foo($x, $y)");
        
        // Built-in functions
        parse_successfully("print 'hello'");
        parse_successfully("print");
        parse_successfully("shift");
        parse_successfully("pop");
    }

    #[test]
    fn test_blocks() {
        // Empty block
        parse_successfully("{ }");
        
        // Block with statements
        parse_successfully("{ $x = 1; }");
        parse_successfully("{ $x = 1; $y = 2; }");
        
        // Nested blocks
        parse_successfully("{ { } }");
        parse_successfully("{ $x = 1; { $y = 2; } }");
    }

    #[test]
    fn test_comments() {
        // Just comments
        parse_successfully("# comment");
        parse_successfully("# comment 1\n# comment 2");
        
        // Comments with code
        parse_successfully("$x = 42; # comment");
        parse_successfully("# comment\n$x = 42;");
        parse_successfully("$x = 42;\n# comment\n$y = 43;");
    }

    #[test]
    fn test_package_declarations() {
        // Simple package
        parse_successfully("package Foo;");
        parse_successfully("package Foo::Bar;");
        parse_successfully("package Foo::Bar::Baz;");
        
        // Package with code
        parse_successfully("package Foo; $x = 42;");
        parse_successfully("package Foo; sub bar { }");
    }

    #[test]
    fn test_array_hash_access() {
        // Array element access
        parse_successfully("$array[0]");
        parse_successfully("$array[1]");
        parse_successfully("$array[$i]");
        
        // Hash element access
        parse_successfully("$hash{key}");
        parse_successfully("$hash{'key'}");
        parse_successfully("$hash{$key}");
    }

    #[test]
    fn test_special_literals() {
        // Barewords (identifiers)
        parse_successfully("foo");
        parse_successfully("bar");
        parse_successfully("foo_bar");
        parse_successfully("FooBar");
    }

    #[test]
    fn test_return_statements() {
        parse_successfully("return");
        parse_successfully("return 42");
        parse_successfully("return $x");
        parse_successfully("return ($x, $y)");
        parse_successfully("return $x + $y");
    }

    #[test]
    fn test_my_declarations() {
        // Simple my
        parse_successfully("my $x");
        parse_successfully("my $x = 42");
        parse_successfully("my @array");
        parse_successfully("my %hash");
        
        // Multiple declarations
        parse_successfully("my ($x, $y)");
        parse_successfully("my ($x, $y) = (1, 2)");
        parse_successfully("my ($x, $y, $z)");
    }

    #[test]
    fn test_postfix_operators() {
        // Increment/decrement
        parse_successfully("$x++");
        parse_successfully("$x--");
        parse_successfully("$count++");
        
        // In expressions
        parse_successfully("$x = $y++");
        parse_successfully("$array[$i++]");
    }

    #[test]
    fn test_ternary_operator() {
        parse_successfully("$x ? $y : $z");
        parse_successfully("$a > $b ? $a : $b");
        parse_successfully("$x == 1 ? 'one' : 'other'");
    }

    #[test]
    fn test_simple_regex() {
        // Basic match
        parse_successfully("/pattern/");
        parse_successfully("/hello/");
        parse_successfully(r#"/\d+/"#);
        
        // With flags
        parse_successfully("/pattern/i");
        parse_successfully("/pattern/g");
        parse_successfully("/pattern/igms");
    }

    #[test]
    fn test_parenthesized_expressions() {
        parse_successfully("($x)");
        parse_successfully("($x + $y)");
        parse_successfully("($x + $y) * $z");
        parse_successfully("$x * ($y + $z)");
    }

    #[test]
    fn test_error_recovery() {
        // Missing semicolons - should still parse
        parse_successfully("$x = 42");
        parse_successfully("$x = 42\n$y = 43");
        
        // Multiple semicolons
        parse_successfully("$x = 42;;");
        parse_successfully(";;;");
    }

    #[test]
    fn test_s_expression_output() {
        // Basic checks
        check_sexp_contains("$x", "scalar_variable");
        check_sexp_contains("@array", "array_variable");
        check_sexp_contains("%hash", "hash_variable");
        check_sexp_contains("42", "number");
        check_sexp_contains("'string'", "string_literal");
        check_sexp_contains("$x = 42", "assignment");
        check_sexp_contains("sub foo { }", "subroutine");
        check_sexp_contains("if ($x) { }", "if_statement");
    }

    #[test]
    fn test_parser_reuse() {
        let mut parser = PureRustPerlParser::new();
        
        // Parse multiple times
        assert!(parser.parse("$x = 1").is_ok());
        assert!(parser.parse("@array = (1, 2, 3)").is_ok());
        assert!(parser.parse("sub foo { }").is_ok());
        
        // Should handle empty input
        assert!(parser.parse("").is_ok());
        
        // Should continue working after empty input
        assert!(parser.parse("$x = 42").is_ok());
    }

    #[test]
    fn test_whitespace_handling() {
        // Various whitespace
        parse_successfully("  $x  =  42  ");
        parse_successfully("\t$x\t=\t42\t");
        parse_successfully("\n$x = 42\n");
        parse_successfully("$x\n=\n42");
        
        // No whitespace
        parse_successfully("$x=42");
        parse_successfully("$x=$y+$z");
    }

    #[test]
    fn test_nested_structures() {
        // Nested parentheses
        parse_successfully("((($x)))");
        parse_successfully("(($x + $y) * ($z - $w))");
        
        // Nested blocks
        parse_successfully("{ { { } } }");
        parse_successfully("{ $x = 1; { $y = 2; { $z = 3; } } }");
        
        // Nested control flow
        parse_successfully("if ($x) { if ($y) { $z = 1; } }");
        parse_successfully("while ($x) { while ($y) { $z++; } }");
    }
}