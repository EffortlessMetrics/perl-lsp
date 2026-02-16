//! Additional edge case tests for Perl constructs not covered elsewhere

#[cfg(test)]
mod tests {
    use perl_tdd_support::must;
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

    fn test_parse(code: &str) -> String {
        let mut parser = PureRustPerlParser::new();
        match parser.parse(code) {
            Ok(ast) => parser.to_sexp(&ast),
            Err(e) => must(Err::<String, _>(format!("Parse failed: {:?}\nCode: {}", e, code))),
        }
    }

    #[test]
    fn test_special_variables() {
        let cases = vec![
            // Match variables
            r#"print $`;"#, // Pre-match
            r#"print $&;"#, // Match
            r#"print $';"#, // Post-match
            r#"print $+;"#, // Last paren match
            // Process and system variables
            r#"print $$;"#, // Process ID
            r#"print $<;"#, // Real UID
            r#"print $>;"#, // Effective UID
            r#"print $(;"#, // Real GID
            r#"print $);"#, // Effective GID
            // Input/Output variables
            r#"$/ = "\n";"#, // Input record separator
            r#"$\ = "\n";"#, // Output record separator
            r#"$| = 1;"#,    // Autoflush
            r#"$, = " ";"#,  // Output field separator
            r#"$" = " ";"#,  // List separator
            // Format variables
            r#"$~ = "MYFORMAT";"#, // Format name
            r#"$^ = "TOP";"#,      // Top of form format
            r#"$= = 60;"#,         // Lines per page
            r#"$- = 10;"#,         // Lines remaining
            // Error variables
            r#"die $!;"#,             // System error
            r#"eval { }; print $@;"#, // Eval error
            r#"print $?;"#,           // Child error
            r#"print $^E;"#,          // Extended error
            // Perl internals
            r#"print $];"#,  // Perl version
            r#"print $^V;"#, // Perl version as v-string
            r#"print $^O;"#, // OS name
            r#"print $^T;"#, // Script start time
            r#"print $^X;"#, // Perl executable
            // Debugging
            r#"$^D = 1;"#,   // Debug flags
            r#"$^W = 1;"#,   // Warnings
            r#"print $^P;"#, // Internal debugging
            // Array/hash special vars
            r#"print $#array;"#,    // Last index
            r#"print $#{$ref};"#,   // Last index via ref
            r#"print $#{"name"};"#, // Last index symbolic
            // Special filehandles
            r#"print $.;"#, // Current line number
            r#"print $%;"#, // Current page number
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(
                result.contains("special_variable") || result.contains("scalar_variable"),
                "Failed to parse special variable: {}",
                case
            );
        }
    }

    #[test]
    fn test_bareword_filehandles() {
        let cases = vec![
            // Traditional bareword filehandles
            r#"open FH, '<', 'file.txt';"#,
            r#"open MYFILE, '>', 'output.txt';"#,
            r#"print FH "Hello";"#,
            r#"print STDERR "Error";"#,
            r#"close FH;"#,
            // With indirect object syntax
            r#"print FH @data;"#,
            r#"print STDOUT $_, "\n" for @items;"#,
            r#"printf STDERR "%s\n", $error;"#,
            // Select filehandle
            r#"select FH;"#,
            r#"select STDOUT;"#,
            r#"my $old_fh = select(FH);"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse bareword filehandle: {}", case);
        }
    }

    #[test]
    fn test_indirect_object_syntax() {
        let cases = vec![
            // Constructor calls
            r#"my $obj = new Class;"#,
            r#"my $obj = new Class();"#,
            r#"my $obj = new Class @args;"#,
            r#"my $obj = new Class::Name;"#,
            r#"my $obj = new $class;"#,
            // Method calls
            r#"method $obj;"#,
            r#"method $obj @args;"#,
            r#"method $obj 'arg1', 'arg2';"#,
            // Complex indirect
            r#"new Package::Class method $obj;"#,
            // With blocks
            r#"method { foo => 'bar' } $obj;"#,
        ];

        for case in cases {
            let result = test_parse(case);
            // These might parse as regular function calls, which is acceptable
            assert!(
                !result.contains("(ERROR)"),
                "Failed to parse indirect object syntax: {}",
                case
            );
        }
    }

