use perl_parser::Parser;

#[test]
fn test_regex_modifiers() {
    let code = r#"
    /hello/i
    /world/gimsx
    "#;
    
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let sexp = ast.to_sexp();
    
    println!("Regex modifiers test:");
    println!("{}", sexp);
    
    assert!(sexp.contains("(regex /hello/ i)"));
    assert!(sexp.contains("(regex /world/ gimsx)"));
}

#[test]
fn test_substitution() {
    let code = r#"
    $str =~ s/foo/bar/g
    $str =~ s{old}{new}gi
    "#;
    
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let sexp = ast.to_sexp();
    
    println!("Substitution test:");
    println!("{}", sexp);
    
    assert!(sexp.contains("substitution"));
    assert!(sexp.contains("foo"));
    assert!(sexp.contains("bar"));
}

#[test]
fn test_transliteration() {
    let code = r#"
    $str =~ tr/a-z/A-Z/
    $str =~ y/0-9/a-j/c
    "#;
    
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let sexp = ast.to_sexp();
    
    println!("Transliteration test:");
    println!("{}", sexp);
    
    assert!(sexp.contains("transliteration"));
}

#[test]
fn test_qw_construct() {
    let code = r#"
    my @words = qw(one two three)
    my @braces = qw{foo bar baz}
    "#;
    
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let sexp = ast.to_sexp();
    
    println!("qw() construct test:");
    println!("{}", sexp);
    
    assert!(sexp.contains("array"));
    assert!(sexp.contains("'one'"));
    assert!(sexp.contains("'two'"));
    assert!(sexp.contains("'three'"));
}

#[test]
fn test_heredoc() {
    let code = r#"
my $heredoc = <<'END';
This is content
END
"#;
    
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let sexp = ast.to_sexp();
    
    println!("Heredoc test:");
    println!("{}", sexp);
    
    assert!(sexp.contains("heredoc"));
}

#[test]
fn test_all_improvements() {
    let code = r#"
# Regex with modifiers
if ($text =~ /hello/i) { }

# Substitution
$str =~ s/foo/bar/g;

# Transliteration  
$upper =~ tr/a-z/A-Z/;

# qw construct
my @words = qw(one two three);

# Quote regex with modifiers
my $regex = qr/pattern/ms;
"#;
    
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let sexp = ast.to_sexp();
    
    println!("Complete test output:");
    println!("{}", sexp);
}