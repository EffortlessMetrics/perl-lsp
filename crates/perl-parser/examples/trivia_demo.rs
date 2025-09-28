//! Demonstration of trivia (comment and whitespace) preservation
//!
//! This example shows how the parser preserves comments and formatting,
//! which is essential for code formatting and refactoring tools.

use perl_parser::trivia::Trivia;
use perl_parser::trivia_parser::{format_with_trivia, TriviaPreservingParser};

fn main() {
    println!("=== Trivia Preservation Demo ===\n");

    // Test cases showing various trivia types
    let test_cases = vec![
        (
            "Simple comment and whitespace",
            r#"# This is a file header
my $x = 42;  # inline comment
"#,
        ),
        (
            "Multiple comment types",
            r#"#!/usr/bin/perl
# File: example.pl
# Purpose: Demo trivia

use strict;  # Always use strict
use warnings;

=head1 NAME

Example - A demo script

=head1 DESCRIPTION

This demonstrates trivia preservation.

=cut

my $var = "hello";  # String variable
"#,
        ),
        (
            "Formatting preservation",
            r#"my $x    =    42;    # Extra spaces

    # Blank lines above
    
my   $y   =   99;  # Weird spacing
"#,
        ),
        (
            "POD in middle of code",
            r#"sub foo {
    my $x = shift;
    
=for comment
    This is a hidden comment in POD format
    It should be preserved but not executed
=cut
    
    return $x * 2;
}
"#,
        ),
    ];

    for (description, source) in test_cases {
        println!("Test: {}", description);
        println!("Original source:");
        println!("{}", source);
        println!("---");

        let parser = TriviaPreservingParser::new(source.to_string());
        let result = parser.parse();

        // Show collected trivia
        println!("Leading trivia collected:");
        for (i, trivia_token) in result.leading_trivia.iter().enumerate() {
            let kind = trivia_token.trivia.kind_name();
            let content = match &trivia_token.trivia {
                Trivia::Whitespace(s) => format!("{:?}", s),
                Trivia::LineComment(s) => s.clone(),
                Trivia::PodComment(s) => {
                    let first_line = s.lines().next().unwrap_or("");
                    format!("{}...", first_line)
                }
                Trivia::Newline => "\\n".to_string(),
            };
            println!("  {}. {} - {}", i + 1, kind, content);
        }

        // Show AST structure
        println!("\nAST structure:");
        println!("  {:?}", result.node.kind);

        println!("\n---\n");
    }

    // Demonstrate round-trip formatting
    println!("=== Round-trip Formatting Demo ===\n");

    let format_test = r#"# Header comment
    my $x = 42;  # Set x
    
    # Another statement
    our $y = 99;
"#;

    println!("Original:");
    println!("{}", format_test);

    let parser = TriviaPreservingParser::new(format_test.to_string());
    let ast_with_trivia = parser.parse();

    // In a real implementation, this would perfectly reconstruct the source
    println!("After parsing (simplified):");
    println!("{}", format_with_trivia(&ast_with_trivia));

    println!("\n=== Trivia Statistics ===\n");

    // Analyze a larger example
    let large_example = r#"#!/usr/bin/perl
#
# Copyright (c) 2024 Example Corp
# Licensed under MIT License
#

use strict;
use warnings;
use feature 'say';

=head1 NAME

LargeExample - A comprehensive Perl script

=head1 SYNOPSIS

    perl large_example.pl [options]

=head1 DESCRIPTION

This script demonstrates comprehensive trivia handling including:

=over 4

=item * Line comments with # 

=item * POD documentation

=item * Various whitespace patterns

=back

=cut

# Global variables
our $VERSION = '1.0';  # Version number
our $DEBUG   = 0;      # Debug flag

# Main subroutine
sub main {
    # Process command line arguments
    
    say "Starting program...";  # Use modern 'say'
    
    # TODO: Add more functionality
    # FIXME: Handle edge cases
    # NOTE: This is just a demo
}

# Run the main subroutine
main() unless caller;

=head1 AUTHOR

Example Author <author@example.com>

=cut

# End of file
"#;

    let parser = TriviaPreservingParser::new(large_example.to_string());
    let result = parser.parse();

    // Count trivia types
    let mut comment_count = 0;
    let mut whitespace_count = 0;
    let mut newline_count = 0;
    let mut pod_count = 0;

    for trivia_token in &result.leading_trivia {
        match &trivia_token.trivia {
            Trivia::LineComment(_) => comment_count += 1,
            Trivia::Whitespace(_) => whitespace_count += 1,
            Trivia::Newline => newline_count += 1,
            Trivia::PodComment(_) => pod_count += 1,
        }
    }

    println!("Trivia statistics for large example:");
    println!("  Line comments: {}", comment_count);
    println!("  Whitespace nodes: {}", whitespace_count);
    println!("  Newlines: {}", newline_count);
    println!("  POD sections: {}", pod_count);
    println!("  Total trivia nodes: {}", result.leading_trivia.len());

    println!("\n=== Benefits of Trivia Preservation ===\n");
    println!("1. **Code Formatting**: Preserve original formatting when refactoring");
    println!("2. **Documentation**: Keep comments and POD with associated code");
    println!("3. **Round-trip Editing**: Parse → Modify → Reformat without losing style");
    println!("4. **IDE Features**: Show comments in hover, preserve in quick fixes");
    println!("5. **Style Enforcement**: Detect and fix formatting issues");
}
