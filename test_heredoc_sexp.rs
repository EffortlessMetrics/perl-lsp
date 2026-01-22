use perl_parser::Parser;

fn main() {
    let inputs = vec![
        ("bare", r#"my $x = <<EOF;
content here
EOF
"#),
        ("double-quoted", r#"my $x = <<"EOF";
content $var here
EOF
"#),
        ("single-quoted", r#"my $x = <<'EOF';
content $var here
EOF
"#),
        ("backtick", r#"my $x = <<`EOF`;
echo "command"
EOF
"#),
        ("indented", r#"my $x = <<~EOF;
    indented content
  EOF
"#),
    ];

    for (name, input) in inputs {
        println!("\n=== {} ===", name);
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(ast) => {
                println!("S-expression:\n{}", ast.to_sexp());
            }
            Err(e) => {
                println!("Parse error: {}", e);
            }
        }
    }
}
