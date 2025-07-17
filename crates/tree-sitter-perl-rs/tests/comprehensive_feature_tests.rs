//! Comprehensive tests for all new pure Rust parser features

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::pure_rust_parser::{PerlParser, Rule};
    use pest::Parser;

    fn assert_parses(input: &str) {
        let result = PerlParser::parse(Rule::program, input);
        assert!(result.is_ok(), "Failed to parse: {}\nError: {:?}", input, result.err());
    }

    fn assert_parse_fails(input: &str) {
        let result = PerlParser::parse(Rule::program, input);
        assert!(result.is_err(), "Expected parse to fail but it succeeded: {}", input);
    }

    #[test]
    fn test_scalar_references_comprehensive() {
        let test_cases = vec![
            // Basic scalar references
            r#"my $ref = \$scalar;"#,
            r#"my $ref = \$_;"#,
            r#"my $ref = \$1;"#,
            r#"my $ref = \$#array;"#,
            r#"my $ref = \${var};"#,
            r#"my $ref = \$hash{key};"#,
            r#"my $ref = \$array[0];"#,
            
            // Multiple references
            r#"my ($ref1, $ref2) = (\$var1, \$var2);"#,
            
            // Nested references
            r#"my $ref_ref = \\$scalar;"#,
            r#"my $ref = \${$scalar_ref};"#,
            
            // In expressions
            r#"print \$var;"#,
            r#"push @refs, \$item;"#,
            r#"$hash{ref} = \$value;"#,
            
            // Dereferencing
            r#"my $value = $$ref;"#,
            r#"my $value = ${$ref};"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_array_references_comprehensive() {
        let test_cases = vec![
            // Basic array references
            r#"my $aref = \@array;"#,
            r#"my $aref = \@_;"#,
            r#"my $aref = \@ARGV;"#,
            r#"my $aref = \@{$array_ref};"#,
            
            // Array slices
            r#"my $ref = \@array[0..5];"#,
            r#"my $ref = \@hash{@keys};"#,
            
            // Anonymous arrays
            r#"my $aref = [1, 2, 3];"#,
            r#"my $aref = [];"#,
            r#"my $aref = [qw(a b c)];"#,
            r#"my $aref = [$x, $y, $z];"#,
            r#"my $nested = [[1, 2], [3, 4]];"#,
            
            // Dereferencing
            r#"my @array = @$aref;"#,
            r#"my @array = @{$aref};"#,
            r#"my $elem = $aref->[0];"#,
            r#"my $elem = $$aref[0];"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_hash_references_comprehensive() {
        let test_cases = vec![
            // Basic hash references
            r#"my $href = \%hash;"#,
            r#"my $href = \%ENV;"#,
            r#"my $href = \%{$hash_ref};"#,
            
            // Anonymous hashes
            r#"my $href = { key => 'value' };"#,
            r#"my $href = {};"#,
            r#"my $href = { a => 1, b => 2 };"#,
            r#"my $href = { 'key' => $value };"#,
            r#"my $nested = { a => { b => 'c' } };"#,
            
            // Hash slices
            r#"my $ref = \%hash{@keys};"#,
            
            // Dereferencing
            r#"my %hash = %$href;"#,
            r#"my %hash = %{$href};"#,
            r#"my $val = $href->{key};"#,
            r#"my $val = $$href{key};"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_subroutine_and_glob_references() {
        let test_cases = vec![
            // Subroutine references
            r#"my $sub_ref = \&function;"#,
            r#"my $sub_ref = \&Some::Module::function;"#,
            r#"my $sub_ref = sub { return 42; };"#,
            r#"my $sub_ref = sub { my $x = shift; return $x * 2; };"#,
            r#"my $closure = sub { my $x = $outer; return $x; };"#,
            
            // Calling subroutine references
            r#"$sub_ref->();"#,
            r#"&$sub_ref();"#,
            r#"$sub_ref->($arg1, $arg2);"#,
            
            // Glob references
            r#"my $glob_ref = \*STDOUT;"#,
            r#"my $glob_ref = \*Some::Package::VAR;"#,
            r#"my $fh_ref = \*FH;"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_string_interpolation_comprehensive() {
        let test_cases = vec![
            // Basic interpolation
            r#"my $str = "$var";"#,
            r#"my $str = "@array";"#,
            r#"my $str = "$hash{key}";"#,
            r#"my $str = "$array[0]";"#,
            
            // Complex scalar interpolation
            r#"my $str = "${var}";"#,
            r#"my $str = "${var}s";"#,
            r#"my $str = "${var}_suffix";"#,
            r#"my $str = "prefix_${var}_suffix";"#,
            r#"my $str = "${hash{key}}";"#,
            r#"my $str = "${$scalar_ref}";"#,
            
            // Complex array interpolation
            r#"my $str = "@{[1, 2, 3]}";"#,
            r#"my $str = "@{[$x, $y, $z]}";"#,
            r#"my $str = "@{[qw(a b c)]}";"#,
            r#"my $str = "Items: @{[1..10]}";"#,
            r#"my $str = "@{[ sort @array ]}";"#,
            
            // Mixed interpolation
            r#"my $str = "Hello ${name}, you have @{[$count + 1]} items";"#,
            r#"my $str = "${user}'s items: @{[ keys %items ]}";"#,
            
            // Edge cases
            r#"my $str = "\$not_interpolated";"#,
            r#"my $str = "\\${also_not}";"#,
            r#"my $str = "$var\n$var2";"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_regex_patterns_comprehensive() {
        let test_cases = vec![
            // Basic regex
            r#"if (/pattern/) { }"#,
            r#"if ($text =~ /pattern/) { }"#,
            r#"if ($text !~ /pattern/) { }"#,
            
            // Character classes
            r#"if (/\w+/) { }"#,
            r#"if (/\d+/) { }"#,
            r#"if (/\s+/) { }"#,
            r#"if (/\W+/) { }"#,
            r#"if (/\D+/) { }"#,
            r#"if (/\S+/) { }"#,
            
            // Quantifiers
            r#"if (/a*/) { }"#,
            r#"if (/a+/) { }"#,
            r#"if (/a?/) { }"#,
            r#"if (/a{3}/) { }"#,
            r#"if (/a{3,}/) { }"#,
            r#"if (/a{3,5}/) { }"#,
            
            // Anchors
            r#"if (/^start/) { }"#,
            r#"if (/end$/) { }"#,
            r#"if (/\bword\b/) { }"#,
            r#"if (/\Bnot\B/) { }"#,
            
            // Groups
            r#"if (/(group)/) { }"#,
            r#"if (/(?:non-capturing)/) { }"#,
            r#"if (/(?<name>pattern)/) { }"#,
            r#"if (/(?=lookahead)/) { }"#,
            r#"if (/(?!negative)/) { }"#,
            
            // Modifiers
            r#"if (/pattern/i) { }"#,
            r#"if (/pattern/gims) { }"#,
            r#"if (/pattern/x) { }"#,
            r#"if (/pattern/msixpogcual) { }"#,
            
            // qr operator
            r#"my $re = qr/pattern/;"#,
            r#"my $re = qr/\w+/;"#,
            r#"my $re = qr/\d{2,4}/;"#,
            r#"my $re = qr/\s*\n/;"#,
            r#"my $re = qr/\w+\s*/;"#,
            r#"my $re = qr/(?<word>\w+)/;"#,
            r#"my $re = qr/pattern/ims;"#,
            
            // Different delimiters
            r#"my $re = qr!pattern!;"#,
            r#"my $re = qr#pattern#;"#,
            r#"my $re = qr{pattern};"#,
            r#"my $re = qr[pattern];"#,
            r#"my $re = qr(pattern);"#,
            
            // Complex patterns
            r#"if (/\w+\s*=\s*\d+/) { }"#,
            r#"if (/^[a-zA-Z_]\w*$/) { }"#,
            r#"if (/(?:https?|ftp):\/\//) { }"#,
            r#"my $re = qr/\$\{?\w+\}?/;"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_operators_comprehensive() {
        let test_cases = vec![
            // Arithmetic
            r#"$x = $y + $z;"#,
            r#"$x = $y - $z;"#,
            r#"$x = $y * $z;"#,
            r#"$x = $y / $z;"#,
            r#"$x = $y % $z;"#,
            r#"$x = $y ** $z;"#,
            r#"$x = $y ** 2;"#,
            r#"$x = 2 ** 10;"#,
            r#"$x = $base ** $exponent;"#,
            
            // String operators
            r#"$x = $y . $z;"#,
            r#"$x = $y x 3;"#,
            
            // Comparison
            r#"if ($x == $y) { }"#,
            r#"if ($x != $y) { }"#,
            r#"if ($x < $y) { }"#,
            r#"if ($x > $y) { }"#,
            r#"if ($x <= $y) { }"#,
            r#"if ($x >= $y) { }"#,
            r#"if ($x <=> $y) { }"#,
            r#"if ($x eq $y) { }"#,
            r#"if ($x ne $y) { }"#,
            r#"if ($x lt $y) { }"#,
            r#"if ($x gt $y) { }"#,
            r#"if ($x le $y) { }"#,
            r#"if ($x ge $y) { }"#,
            r#"if ($x cmp $y) { }"#,
            r#"if ($x ~~ $y) { }"#,
            
            // Logical
            r#"if ($x && $y) { }"#,
            r#"if ($x || $y) { }"#,
            r#"if (!$x) { }"#,
            r#"if ($x and $y) { }"#,
            r#"if ($x or $y) { }"#,
            r#"if (not $x) { }"#,
            r#"$x = $y // $z;"#,
            
            // Bitwise
            r#"$x = $y & $z;"#,
            r#"$x = $y | $z;"#,
            r#"$x = $y ^ $z;"#,
            r#"$x = ~$y;"#,
            r#"$x = $y << 2;"#,
            r#"$x = $y >> 2;"#,
            
            // Assignment
            r#"$x = $y;"#,
            r#"$x += $y;"#,
            r#"$x -= $y;"#,
            r#"$x *= $y;"#,
            r#"$x /= $y;"#,
            r#"$x %= $y;"#,
            r#"$x **= $y;"#,
            r#"$x .= $y;"#,
            r#"$x &&= $y;"#,
            r#"$x ||= $y;"#,
            r#"$x //= $y;"#,
            
            // Range
            r#"@range = (1..10);"#,
            r#"@range = (1...10);"#,
            r#"@range = ('a'..'z');"#,
            
            // Ternary
            r#"$x = $cond ? $true : $false;"#,
            r#"$x = $a ? $b : $c ? $d : $e;"#,
            
            // Increment/decrement
            r#"$x++;"#,
            r#"++$x;"#,
            r#"$x--;"#,
            r#"--$x;"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_heredoc_declarations() {
        let test_cases = vec![
            // Basic heredocs
            r#"print <<EOF;"#,
            r#"print <<'EOF';"#,
            r#"print <<"EOF";"#,
            r#"print <<`EOF`;"#,
            r#"print <<\EOF;"#,
            
            // Indented heredocs
            r#"print <<~EOF;"#,
            r#"print <<~'EOF';"#,
            r#"print <<~"EOF";"#,
            
            // Multiple heredocs - TODO: complex case
            // r#"print <<EOF1, <<EOF2;"#,
            
            // In assignments
            r#"my $text = <<EOF;"#,
            r#"my $text = <<'HEREDOC';"#,
            
            // With different markers
            r#"print <<END_OF_TEXT;"#,
            r#"print <<__DATA__;"#,
            r#"print <<'SQL';"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_complex_combinations() {
        let test_cases = vec![
            // References with interpolation
            r#"my $str = "Ref: ${$scalar_ref}";"#,
            r#"my $str = "Array: @{$array_ref}";"#,
            r#"my $str = "Hash keys: @{[ keys %{$hash_ref} ]}";"#,
            
            // Regex with references
            r#"if ($$text_ref =~ /\w+/) { }"#,
            r#"my $re_ref = \qr/pattern/;"#,
            r#"if ($text =~ $$re_ref) { }"#,
            
            // Complex expressions with operators
            r#"my $result = ($x ** 2) + ($y ** 2);"#,
            r#"my $is_valid = ($x > 0) && ($y > 0) && ($x ** 2 + $y ** 2 < $radius ** 2);"#,
            
            // Anonymous structures with references
            r#"my $data = { array => [1, 2, 3], hash => { a => \$x, b => \@y } };"#,
            r#"my $complex = [ \$scalar, \@array, \%hash, \&sub, sub { $x ** 2 } ];"#,
            
            // String interpolation with expressions
            r#"my $msg = "Result: @{[ $x ** 2 + $y ** 2 ]}";"#,
            r#"my $info = "${name}'s score: @{[ int($score ** 0.5 * 100) ]}%";"#,
            
            // Nested references
            r#"my $ref_to_ref_to_array = \\@array;"#,
            r#"my $value = ${${$ref_to_ref}};"#,
            
            // Complex regex patterns
            r#"my $email_re = qr/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;"#,
            r#"if ($url =~ qr{^https?://(?:www\.)?([^/]+)}) { }"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_edge_cases_and_corner_cases() {
        let test_cases = vec![
            // Empty constructs
            r#"my $aref = [];"#,
            r#"my $href = {};"#,
            r#"my $code = sub { };"#,
            
            // Unicode in strings
            r#"my $str = "Hello 世界";"#,
            r#"my $str = "${var}™";"#,
            
            // Special variables
            r#"my $ref = \$_;"#,
            r#"my $ref = \@_;"#,
            r#"my $ref = \%ENV;"#,
            r#"my $ref = \$@;"#,
            r#"my $ref = \$!;"#,
            
            // Multiple operations
            r#"my $x = 2 ** 3 ** 2;"#,  // Right associative
            r#"my $str = "a" . "b" . "c";"#,
            
            // Complex nesting
            r#"my $x = ${${$ref{key}[0]}};"#,
            r#"my $str = "@{[map { $_ ** 2 } @{$aref}]}";"#,
        ];

        for code in test_cases {
            assert_parses(code);
        }
    }

    #[test]
    fn test_parse_failures() {
        let test_cases = vec![
            // Invalid references
            r#"my $ref = \;"#,
            r#"my $ref = \123;"#,
            
            // Invalid interpolation
            r#"my $str = "${}";"#,
            r#"my $str = "@{[]";"#,  // Missing closing
            
            // Invalid regex
            r#"my $re = qr/(/;"#,  // Unmatched paren
            
            // Invalid operators
            r#"$x = $y *** $z;"#,  // Triple star
        ];

        for code in test_cases {
            assert_parse_fails(code);
        }
    }
}