//! Error Handling Example
//!
//! This example demonstrates how to handle parse errors gracefully.

use perl_parser::{ParseError, Parser};

fn main() {
    // Examples of code with various syntax errors
    let test_cases = vec![
        ("Valid", "my $x = 42; print $x;"),
        ("Missing semicolon", "my $x = 42\nprint $x;"),
        ("Unclosed string", r#"my $x = "hello"#),
        ("Invalid syntax", "my $ = 42;"),
        ("Unmatched bracket", "my @arr = [1, 2, 3;"),
        ("Missing closing brace", "if ($x) { print 'yes';"),
    ];

    for (name, code) in test_cases {
        println!("\n📝 Test: {}", name);
        println!("Code: {}", code);
        println!("{}", "─".repeat(50));

        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Parse successful!");
                println!("AST: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Parse error!");
                print_error(code, &e);
            }
        }
    }

    // Example of handling errors programmatically
    println!("\n\n🔧 Programmatic Error Handling Example");
    println!("{}", "─".repeat(50));

    let code = r#"
sub calculate {
    my ($a, $b) = @_;
    return $a + $b  # Missing semicolon
}

my $result = calculate(10, 20)
print "Result: $result\n";  # This line won't parse due to previous error
"#;

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(_) => {
            println!("No errors found");
        }
        Err(e) => {
            println!("Found parse error:");

            // Extract error information
            match e {
                ParseError::UnexpectedToken {
                    expected,
                    found,
                    location,
                } => {
                    println!("  Type: Unexpected token");
                    println!("  Expected: {}", expected);
                    println!("  Found: {}", found);
                    println!("  Location: {}", location);

                    // Show context
                    show_error_context(code, location);
                }
                ParseError::UnexpectedEof => {
                    println!("  Type: Unexpected end of file");
                    println!("  Error: {}", e);
                }
                ParseError::SyntaxError { message, location } => {
                    println!("  Type: Syntax error");
                    println!("  Message: {}", message);
                    println!("  Location: {}", location);

                    show_error_context(code, location);
                }
                _ => {
                    println!("  Error: {}", e);
                }
            }
        }
    }
}

fn print_error(code: &str, error: &ParseError) {
    println!("Error: {}", error);

    // Try to extract position information
    let position = match error {
        ParseError::UnexpectedToken { location, .. } | ParseError::SyntaxError { location, .. } => {
            Some(*location)
        }
        _ => None,
    };

    if let Some(pos) = position {
        show_error_context(code, pos);
    }
}

fn show_error_context(code: &str, position: usize) {
    // Find line and column
    let mut line = 1;
    let mut col = 1;
    let mut _line_start = 0;

    for (i, ch) in code.chars().enumerate() {
        if i == position {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
            _line_start = i + 1;
        } else {
            col += 1;
        }
    }

    // Show the error line
    if let Some(line_text) = code.lines().nth(line - 1) {
        println!("\n  Line {}: {}", line, line_text);
        println!("  {}^", " ".repeat(col + 7));
        println!("  Error at line {}, column {}", line, col);
    }
}
