//! Comprehensive unit tests for the pure Rust parser

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

    // Helper to check S-expression output
    fn check_sexp(input: &str, expected_contains: &str) {
        let mut parser = PureRustPerlParser::new();
        let ast = parse_successfully(input);
        let sexp = parser.to_sexp(&ast);
        assert!(
            sexp.contains(expected_contains),
            "S-expression '{}' does not contain '{}' for input '{}'",
            sexp,
            expected_contains,
            input
        );
    }

    #[test]
    fn test_basic_literals() {
        // Numbers
        parse_successfully("42");
        parse_successfully("3.14");
        parse_successfully("1e10");
        parse_successfully("0xFF");
        parse_successfully("0b1010");
        parse_successfully("0755");
        
        // Strings
        parse_successfully("'single quoted'");
        parse_successfully("\"double quoted\"");
        parse_successfully("q{custom delimiter}");
        parse_successfully("qq[interpolated]");
        parse_successfully("qw(word list)");
    }

    #[test]
    fn test_variables() {
        // Scalar variables
        parse_successfully("$scalar");
        parse_successfully("$_");
        parse_successfully("$!");
        parse_successfully("${variable}");
        parse_successfully("$Package::variable");
        
        // Array variables
        parse_successfully("@array");
        parse_successfully("@_");
        parse_successfully("@{array}");
        parse_successfully("@Package::array");
        
        // Hash variables
        parse_successfully("%hash");
        parse_successfully("%ENV");
        parse_successfully("%{hash}");
        parse_successfully("%Package::hash");
    }

    #[test]
    fn test_variable_access() {
        // Array access
        parse_successfully("$array[0]");
        parse_successfully("$array[-1]");
        parse_successfully("$array[$index]");
        parse_successfully("$array[$i + 1]");
        
        // Hash access
        parse_successfully("$hash{key}");
        parse_successfully("$hash{'key'}");
        parse_successfully("$hash{$key}");
        parse_successfully("$hash{key1}{key2}");
        
        // Slices
        parse_successfully("@array[0..10]");
        parse_successfully("@hash{'key1', 'key2'}");
        parse_successfully("@array[@indices]");
    }

    #[test]
    fn test_operators() {
        // Arithmetic
        parse_successfully("$a + $b");
        parse_successfully("$a - $b");
        parse_successfully("$a * $b");
        parse_successfully("$a / $b");
        parse_successfully("$a % $b");
        parse_successfully("$a ** $b");
        
        // Comparison
        parse_successfully("$a == $b");
        parse_successfully("$a != $b");
        parse_successfully("$a < $b");
        parse_successfully("$a > $b");
        parse_successfully("$a <= $b");
        parse_successfully("$a >= $b");
        parse_successfully("$a <=> $b");
        
        // String operators
        parse_successfully("$a eq $b");
        parse_successfully("$a ne $b");
        parse_successfully("$a lt $b");
        parse_successfully("$a gt $b");
        parse_successfully("$a le $b");
        parse_successfully("$a ge $b");
        parse_successfully("$a cmp $b");
        parse_successfully("$a . $b");
        parse_successfully("$a x 3");
        
        // Logical
        parse_successfully("$a && $b");
        parse_successfully("$a || $b");
        parse_successfully("!$a");
        parse_successfully("$a and $b");
        parse_successfully("$a or $b");
        parse_successfully("not $a");
        
        // Bitwise
        parse_successfully("$a & $b");
        parse_successfully("$a | $b");
        parse_successfully("$a ^ $b");
        parse_successfully("~$a");
        parse_successfully("$a << 2");
        parse_successfully("$a >> 2");
    }

    #[test]
    fn test_assignments() {
        // Simple assignments
        parse_successfully("$x = 42");
        parse_successfully("@array = (1, 2, 3)");
        parse_successfully("%hash = (key => 'value')");
        
        // Multiple assignments
        parse_successfully("($x, $y) = (1, 2)");
        parse_successfully("($a, $b, @rest) = @array");
        parse_successfully("my ($x, $y) = @_;");
        
        // Augmented assignments
        parse_successfully("$x += 1");
        parse_successfully("$x -= 1");
        parse_successfully("$x *= 2");
        parse_successfully("$x /= 2");
        parse_successfully("$x .= 'suffix'");
        parse_successfully("$x ||= 'default'");
        parse_successfully("$x //= 'default'");
    }

    #[test]
    fn test_control_flow() {
        // If statements
        parse_successfully("if ($x) { print }");
        parse_successfully("if ($x) { print } else { warn }");
        parse_successfully("if ($x) { } elsif ($y) { } else { }");
        
        // Unless
        parse_successfully("unless ($x) { print }");
        parse_successfully("unless ($x) { print } else { warn }");
        
        // While loops
        parse_successfully("while ($x) { print }");
        parse_successfully("while (my $line = <>) { print }");
        parse_successfully("until ($done) { work() }");
        
        // For loops
        parse_successfully("for my $i (1..10) { print $i }");
        parse_successfully("for (@array) { print }");
        parse_successfully("foreach my $item (@list) { process($item) }");
        parse_successfully("for (;;) { last }");
        parse_successfully("for (my $i = 0; $i < 10; $i++) { }");
        
        // Postfix conditions
        parse_successfully("print if $x");
        parse_successfully("die unless $y");
        parse_successfully("next while $condition");
        parse_successfully("last until $done");
    }

    #[test]
    fn test_subroutines() {
        // Basic subroutines
        parse_successfully("sub foo { }");
        parse_successfully("sub foo { return 42 }");
        parse_successfully("sub foo { my $x = shift; return $x }");
        
        // With prototypes
        parse_successfully("sub foo($) { }");
        parse_successfully("sub foo($$@) { }");
        parse_successfully("sub foo(&@) { }");
        
        // With attributes
        parse_successfully("sub foo :lvalue { }");
        parse_successfully("sub foo :method :lvalue { }");
        
        // Anonymous subs
        parse_successfully("sub { }");
        parse_successfully("sub { return 42 }");
        parse_successfully("my $ref = sub { };");
    }

    #[test]
    fn test_regular_expressions() {
        // Match
        parse_successfully("/pattern/");
        parse_successfully("m/pattern/");
        parse_successfully("m{pattern}");
        parse_successfully("/pattern/i");
        parse_successfully("/pattern/gims");
        
        // Substitution
        parse_successfully("s/old/new/");
        parse_successfully("s{old}{new}");
        parse_successfully("s/old/new/g");
        parse_successfully("s/old/new/gims");
        
        // Transliteration
        parse_successfully("tr/a-z/A-Z/");
        parse_successfully("tr{a-z}{A-Z}");
        parse_successfully("y/a-z/A-Z/");
        
        // Quoted regex
        parse_successfully("qr/pattern/");
        parse_successfully("qr{pattern}i");
        
        // Match operators
        parse_successfully("$x =~ /pattern/");
        parse_successfully("$x !~ /pattern/");
        parse_successfully("$x =~ s/old/new/");
    }

    #[test]
    fn test_references() {
        // Taking references
        parse_successfully("\\$scalar");
        parse_successfully("\\@array");
        parse_successfully("\\%hash");
        parse_successfully("\\&sub");
        parse_successfully("\\*glob");
        
        // Dereferencing
        parse_successfully("$$ref");
        parse_successfully("@$ref");
        parse_successfully("%$ref");
        parse_successfully("&$ref");
        parse_successfully("*$ref");
        
        // Arrow dereferencing
        parse_successfully("$ref->[0]");
        parse_successfully("$ref->{key}");
        parse_successfully("$ref->()");
        parse_successfully("$ref->method()");
        
        // Complex dereferencing
        parse_successfully("$ref->[0]{key}[1]");
        parse_successfully("${$ref}");
        parse_successfully("@{$ref}");
        parse_successfully("%{$ref}");
    }

    #[test]
    fn test_special_constructs() {
        // Do blocks
        parse_successfully("do { }");
        parse_successfully("do { my $x = 1; $x }");
        
        // Eval
        parse_successfully("eval { }");
        parse_successfully("eval 'code'");
        parse_successfully("eval { die }; print $@");
        
        // Require/use
        parse_successfully("require Module");
        parse_successfully("require 'file.pl'");
        parse_successfully("use Module");
        parse_successfully("use Module qw(import list)");
        parse_successfully("use Module 1.23");
        
        // Package declarations
        parse_successfully("package Foo;");
        parse_successfully("package Foo::Bar;");
        parse_successfully("package Foo 1.23;");
        parse_successfully("package Foo { }");
    }

    #[test]
    fn test_list_operations() {
        // List construction
        parse_successfully("(1, 2, 3)");
        parse_successfully("(1..10)");
        parse_successfully("('a'..'z')");
        parse_successfully("(1, 2, @array, 3, 4)");
        
        // Fat comma
        parse_successfully("(key => 'value')");
        parse_successfully("(a => 1, b => 2)");
        parse_successfully("func(arg => 'value')");
        
        // List operators
        parse_successfully("sort @array");
        parse_successfully("sort { $a <=> $b } @array");
        parse_successfully("grep { $_ > 0 } @array");
        parse_successfully("map { $_ * 2 } @array");
        parse_successfully("reverse @array");
    }

    #[test]
    fn test_error_cases() {
        // Syntax errors
        parse_fails("$");
        parse_fails("@");
        parse_fails("%");
        parse_fails("sub");
        parse_fails("if () {");
        parse_fails("{ ]");
        parse_fails("my $x =");
    }

    #[test]
    fn test_complex_expressions() {
        // Nested expressions
        parse_successfully("$a + $b * $c - $d / $e");
        parse_successfully("($a || $b) && ($c || $d)");
        parse_successfully("$hash{$array[$index + 1]}");
        
        // Ternary operator
        parse_successfully("$x ? $y : $z");
        parse_successfully("$a > $b ? $a : $b");
        parse_successfully("$x ? $y : $z ? $w : $v");
        
        // Chained operations
        parse_successfully("$obj->method1()->method2()->method3()");
        parse_successfully("$ref->[0]->{key}->[1]");
    }

    #[test]
    fn test_heredocs() {
        // Basic heredocs
        parse_successfully("<<EOF\nHello\nWorld\nEOF");
        parse_successfully("<<'EOF'\nNo interpolation\nEOF");
        parse_successfully("<<\"EOF\"\nWith interpolation $var\nEOF");
        
        // Multiple heredocs
        parse_successfully("print <<EOF, <<'END';\nFirst\nEOF\nSecond\nEND");
    }

    #[test]
    fn test_filehandles() {
        // Filehandle operations
        parse_successfully("open(FH, '<', 'file.txt')");
        parse_successfully("open(my $fh, '>', 'file.txt')");
        parse_successfully("print FH 'content'");
        parse_successfully("print $fh 'content'");
        parse_successfully("<FH>");
        parse_successfully("<$fh>");
        parse_successfully("close(FH)");
    }

    #[test]
    fn test_special_literals() {
        // Special literals
        parse_successfully("__FILE__");
        parse_successfully("__LINE__");
        parse_successfully("__PACKAGE__");
        parse_successfully("__SUB__");
        parse_successfully("__END__");
        parse_successfully("__DATA__");
    }

    #[test]
    fn test_pragmas() {
        // Common pragmas
        parse_successfully("use strict;");
        parse_successfully("use warnings;");
        parse_successfully("use feature 'say';");
        parse_successfully("no warnings 'uninitialized';");
        parse_successfully("use v5.32;");
        parse_successfully("use 5.032;");
    }

    #[test]
    fn test_statement_modifiers() {
        // All statement modifiers
        parse_successfully("print if $x;");
        parse_successfully("print unless $x;");
        parse_successfully("print while $x;");
        parse_successfully("print until $x;");
        parse_successfully("print for @list;");
        parse_successfully("print when $x;");
    }

    #[test]
    fn test_typeglobs() {
        // Typeglob operations
        parse_successfully("*foo");
        parse_successfully("*foo = *bar");
        parse_successfully("*foo{SCALAR}");
        parse_successfully("*foo{ARRAY}");
        parse_successfully("*foo{HASH}");
        parse_successfully("*foo{CODE}");
        parse_successfully("local *FH");
    }

    #[test]
    fn test_edge_cases() {
        // Empty constructs
        parse_successfully("");
        parse_successfully(";");
        parse_successfully(";;");
        parse_successfully("{}");
        parse_successfully("sub {}");
        
        // Whitespace handling
        parse_successfully("  $x  =  42  ;  ");
        parse_successfully("\t\t$x\t=\t42\t;\t");
        parse_successfully("\n\n$x = 42;\n\n");
        
        // Comments
        parse_successfully("# comment");
        parse_successfully("$x = 42; # comment");
        parse_successfully("# comment\n$x = 42;");
        parse_successfully("$x = # comment\n42;");
    }

    #[test]
    fn test_s_expression_output() {
        // Test specific S-expression patterns
        check_sexp("$x", "scalar_variable");
        check_sexp("@array", "array_variable");
        check_sexp("%hash", "hash_variable");
        check_sexp("42", "integer");
        check_sexp("3.14", "float");
        check_sexp("'string'", "string_literal");
        check_sexp("$x = 42", "assignment");
        check_sexp("sub foo { }", "subroutine");
        check_sexp("if ($x) { }", "if_statement");
        check_sexp("/pattern/", "regex");
    }

    #[test]
    fn test_unicode_support() {
        // Unicode identifiers
        parse_successfully("my $café = 'coffee';");
        parse_successfully("sub π { 3.14159 }");
        parse_successfully("my $привет = 'hello';");
        
        // Unicode in strings
        parse_successfully("'café'");
        parse_successfully("\"Hello 世界\"");
        parse_successfully("qw(α β γ)");
        
        // Unicode in regex
        parse_successfully("/café/");
        parse_successfully("/\\p{Letter}/");
        parse_successfully("/\\x{263A}/");
    }

    #[test]
    fn test_parser_reuse() {
        // Test that parser can be reused
        let mut parser = PureRustPerlParser::new();
        
        // Parse multiple inputs
        assert!(parser.parse("$x = 1").is_ok());
        assert!(parser.parse("@array = (1, 2, 3)").is_ok());
        assert!(parser.parse("sub foo { }").is_ok());
        
        // Parse with errors
        assert!(parser.parse("$").is_err());
        
        // Should still work after error
        assert!(parser.parse("$x = 42").is_ok());
    }

    #[test]
    fn test_performance_characteristics() {
        use std::time::Instant;
        
        let mut parser = PureRustPerlParser::new();
        
        // Simple parse
        let start = Instant::now();
        let _ = parser.parse("$x = 42");
        let simple_time = start.elapsed();
        
        // Complex parse
        let complex = r#"
            sub complex_function {
                my ($x, $y, @rest) = @_;
                my %hash = (
                    key1 => 'value1',
                    key2 => [1, 2, 3],
                    key3 => { nested => 'hash' }
                );
                
                for my $item (@rest) {
                    if ($item =~ /pattern/) {
                        $hash{matches}++;
                    }
                }
                
                return wantarray ? %hash : \%hash;
            }
        "#;
        
        let start = Instant::now();
        let _ = parser.parse(complex);
        let complex_time = start.elapsed();
        
        // Complex parse should take more time but not excessively so
        assert!(complex_time < simple_time * 100, 
                "Complex parse took too long: {:?} vs {:?}", 
                complex_time, simple_time);
    }
}