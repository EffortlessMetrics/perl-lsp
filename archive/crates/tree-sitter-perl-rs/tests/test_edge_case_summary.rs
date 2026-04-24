//! Summary of edge case coverage
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn edge_case_coverage_summary() {
    println!("\n=== EDGE CASE COVERAGE SUMMARY ===\n");

    // Format strings
    let format_test = r#"format STDOUT = 
@<<<< @>>>> 
$a, $b
."#;
    test_code("Format strings", format_test);

    // V-strings
    let vstring_test = "my $v = v1.2.3;";
    test_code("V-strings", vstring_test);

    // Encoding pragmas
    let encoding_test = "use utf8; use encoding 'latin1';";
    test_code("Encoding pragmas", encoding_test);

    // Typeglobs
    let typeglob_test = "*foo = *bar; *foo{SCALAR} = \\$x;";
    test_code("Typeglobs", typeglob_test);

    // Indirect object syntax
    let indirect_test = "new Module @args; print $fh $data;";
    test_code("Indirect object syntax", indirect_test);

    // Lvalue subs
    let lvalue_test = "sub temp :lvalue { $x }";
    test_code("Lvalue subroutines", lvalue_test);

    // Hash/array slices
    let slice_test = "@hash{@keys} = @values;";
    test_code("Hash/array slices", slice_test);

    // Regex code assertions
    let regex_test = r#"$x =~ /pattern(?{ $code })/;"#;
    test_code("Regex code assertions", regex_test);

    // __DATA__ section
    let data_test = "__DATA__\ndata here";
    test_code("__DATA__ section", data_test);

    // Source filters (would fail)
    let filter_test = "use Filter::Simple;";
    test_code("Source filters", filter_test);

    // Operator overloading
    let overload_test = r#"use overload '+' => \&add;"#;
    test_code("Operator overloading", overload_test);

    // File test stacking
    let filetest_test = "-f -w -x $file";
    test_code("Stacked file tests", filetest_test);

    // Special filehandle _
    let underscore_test = "-f $file && -w _";
    test_code("Underscore filehandle", underscore_test);

    // Complex symbolic refs
    let symref_test = r#"$${"var_" . $n} = 42;"#;
    test_code("Symbolic references", symref_test);

    // Multi-char quote delimiters
    let quote_test = "q###text###;";
    test_code("Multi-char delimiters", quote_test);
}

fn test_code(name: &str, code: &str) {
    let mut lexer = PerlLexer::new(code);
    let mut tokens = Vec::new();
    let mut errors = 0;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::Error(_)) {
            errors += 1;
        }
        tokens.push(token);
    }

    let status = if errors == 0 { "✓ SUPPORTED" } else { "✗ PARTIAL/UNSUPPORTED" };
    println!("{:<25} {} ({} tokens, {} errors)", name, status, tokens.len(), errors);
}
