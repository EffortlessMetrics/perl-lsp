//! Test heredoc support
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic heredoc
        r#"print <<EOF;
Hello World
EOF
"#,
        // Quoted heredoc (no interpolation)
        r#"print <<'END';
$var is literal
END
"#,
        // Double-quoted heredoc (with interpolation)
        r#"print <<"TEXT";
Hello $name
TEXT
"#,
        // Multiple heredocs
        r#"print <<FOO, <<BAR;
First document
FOO
Second document
BAR
"#,
        // Indented heredoc (Perl 5.26+)
        r#"print <<~"EOF";
    This is indented
    Another line
    EOF
"#,
        // Heredoc in assignment
        r#"my $text = <<'END';
Multi-line
text here
END
"#,
        // Heredoc as function argument
        r#"process(<<EOF, $other_arg);
Some data
More data
EOF
"#,
        // Empty heredoc
        r#"print <<EMPTY;
EMPTY
"#,
    ];

    for (i, test) in tests.iter().enumerate() {
        println!("\nTest {}: {}", i + 1, test.lines().next().unwrap_or(""));
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
