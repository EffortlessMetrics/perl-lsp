//! Demonstration of import optimization code actions
//!
//! This example shows how the import optimizer integrates with LSP code actions
//! to provide comprehensive import management.

use perl_lsp_code_actions::{CodeActionKind, CodeActionsProvider};
use perl_parser_core::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"#!/usr/bin/perl
use warnings;
use strict;
use List::Util qw(max min sum);
use Data::Dumper qw(Dumper);
use List::Util qw(first);
use JSON qw(encode_json decode_json to_json);

# Only use some imports - others are unused
my @numbers = (1, 2, 3, 4, 5);
my $max = max(@numbers);
print Dumper(\@numbers);
my $json = encode_json({ max => $max });
"#;

    println!("Original Perl code:");
    println!("{}", source);
    println!("\n{}\n", "=".repeat(70));

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    println!("Available code actions:");
    for (i, action) in actions.iter().enumerate() {
        println!("  {}. {} ({:?})", i + 1, action.title, action.kind);
    }

    // Find the organize imports action
    if let Some(organize_action) =
        actions.iter().find(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports))
    {
        println!("\n{}\n", "=".repeat(70));
        println!("Organize Imports Action:");
        println!("  Title: {}", organize_action.title);
        println!("  Edits: {} change(s)", organize_action.edit.changes.len());

        for (i, edit) in organize_action.edit.changes.iter().enumerate() {
            println!("\n  Edit #{}:", i + 1);
            println!("    Range: {:?}", edit.location);
            println!("    New text:");
            for line in edit.new_text.lines() {
                println!("      {}", line);
            }
        }
    } else {
        println!("\nNo 'Organize Imports' action available");
    }

    // Demonstrate missing imports detection
    let source_with_missing = r#"use strict;
use warnings;

my $result = JSON::encode_json({key => 'value'});
my $path = File::Spec::catfile('/tmp', 'test.txt');
"#;

    println!("\n{}\n", "=".repeat(70));
    println!("Code with missing imports:");
    println!("{}", source_with_missing);

    let mut parser2 = Parser::new(source_with_missing);
    let ast2 = parser2.parse()?;
    let provider2 = CodeActionsProvider::new(source_with_missing.to_string());
    let actions2 = provider2.get_code_actions(&ast2, (0, source_with_missing.len()), &[]);

    println!("\nDetected actions:");
    for (i, action) in actions2.iter().enumerate() {
        if action.title.contains("import") {
            println!("  {}. {}", i + 1, action.title);
        }
    }

    Ok(())
}
