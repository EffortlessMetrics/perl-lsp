use perl_parser::Parser;

#[test]
fn test_isa_operator() {
    let test_cases = vec![
        ("$obj ISA 'MyClass'", "ISA with string literal"),
        ("$x ISA $class", "ISA with variable"),
        ("$self ISA MyClass::SubClass", "ISA with qualified name"),
        ("ref($x) ISA 'ARRAY'", "ISA with function call"),
    ];

    for (code, desc) in test_cases {
        println!("Testing {}: {}", desc, code);
        let mut parser = Parser::new(code);
        let result = parser.parse();

        assert!(result.is_ok(), "Failed to parse '{}': {:?}", code, result);
        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        println!("  Result: {}", sexp);
        assert!(sexp.contains("ISA"), "ISA operator not found in output for '{}'", code);
    }
}

#[test]
fn test_all_improvements() {
    // Comprehensive test of all the features we've implemented
    let code = r#"
# Regex with modifiers
if ($text =~ /hello/i) { }
my $compiled = qr/pattern/ms;

# Substitution with replacement
$str =~ s/foo/bar/g;
$str =~ s{old}{new}gi;

# Transliteration
$upper =~ tr/a-z/A-Z/;
$upper =~ y/0-9/a-j/c;

# qw() constructs
my @words = qw(one two three);
my @braces = qw{foo bar baz};
my @brackets = qw[x y z];

# Heredoc with content
# (Heredoc content collection requires lexer support for HeredocBody tokens)

# Statement modifiers
print $x if $y;
die unless $ok;
$x++ while $x < 10;

# ISA operator
$obj ISA 'MyClass';
ref($x) ISA 'ARRAY';

# File test operators
if (-f $file && -r $file) { }

# Smart match
$x ~~ $y;
$x ~~ /pattern/;

# Special blocks
BEGIN { }
END { }

# Attributes
sub foo : lvalue { }
my $x :shared;
"#;

    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Failed to parse comprehensive test: {:?}", result);
    let ast = result.unwrap();
    let sexp = ast.to_sexp();

    // Verify key features are present
    assert!(sexp.contains("regex"), "Regex not found");
    assert!(sexp.contains("substitution"), "Substitution not found");
    assert!(sexp.contains("transliteration"), "Transliteration not found");
    assert!(sexp.contains("array"), "qw() array not found");
    assert!(sexp.contains("statement_modifier"), "Statement modifiers not found");
    assert!(sexp.contains("ISA"), "ISA operator not found");
    assert!(sexp.contains("unary_-f"), "File test operator not found");
    assert!(sexp.contains("~~"), "Smart match not found");
    assert!(sexp.contains("BEGIN"), "BEGIN block not found");
    assert!(sexp.contains("lvalue"), "Attributes not found");

    println!("All features successfully parsed!");
}
