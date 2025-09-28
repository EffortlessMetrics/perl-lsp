//! Showcase of enhanced tree-sitter-perl features

use std::io::Cursor;
use tree_sitter_perl::{
    error_recovery::{ErrorRecoveryParser, RecoveryStrategy},
    sexp_formatter::SexpFormatter,
    streaming_parser::{ParseEvent, StreamConfig, StreamingParser},
    EnhancedFullParser,
};

fn main() {
    println!("=== Tree-sitter-perl Enhanced Features Showcase ===\n");

    // 1. Enhanced Parser with Heredocs and Special Sections
    showcase_enhanced_parser();

    // 2. Error Recovery
    showcase_error_recovery();

    // 3. Streaming Parser
    showcase_streaming_parser();

    // 4. S-expression Formatting
    showcase_sexp_formatting();
}

fn showcase_enhanced_parser() {
    println!("1. Enhanced Parser Demo");
    println!("-----------------------");

    let complex_perl = r#"#!/usr/bin/perl
use strict;
use warnings;

=head1 NAME

ComplexScript - Demonstrates enhanced parsing

=cut

# Various heredoc types
my $single = <<'EOF';
Single quoted heredoc
No interpolation: $var
EOF

my $double = <<"END";
Double quoted heredoc
With interpolation: $ENV{HOME}
END

my $backtick = <<`CMD`;
ls -la
CMD

my $indented = <<~'INDENT';
    This is indented
    content that preserves
    relative indentation
INDENT

# Complex data structure with heredoc
my %config = (
    name => "Test",
    description => <<'DESC',
This is a multi-line
description in a hash
DESC
    version => "1.0",
);

sub process {
    my $input = shift;
    return <<~"RESULT";
        Processing: $input
        Status: Complete
RESULT
}

print process("test data");

__DATA__
This is data section content
that can be read with <DATA>
Multiple lines supported
"#;

    let mut parser = EnhancedFullParser::new();
    match parser.parse(complex_perl) {
        Ok(ast) => {
            println!("✓ Successfully parsed complex Perl code");
            println!("  - Found {} POD sections", parser.pod_sections.len());
            if let Some(data_line) = parser.data_section_start {
                println!("  - DATA section starts at line {}", data_line);
            }
            println!("  - AST nodes: {}", count_nodes(&ast));
        }
        Err(e) => println!("✗ Parse failed: {:?}", e),
    }
    println!();
}

fn showcase_error_recovery() {
    println!("2. Error Recovery Demo");
    println!("----------------------");

    let malformed_perl = r#"
# Missing closing quote
my $str = "unclosed string
print "This should still parse";

# Invalid syntax
if ($x { 
    print "missing closing paren";
}

# Missing semicolon
my $y = 42
my $z = 43;

# Valid code after errors
sub valid_function {
    return "This function is valid";
}
"#;

    let mut parser = ErrorRecoveryParser::new()
        .with_strategies(vec![
            RecoveryStrategy::CreateErrorNode,
            RecoveryStrategy::SkipToStatementEnd,
            RecoveryStrategy::SkipLine,
        ])
        .with_max_attempts(10);

    match parser.parse(malformed_perl) {
        Ok(ast) => {
            println!("✓ Parsed with error recovery");
            println!("  - Recovered from {} errors", parser.errors().len());
            for (i, error) in parser.errors().iter().enumerate() {
                println!(
                    "  - Error {}: Line {}, Strategy: {:?}",
                    i + 1,
                    error.line,
                    error.recovery_used
                );
            }
            println!("  - Successfully parsed nodes: {}", count_nodes(&ast));
        }
        Err(_) => println!("✗ Failed to recover from errors"),
    }
    println!();
}

fn showcase_streaming_parser() {
    println!("3. Streaming Parser Demo");
    println!("------------------------");

    let large_perl = r#"
print "Starting file\n";

=head1 DOCUMENTATION

This is POD documentation
that the streaming parser extracts

=cut

sub function1 {
    my $x = shift;
    return $x * 2;
}

sub function2 {
    my ($a, $b) = @_;
    return $a + $b;
}

print "Middle of file\n";

my $data = <<'EOF';
Heredoc content
in streaming mode
EOF

print $data;

__DATA__
Data section content
Line 2 of data
Line 3 of data
"#;

    let cursor = Cursor::new(large_perl);
    let mut parser = StreamingParser::new(cursor, StreamConfig::default());

    println!("✓ Streaming parse events:");
    let events: Vec<_> = parser.parse().collect();

    let mut statement_count = 0;
    let mut special_sections = 0;
    let mut errors = 0;

    for event in &events {
        match event {
            ParseEvent::Node(_) => statement_count += 1,
            ParseEvent::SpecialSection { kind, start_line, .. } => {
                special_sections += 1;
                println!("  - Found {:?} section at line {}", kind, start_line);
            }
            ParseEvent::Error { .. } => errors += 1,
            _ => {}
        }
    }

    println!("  - Total statements parsed: {}", statement_count);
    println!("  - Special sections found: {}", special_sections);
    println!("  - Parse errors: {}", errors);
    println!();
}

fn showcase_sexp_formatting() {
    println!("4. S-expression Formatter Demo");
    println!("------------------------------");

    let simple_perl = r#"
sub greet {
    my $name = shift;
    print "Hello, $name!\n";
}

greet("World");
"#;

    let mut parser = EnhancedFullParser::new();
    match parser.parse(simple_perl) {
        Ok(ast) => {
            let formatter = SexpFormatter::new(simple_perl);

            println!("✓ Tree-sitter compatible S-expression:");
            let sexp = formatter.format(&ast);
            // Print first few lines of S-expression
            for line in sexp.lines().take(10) {
                println!("  {}", line);
            }
            if sexp.lines().count() > 10 {
                println!("  ...");
            }

            // Compact mode
            let compact_formatter = SexpFormatter::new(simple_perl).compact(true);
            let compact = compact_formatter.format(&ast);
            println!("\n✓ Compact S-expression (first 100 chars):");
            println!("  {}", &compact[..compact.len().min(100)]);
            if compact.len() > 100 {
                println!("  ...");
            }
        }
        Err(e) => println!("✗ Parse failed: {:?}", e),
    }
    println!();
}

fn count_nodes(ast: &tree_sitter_perl::AstNode) -> usize {
    use tree_sitter_perl::AstNode;

    match ast {
        AstNode::Program(nodes) | AstNode::Block(nodes) => {
            1 + nodes.iter().map(count_nodes).sum::<usize>()
        }
        AstNode::Statement(inner) => 1 + count_nodes(inner),
        AstNode::BinaryOp { left, right, .. } => 1 + count_nodes(left) + count_nodes(right),
        AstNode::UnaryOp { operand, .. } => 1 + count_nodes(operand),
        AstNode::Assignment { target, value, .. } => 1 + count_nodes(target) + count_nodes(value),
        AstNode::FunctionCall { function, args } => {
            1 + count_nodes(function) + args.iter().map(count_nodes).sum::<usize>()
        }
        AstNode::MethodCall { object, args, .. } => {
            1 + count_nodes(object) + args.iter().map(count_nodes).sum::<usize>()
        }
        AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
            let mut count = 1 + count_nodes(condition) + count_nodes(then_block);
            for (cond, block) in elsif_clauses {
                count += count_nodes(cond) + count_nodes(block);
            }
            if let Some(block) = else_block {
                count += count_nodes(block);
            }
            count
        }
        _ => 1, // Leaf nodes
    }
}
