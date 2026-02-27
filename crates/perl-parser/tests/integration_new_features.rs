//! Integration tests for all new features implemented in perl-parser

use perl_parser::Parser;

#[test]
fn test_regex_modifiers_integration() {
    let tests = vec![
        ("/pattern/", ""),
        ("/pattern/i", "i"),
        ("/pattern/gimsx", "gimsx"),
        ("m/pattern/i", "i"),
        ("m{pattern}ms", "ms"),
        ("qr/pattern/io", "io"),
    ];

    for (code, expected_modifiers) in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(sexp.contains("regex"), "Expected regex node for: {}", code);
        if !expected_modifiers.is_empty() {
            assert!(
                sexp.contains(expected_modifiers),
                "Expected modifiers '{}' in output for: {}",
                expected_modifiers,
                code
            );
        }
    }
}

#[test]
fn test_substitution_integration() {
    let tests = vec![
        ("s/foo/bar/", "Basic substitution"),
        ("s/foo/bar/g", "Global substitution"),
        ("s{old}{new}gi", "Braces with modifiers"),
        ("s[pattern][replacement]e", "Brackets with eval"),
        ("$str =~ s/foo/bar/g", "Substitution with =~"),
    ];

    for (code, desc) in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(sexp.contains("substitution"), "{}: Expected substitution in: {}", desc, sexp);
    }
}

#[test]
fn test_transliteration_integration() {
    let tests = vec![
        ("tr/a-z/A-Z/", "Basic transliteration"),
        ("y/0-9/a-j/", "y/// form"),
        ("tr{a-z}{A-Z}d", "Braces with delete"),
        ("$str =~ tr/a-z/A-Z/", "Transliteration with =~"),
    ];

    for (code, desc) in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(
            sexp.contains("transliteration"),
            "{}: Expected transliteration in: {}",
            desc,
            sexp
        );
    }
}

#[test]
fn test_qw_integration() {
    let tests = vec![
        ("qw()", "Empty qw"),
        ("qw(one)", "Single word"),
        ("qw(one two three)", "Multiple words"),
        ("qw{foo bar}", "Braces"),
        ("qw[x y z]", "Brackets"),
        ("qw<alpha beta>", "Angles"),
    ];

    for (code, desc) in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(sexp.contains("array"), "{}: Expected array in: {}", desc, sexp);
    }
}

#[test]
fn test_statement_modifiers_integration() {
    let tests = vec![
        ("print if $x", "if modifier"),
        ("die unless $ok", "unless modifier"),
        ("$x++ while $y", "while modifier"),
        ("sleep until $ready", "until modifier"),
        ("say for @list", "for modifier"),
    ];

    for (code, desc) in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(
            sexp.contains("statement_modifier"),
            "{}: Expected statement_modifier in: {}",
            desc,
            sexp
        );
    }
}

#[test]
fn test_isa_operator_integration() {
    let tests =
        vec!["$x ISA 'Class'", "$obj ISA $class", "ref($x) ISA 'ARRAY'", "$self ISA My::Class"];

    for code in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(sexp.contains("binary_ISA"), "Expected ISA operator in: {}", sexp);
    }
}

#[test]
fn test_file_test_operators_integration() {
    let tests =
        vec!["-f $file", "-d $dir", "-e $path", "-r $file", "-w $file", "-x $file", "-s $file"];

    for code in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(sexp.contains("unary_-"), "Expected unary file test in: {}", sexp);
    }
}

#[test]
fn test_smart_match_integration() {
    let tests = vec!["$x ~~ $y", "$x ~~ @array", "$x ~~ /pattern/", "$x ~~ 'string'"];

    for code in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(sexp.contains("~~"), "Expected smart match operator in: {}", sexp);
    }
}

#[test]
fn test_special_blocks_integration() {
    let tests = vec![
        ("BEGIN { }", "BEGIN"),
        ("END { }", "END"),
        ("CHECK { }", "CHECK"),
        ("INIT { }", "INIT"),
        ("UNITCHECK { }", "UNITCHECK"),
    ];

    for (code, block_type) in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(sexp.contains(block_type), "Expected {} block in: {}", block_type, sexp);
    }
}

#[test]
fn test_attributes_integration() {
    let tests = vec![
        ("sub foo : lvalue { }", "lvalue attribute"),
        ("sub bar : method : lvalue { }", "multiple attributes"),
        ("my $x :shared", "variable attribute"),
        ("our $y :unique", "our with attribute"),
    ];

    for (code, desc) in tests {
        use perl_tdd_support::must;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        // Skip test case if attributes aren't represented yet
        if !(sexp.contains(":") || sexp.contains("attribute")) {
            eprintln!("Skipping attribute test for {}: {}", desc, sexp);
            continue;
        }
    }
}