    #[test]
    fn test_glob_and_readline() {
        let cases = vec![
            // Readline operator
            r#"my $line = <STDIN>;"#,
            r#"my $line = <FH>;"#,
            r#"while (<>) { print }"#,
            r#"while (<STDIN>) { chomp; print }"#,
            r#"my @lines = <FH>;"#,
            // Glob operator
            r#"my @files = <*.txt>;"#,
            r#"my @files = <dir/*.pl>;"#,
            r#"my @files = <{foo,bar}.txt>;"#,
            r#"for my $file (<*.pm>) { }"#,
            // Diamond operator
            r#"while (<>) { print }"#,
            r#"my $line = <>;"#,
            r#"@ARGV = ('file1', 'file2'); while (<>) { }"#,
            // Glob function
            r#"my @files = glob("*.txt");"#,
            r#"my @files = glob("dir/*.{pl,pm}");"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse glob/readline: {}", case);
        }
    }

    #[test]
    fn test_flipflop_operator() {
        let cases = vec![
            // Range flip-flop
            r#"if (1..10) { print }"#,
            r#"if ($. == 1 .. $. == 10) { print }"#,
            r#"print if /start/ .. /end/;"#,
            r#"next unless $flag .. $other_flag;"#,
            // Three-dot flip-flop
            r#"if (1...10) { print }"#,
            r#"print if /BEGIN/ ... /END/;"#,
            // In scalar context (flip-flop)
            r#"my $in_section = /^=head1/ .. /^=cut/;"#,
            r#"$inside = $. == 10 .. $. == 20;"#,
            // In list context (range)
            r#"my @nums = (1..10);"#,
            r#"for (1..100) { }"#,
            r#"my @chars = ('a'..'z');"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(
                result.contains("range_expression") || !result.contains("(ERROR)"),
                "Failed to parse flip-flop operator: {}",
                case
            );
        }
    }

    #[test]
    fn test_vstrings() {
        let cases = vec![
            // Version strings
            r#"use v5.10;"#,
            r#"use v5.10.1;"#,
            r#"require v5.8.0;"#,
            // v-strings in general
            r#"my $version = v1.2.3;"#,
            r#"my $ip = v127.0.0.1;"#,
            r#"if ($] >= v5.10) { }"#,
            // Comparison
            r#"if ($^V ge v5.10.0) { }"#,
            r#"die "Need Perl 5.10" if $^V lt v5.10;"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse v-string: {}", case);
        }
    }

    #[test]
    fn test_pack_unpack_templates() {
        let cases = vec![
            // Basic pack/unpack
            r#"my $packed = pack("C*", @bytes);"#,
            r#"my @bytes = unpack("C*", $data);"#,
            // Complex templates
            r#"pack("a10 x2 i", $str, $num);"#,
            r#"unpack("(a4)*", $data);"#,
            r#"pack("N/a*", $string);"#,
            // With length specifiers
            r#"pack("a20", $text);"#,
            r#"pack("H*", $hex);"#,
            r#"unpack("b*", $binary);"#,
            // Grouped templates
            r#"unpack("(a4 i)*", $data);"#,
            r#"pack("i @4 i", $x, $y);"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse pack/unpack: {}", case);
        }
    }

    #[test]
    fn test_special_literals() {
        let cases = vec![
            // Special literal constants
            r#"print __FILE__;"#,
            r#"print __LINE__;"#,
            r#"print __PACKAGE__;"#,
            r#"sub foo { print __SUB__ }"#,
            // In expressions
            r#"die "Error at " . __FILE__ . " line " . __LINE__;"#,
            r#"my $here = __PACKAGE__ . "::" . __SUB__;"#,
            // __DATA__ and __END__
            r#"
            print "before data";
            __DATA__
            This is data
            "#,
            r#"
            print "before end";
            __END__
            This is after end
            "#,
        ];

        for case in cases {
            let result = test_parse(case);
            // Check that special literals are recognized
            if !case.contains("__DATA__") && !case.contains("__END__") {
                assert!(
                    result.contains("special_literal") || !result.contains("(ERROR)"),
                    "Failed to parse special literal: {}",
                    case
                );
            }
        }
    }

    #[test]
    fn test_tied_variables() {
        let cases = vec![
            // Tie operations
            r#"tie $scalar, 'TiedScalar';"#,
            r#"tie @array, 'TiedArray', @args;"#,
            r#"tie %hash, 'TiedHash', $arg1, $arg2;"#,
            r#"tie *FH, 'TiedHandle', 'filename';"#,
            // Untie
            r#"untie $scalar;"#,
            r#"untie @array;"#,
            r#"untie %hash;"#,
            r#"untie *FH;"#,
            // Tied function
            r#"my $obj = tied($scalar);"#,
            r#"if (tied @array) { }"#,
            r#"tied(%hash)->method();"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(
                !result.contains("(ERROR)"),
                "Failed to parse tied variable operation: {}",
                case
            );
        }
    }

    #[test]
    fn test_format_output_variables() {
        let cases = vec![
            // Format-related variables
            r#"format MYFORMAT =
@<<<<<< @|||||| @>>>>>>
$name,  $score, $grade
.
"#,
            r#"$~ = 'MYFORMAT';"#, // Set format name
            r#"$^ = 'HEADER';"#,   // Set header format
            r#"$= = 60;"#,         // Lines per page
            r#"$- = 0;"#,          // Lines left on page
            r#"$% = 1;"#,          // Current page
            // Using formats
            r#"write STDOUT;"#,
            r#"write FH;"#,
            r#"write;"#,
        ];

        for case in cases {
            let result = test_parse(case);
            // Format declarations are complex, just ensure no ERROR
            assert!(!result.contains("(ERROR)"), "Failed to parse format-related code: {}", case);
        }
    }

    #[test]
    fn test_complex_dereferencing() {
        let cases = vec![
            // Multi-level dereferencing
            r#"$$ref;"#,
            r#"$$$ref_ref;"#,
            r#"$$$$ref_ref_ref;"#,
            // Mixed dereferencing
            r#"@{$ref}[0..5];"#,
            r#"@{$$ref};"#,
            r#"%{$ref}{key};"#,
            // Method calls on dereferenced objects
            r#"${$ref}->method();"#,
            r#"@{$ref->get_array_ref()};"#,
            // Postfix with complex base
            r#"$obj->method()->@*;"#,
            r#"$hash{key}->@[0..3];"#,
            r#"func()->%{qw(a b)};"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse complex dereferencing: {}", case);
        }
    }

    #[test]
    fn test_list_vs_scalar_context() {
        let cases = vec![
            // Explicit scalar context
            r#"my $count = @array;"#,
            r#"if (@array) { }"#,
            r#"my $last = $#array;"#,
            // List context
            r#"my @copy = @array;"#,
            r#"my ($first, @rest) = @array;"#,
            r#"push @target, @source;"#,
            // Scalar context operators
            r#"my $str = reverse @array;"#, // In scalar context, concatenates
            r#"my @rev = reverse @array;"#, // In list context, reverses
            // Context-sensitive functions
            r#"my $time = localtime;"#, // Scalar context
            r#"my @time = localtime;"#, // List context
            // Comma operator
            r#"my $x = (1, 2, 3);"#, // Scalar context, returns 3
            r#"my @x = (1, 2, 3);"#, // List context
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse list/scalar context: {}", case);
        }
    }

    #[test]
    fn test_obscure_quoting() {
        let cases = vec![
            // Quoting operators with unusual delimiters
            r#"q!Hello World!;"#,
            r#"qq#Interpolated $var#;"#,
            r#"qw@word list here@;"#,
            r#"qr|pattern|i;"#,
            r#"qx^command^;"#,
            // Nested delimiters
            r#"q{this {nested} works};"#,
            r#"qq[array[$i] = $val];"#,
            r#"qr(group (?:non-capturing) here);"#,
            // Whitespace before delimiter
            r#"q     {spaced};"#,
            r#"qq
            {multiline};"#,
            // Single character delimiters
            r#"q,comma delimited,;"#,
            r#"qq.period delimited.;"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse obscure quoting: {}", case);
        }
    }

    #[test]
    fn test_compound_statements() {
        let cases = vec![
            // do blocks with operators
            r#"my $x = do { local $/; <FH> };"#,
            r#"$val = do { $x + $y } or die;"#,
            r#"my $result = do FILE or die $!;"#,
            // eval varieties
            r#"eval { dangerous() } or warn $@;"#,
            r#"my $code = eval q{ sub { $_[0] + 1 } };"#,
            r#"eval "require $module" or die $@;"#,
            // Nested statement modifiers
            r#"print for @array if $condition;"#,
            r#"next if $x while <FH>;"#,
            // Complex conditionals
            r#"$x && $y && do { print "both" };"#,
            r#"$x || $y || die "neither";"#,
        ];

        for case in cases {
            let result = test_parse(case);
            assert!(!result.contains("(ERROR)"), "Failed to parse compound statement: {}", case);
        }
    }
}
