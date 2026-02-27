//! Comprehensive tests for all new pure Rust parser features

#[cfg(feature = "pure-rust")]
mod tests {
    use pest::Parser;
    use tree_sitter_perl::pure_rust_parser::{PerlParser, Rule};

    fn assert_parses(input: &str) {
        let result = PerlParser::parse(Rule::program, input);
        assert!(result.is_ok(), "Failed to parse: {}\nError: {:?}", input, result.err());
    }

    fn assert_parse_fails(input: &str) {
        let result = PerlParser::parse(Rule::program, input);
        assert!(result.is_err(), "Expected parse failure for: {}", input);
    }

    #[test]
    fn test_scalar_references_comprehensive() {
        let test_cases = vec![
            // Basic scalar references
            r#"my $ref = \$scalar;"#,
            r#"my $ref = \$_;"#,
            r#"my $ref = \$::global;"#,
            r#"my $ref = \$Package::var;"#,
            // Complex scalar references
            r#"my $ref = \${var};"#,
            r#"my $ref = \${"complex"};"#,
            r#"my $ref = \$$other_ref;"#,
            // Scalar references in expressions
            r#"print $$ref;"#,
            r#"$$ref = 42;"#,
            r#"my $val = $$ref + 10;"#,
            r#"${$ref} = "value";"#,
            // Multiple dereferences
            r#"my $val = $$$ref_ref;"#,
            r#"${${$ref}} = "nested";"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_array_references_comprehensive() {
        let test_cases = vec![
            // Basic array references
            r#"my $aref = \@array;"#,
            r#"my $aref = \@_;"#,
            r#"my $aref = \@::global;"#,
            r#"my $aref = \@Package::array;"#,
            // Complex array references
            r#"my $aref = \@{$array_ref};"#,
            r#"my $aref = \@{"array"};"#,
            // Array reference usage
            r#"push @$aref, 42;"#,
            r#"my @copy = @$aref;"#,
            r#"my $elem = $aref->[0];"#,
            r#"@{$aref} = (1, 2, 3);"#,
            // Anonymous arrays
            r#"my $aref = [1, 2, 3];"#,
            r#"my $aref = [];"#,
            r#"my $nested = [[1, 2], [3, 4]];"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_hash_references_comprehensive() {
        let test_cases = vec![
            // Basic hash references
            r#"my $href = \%hash;"#,
            r#"my $href = \%::global;"#,
            r#"my $href = \%Package::hash;"#,
            // Complex hash references
            r#"my $href = \%{$hash_ref};"#,
            r#"my $href = \%{"hash"};"#,
            // Hash reference usage
            r#"my %copy = %$href;"#,
            r#"my $val = $href->{key};"#,
            r#"%{$href} = (a => 1, b => 2);"#,
            r#"$href->{"key"} = "value";"#,
            // Anonymous hashes
            r#"my $href = {a => 1, b => 2};"#,
            r#"my $href = {};"#,
            r#"my $nested = {a => {b => 1}};"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_subroutine_references_comprehensive() {
        let test_cases = vec![
            // Basic subroutine references
            r#"my $sub_ref = \&sub;"#,
            r#"my $sub_ref = \&::global;"#,
            r#"my $sub_ref = \&Package::sub;"#,
            r#"my $sub_ref = \&Some::Module::function;"#,
            // Subroutine reference usage
            r#"&$sub_ref();"#,
            r#"$sub_ref->();"#,
            r#"$sub_ref->(1, 2, 3);"#,
            r#"my $result = $sub_ref->($arg);"#,
            // Anonymous subroutines
            r#"my $code = sub { print "hello"; };"#,
            r#"my $code = sub { return 42; };"#,
            r#"my $code = sub { my ($x) = @_; return $x * 2; };"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_glob_references_comprehensive() {
        let test_cases = vec![
            // Basic glob references
            r#"my $glob_ref = \*STDOUT;"#,
            r#"my $glob_ref = \*Package::handle;"#,
            r#"my $glob_ref = \*_;"#,
            // Glob reference usage
            r#"print $glob_ref "Hello";"#,
            r#"*{$glob_ref} = \$scalar;"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_string_interpolation_comprehensive() {
        let test_cases = vec![
            // Basic interpolation
            r#"my $str = "Hello $name";"#,
            r#"my $str = "Array: @array";"#,
            r#"my $str = "Hash: %hash";"#,
            // Complex scalar interpolation
            r#"my $str = "Value: ${var}";"#,
            r#"my $str = "Complex: ${$ref}";"#,
            r#"my $str = "Nested: ${hash{key}}";"#,
            // Complex array interpolation
            r#"my $str = "Array: @{[1, 2, 3]}";"#,
            r#"my $str = "Computed: @{[map { $_ * 2 } @array]}";"#,
            r#"my $str = "Array: @{$array_ref}";"#,
            // Mixed interpolation
            r#"my $str = "$scalar and @array and ${complex}";"#,
            r#"my $str = "Result: @{[$x + $y]}";"#,
            // Escape sequences with interpolation
            r#"my $str = "Line 1\n$var\nLine 3";"#,
            r#"my $str = "Tab:\t$var\tEnd";"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_regex_patterns_comprehensive() {
        let test_cases = vec![
            // Basic regex patterns
            r#"if ($str =~ /pattern/) { }"#,
            r#"if ($str =~ m/pattern/) { }"#,
            r#"$str =~ s/old/new/;"#,
            // Regex with special sequences
            r#"if ($str =~ /\w+/) { }"#,
            r#"if ($str =~ /\s*\d+\s*/) { }"#,
            r#"if ($str =~ /\W\S\D/) { }"#,
            // Quote-like regex
            r#"my $re = qr/\w+\s*/;"#,
            r#"my $re = qr/\d{2,4}/;"#,
            r#"my $re = qr/[a-z]+/i;"#,
            // Complex patterns
            r#"if ($str =~ /^start.*end$/) { }"#,
            r#"if ($str =~ /(?:group)/) { }"#,
            r#"if ($str =~ /(?<name>\w+)/) { }"#,
            // Different delimiters
            r#"if ($str =~ m{pattern}) { }"#,
            r#"if ($str =~ m!pattern!) { }"#,
            r#"$str =~ s{old}{new};"#,
            // Regex with flags
            r#"if ($str =~ /pattern/i) { }"#,
            r#"if ($str =~ /pattern/xms) { }"#,
            r#"$str =~ s/old/new/g;"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_operators_comprehensive() {
        let test_cases = vec![
            // Exponentiation operator
            r#"my $x = 2 ** 3;"#,
            r#"my $x = $base ** $exponent;"#,
            r#"$x **= 2;"#,
            // Bitwise operators
            r#"my $x = $a & $b;"#,
            r#"my $x = $a | $b;"#,
            r#"my $x = $a ^ $b;"#,
            r#"$x &= 0xFF;"#,
            r#"$x |= $flags;"#,
            r#"$x ^= $mask;"#,
            // Shift operators
            r#"my $x = $val << 2;"#,
            r#"my $x = $val >> 3;"#,
            r#"$x <<= 1;"#,
            r#"$x >>= 1;"#,
            // Logical operators
            r#"my $x = $a && $b;"#,
            r#"my $x = $a || $b;"#,
            r#"my $x = $a // $b;"#,
            r#"$x &&= $y;"#,
            r#"$x ||= $default;"#,
            r#"$x //= "default";"#,
            // String operators
            r#"my $str = $a . $b;"#,
            r#"$str .= " suffix";"#,
            r#"my $repeated = "x" x 10;"#,
            // Comparison operators
            r#"if ($a == $b) { }"#,
            r#"if ($a != $b) { }"#,
            r#"if ($a < $b) { }"#,
            r#"if ($a > $b) { }"#,
            r#"if ($a <= $b) { }"#,
            r#"if ($a >= $b) { }"#,
            r#"if ($a eq $b) { }"#,
            r#"if ($a ne $b) { }"#,
            r#"if ($a lt $b) { }"#,
            r#"if ($a gt $b) { }"#,
            r#"if ($a le $b) { }"#,
            r#"if ($a ge $b) { }"#,
            r#"if ($a =~ /pattern/) { }"#,
            r#"if ($a !~ /pattern/) { }"#,
            // Range operators
            r#"my @nums = (1..10);"#,
            r#"my @nums = (1...10);"#,
            // Ternary operator
            r#"my $val = $cond ? $true : $false;"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_heredoc_declarations() {
        let test_cases = vec![
            // Basic heredocs
            r#"print <<EOF;"#,
            r#"print <<'EOF';"#,
            r#"print <<"EOF";"#,
            // Indented heredocs
            r#"print <<~EOF;"#,
            r#"print <<~'EOF';"#,
            r#"print <<~"EOF";"#,
            // Multiple heredocs in single statement
            r#"print <<EOF1, <<EOF2;"#,
            // In assignments
            r#"my $text = <<EOF;"#,
            r#"my $text = <<'END';"#,
            // With expressions
            r#"my $result = $x + <<EOF;"#,
            r#"push @array, <<EOF;"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_special_blocks() {
        let test_cases = vec![
            // Special blocks
            r#"BEGIN { print "starting\n"; }"#,
            r#"END { print "ending\n"; }"#,
            r#"CHECK { validate(); }"#,
            r#"INIT { setup(); }"#,
            r#"UNITCHECK { check(); }"#,
            // Labeled blocks
            r#"LABEL: { last LABEL if $done; }"#,
            r#"OUTER: for (@list) { INNER: while (1) { last OUTER; } }"#,
            // Control flow with labels
            r#"next LABEL;"#,
            r#"last LABEL;"#,
            r#"redo LABEL;"#,
            r#"goto LABEL;"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_integration_comprehensive() {
        let test_cases = vec![
            // Complex real-world patterns
            r#"
            sub process_data {
                my ($self, $data_ref) = @_;
                my @results = map { $_ ** 2 } @{$data_ref};
                return \@results;
            }
            "#,
            r#"
            my $config = {
                name => "test",
                values => [1, 2, 3],
                handler => sub { print "Processing: $_[0]\n"; }
            };
            "#,
            r#"
            if ($text =~ m{^(\w+)\s*=\s*"([^"]+)"}) {
                my ($key, $val) = ($1, $2);
                $config->{$key} = $val;
            }
            "#,
            r#"
            my $str = "Values: @{[map { sprintf('%02d', $_) } @nums]}";
            print $str =~ s/\d+/$& * 2/ger;
            "#,
            // Error handling patterns
            r#"
            eval {
                my $result = $obj->method() // die "No result";
                return $result ** 2;
            };
            if ($@) {
                warn "Error: $@";
            }
            "#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_edge_cases() {
        let test_cases = vec![
            // Empty constructs
            r#"my $aref = [];"#,
            r#"my $href = {};"#,
            r#"my $code = sub { };"#,
            // Unicode in strings
            r#"my $str = "Hello 世界";"#,
            r#"my $str = "emoji: 🚀";"#,
            // Complex nesting
            r#"my $ref = \\\$scalar;"#,
            r#"my $data = {a => [1, {b => 2}]};"#,
            // Special variables with references
            r#"my $ref = \$_;"#,
            r#"my $ref = \@_;"#,
            r#"my $ref = \%ENV;"#,
        ];

        for case in test_cases {
            assert_parses(case);
        }
    }

    #[test]
    fn test_parse_failures() {
        let test_cases = vec![
            // Invalid references
            r#"my $ref = \"#,
            r#"my $ref = \;"#,
            // Incomplete interpolation
            r#"my $str = "Missing ${bracket";"#,
            r#"my $str = "Missing @{bracket";"#,
            // Invalid operators
            r#"my $x = 2 *** 3;"#,
            r#"my $x = $a &&& $b;"#,
        ];

        for case in test_cases {
            assert_parse_fails(case);
        }
    }
}
