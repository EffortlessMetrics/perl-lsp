//! Proof of concept for error recovery in the Perl parser
//!
//! This example demonstrates how error recovery could work in practice.

use perl_parser::{Node, NodeKind, ParseError, Parser, SourceLocation};

/// Result of parsing with error recovery
#[derive(Debug)]
pub struct RecoveryParseResult {
    pub ast: Option<Node>,
    pub errors: Vec<ErrorInfo>,
}

#[derive(Debug, Clone)]
pub struct ErrorInfo {
    pub error: ParseError,
    pub line: usize,
    pub column: usize,
    pub context: String,
}

/// Extended parser with error recovery capabilities
pub struct RecoveryParser<'a> {
    inner: Parser<'a>,
    errors: Vec<ParseError>,
    source: &'a str,
}

impl<'a> RecoveryParser<'a> {
    pub fn new(source: &'a str) -> Self {
        RecoveryParser { inner: Parser::new(source), errors: Vec::new(), source }
    }

    pub fn parse_with_recovery(&mut self) -> RecoveryParseResult {
        // Try to parse normally first
        match self.inner.parse() {
            Ok(ast) => RecoveryParseResult { ast: Some(ast), errors: vec![] },
            Err(e) => {
                // Initial parse failed - try recovery strategies
                self.errors.push(e.clone());

                // For this POC, we'll try some simple recovery strategies
                let recovered_ast = self.attempt_recovery();

                RecoveryParseResult { ast: recovered_ast, errors: self.create_error_infos() }
            }
        }
    }

    fn attempt_recovery(&mut self) -> Option<Node> {
        // Strategy 1: Try to parse as a series of statements, skipping errors
        let mut statements = Vec::new();
        let _parser = Parser::new(self.source);

        // Split by lines and try to parse each line
        for (line_no, line) in self.source.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            // Try to parse common statement patterns
            if let Some(stmt) = self.try_parse_line(line, line_no) {
                statements.push(stmt);
            }
        }

        if statements.is_empty() {
            None
        } else {
            Some(Node::new(
                NodeKind::Program { statements },
                SourceLocation { start: 0, end: self.source.len() },
            ))
        }
    }

    fn try_parse_line(&mut self, line: &str, _line_no: usize) -> Option<Node> {
        // Try to identify and parse common patterns
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.starts_with('#') || trimmed.is_empty() {
            return None;
        }

        // Try to parse as a complete statement
        let mut line_parser = Parser::new(line);
        if let Ok(node) = line_parser.parse() {
            if let NodeKind::Program { mut statements } = node.kind {
                return statements.pop();
            }
        }

        // Try adding a semicolon if missing
        if !trimmed.ends_with(';') && !trimmed.ends_with('{') && !trimmed.ends_with('}') {
            let with_semi = format!("{};", trimmed);
            let mut semi_parser = Parser::new(&with_semi);
            if let Ok(node) = semi_parser.parse() {
                if let NodeKind::Program { mut statements } = node.kind {
                    self.errors.push(ParseError::syntax("Missing semicolon", line.len()));
                    return statements.pop();
                }
            }
        }

        // Create an error node
        Some(Node::new(
            NodeKind::String { value: format!("ERROR: {}", line), interpolated: false },
            SourceLocation { start: 0, end: line.len() },
        ))
    }

    fn create_error_infos(&self) -> Vec<ErrorInfo> {
        self.errors
            .iter()
            .map(|error| {
                let (line, column) = self.position_to_line_col(0); // Simplified for POC
                ErrorInfo {
                    error: error.clone(),
                    line,
                    column,
                    context: self.get_error_context(line),
                }
            })
            .collect()
    }

    fn position_to_line_col(&self, _pos: usize) -> (usize, usize) {
        // Simplified for POC
        (1, 1)
    }

    fn get_error_context(&self, line: usize) -> String {
        self.source.lines().nth(line.saturating_sub(1)).unwrap_or("").to_string()
    }
}

/// Demonstrate error recovery with various examples
fn main() {
    println!("=== Perl Parser Error Recovery POC ===\n");

    let examples = vec![
        // Missing semicolon
        ("Missing semicolon", "my $x = 42\nprint $x"),
        // Unclosed string
        ("Unclosed string", r#"print "hello"#),
        // Invalid syntax
        ("Invalid syntax", "if ($x) print 'yes'"),
        // Multiple errors
        ("Multiple errors", "my $x = \nif { print }"),
        // Partial valid code
        ("Partial valid", "my $x = 42;\nthis is not valid\nmy $y = 43;"),
    ];

    for (name, code) in examples {
        println!("Test: {}", name);
        println!("Code:\n{}", code);
        println!("---");

        let mut parser = RecoveryParser::new(code);
        let result = parser.parse_with_recovery();

        if let Some(ast) = result.ast {
            println!("Recovered AST:");
            print_ast(&ast, 0);
        } else {
            println!("No AST could be recovered");
        }

        if !result.errors.is_empty() {
            println!("\nErrors:");
            for error in &result.errors {
                println!("  - {:?}", error.error);
            }
        }

        println!("\n");
    }

    // Demonstrate a more complex example
    let complex_code = r#"
# Perl script with multiple errors
use strict;
use warnings;

my $name = "World"  # Missing semicolon
print "Hello, $name!\n";

sub greet {
    my ($who) = @_;
    print "Greetings, $who\n"  # Missing semicolon
}

# Missing closing brace for if
if ($name eq "World") {
    greet($name);

my @numbers = (1, 2, 3);
foreach my $num (@numbers) {
    print "$num\n";
}
"#;

    println!("=== Complex Example ===");
    println!("This demonstrates recovery from multiple errors in a larger script\n");

    let mut parser = RecoveryParser::new(complex_code);
    let result = parser.parse_with_recovery();

    if let Some(ast) = result.ast {
        println!("Successfully recovered partial AST with {} nodes", count_nodes(&ast));
        println!("\nRecovered structure:");
        print_ast(&ast, 0);
    }

    println!("\nFound {} errors during parsing", result.errors.len());
}

fn print_ast(node: &Node, indent: usize) {
    let prefix = "  ".repeat(indent);
    match &node.kind {
        NodeKind::Program { statements } => {
            println!("{}Program", prefix);
            for stmt in statements {
                print_ast(stmt, indent + 1);
            }
        }
        NodeKind::String { value, .. } => {
            if value.starts_with("ERROR:") {
                println!("{}Error: {}", prefix, value);
            } else {
                println!("{}String: {}", prefix, value);
            }
        }
        _ => {
            // Simplified printing for other nodes
            println!("{}{:?}", prefix, node.kind);
        }
    }
}

fn count_nodes(node: &Node) -> usize {
    match &node.kind {
        NodeKind::Program { statements } => 1 + statements.iter().map(count_nodes).sum::<usize>(),
        _ => 1,
    }
}
