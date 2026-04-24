//! Demonstration of advanced tree-sitter-perl features

use tree_sitter_perl::{
    incremental_parser::{Edit, IncrementalParser, Position},
    lsp_server::{
        PerlLanguageServer, Position as LspPosition, Range, TextDocumentContentChangeEvent,
    },
};

fn main() {
    println!("=== Advanced Tree-sitter-perl Features ===\n");

    // 1. Incremental Parsing
    demo_incremental_parsing();

    // 2. Language Server Protocol
    demo_lsp_features();
}

fn demo_incremental_parsing() {
    println!("1. Incremental Parsing Demo");
    println!("---------------------------");

    let mut parser = IncrementalParser::new();

    // Initial parse
    let initial_source = r#"
sub calculate {
    my ($a, $b) = @_;
    return $a + $b;
}

my $result = calculate(10, 20);
print "Result: $result\n";
"#;

    match parser.parse_initial(initial_source) {
        Ok(_tree) => {
            println!("✓ Initial parse successful");
            // Tree information would be displayed here
        }
        Err(e) => println!("✗ Initial parse failed: {:?}", e),
    }

    // Simulate an edit: change 10 to 15
    let start_byte = initial_source.find("10").unwrap_or(0);
    let edit = Edit {
        start_byte,
        old_end_byte: start_byte + 2,
        new_end_byte: start_byte + 2,
        start_position: Position { line: 6, column: 25 },
        old_end_position: Position { line: 6, column: 27 },
        new_end_position: Position { line: 6, column: 27 },
    };

    let edited_source = initial_source.replace("10", "15");

    match parser.apply_edit(edit.clone(), &edited_source) {
        Ok(_tree) => {
            println!("\n✓ Incremental update successful");
            println!("  - Edit applied at byte {}", edit.start_byte);
            println!("  - New tree created efficiently");
        }
        Err(e) => println!("✗ Incremental update failed: {:?}", e),
    }

    // Another edit: add a new line
    let insert_pos = edited_source.find("print").unwrap_or(0);
    let edit2 = Edit {
        start_byte: insert_pos,
        old_end_byte: insert_pos,
        new_end_byte: insert_pos + 27,
        start_position: Position { line: 7, column: 0 },
        old_end_position: Position { line: 7, column: 0 },
        new_end_position: Position { line: 7, column: 27 },
    };

    let new_line = "my $doubled = $result * 2;\n";
    let final_source =
        format!("{}{}{}", &edited_source[..insert_pos], new_line, &edited_source[insert_pos..]);

    match parser.apply_edit(edit2, &final_source) {
        Ok(_) => {
            println!("✓ Second incremental update successful");
            println!("  - Added new line efficiently");
            println!("  - Edit history: {} edits tracked", parser.edit_history().len());
        }
        Err(e) => println!("✗ Second update failed: {:?}", e),
    }

    println!();
}

fn demo_lsp_features() {
    println!("2. Language Server Protocol Demo");
    println!("--------------------------------");

    let lsp = PerlLanguageServer::new();

    // Open a document
    let uri = "file:///example.pl".to_string();
    let source = r#"#!/usr/bin/perl
use strict;
use warnings;

package Calculator;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub add {
    my ($self, $a, $b) = @_;
    return $a + $b;
}

sub multiply {
    my ($self, $a, $b) = @_;
    return $a * $b;
}

package main;

my $calc = Calculator->new();
my $sum = $calc->add(5, 3);
my $product = $calc->multiply(4, 7);

print "Sum: $sum\n";
print "Product: $product\n";
"#;

    lsp.did_open(uri.clone(), source.to_string(), 1);
    println!("✓ Document opened in LSP");

    // Get diagnostics
    let diagnostics = lsp.get_diagnostics(&uri);
    println!("\n✓ Diagnostics: {} issues found", diagnostics.len());
    for diag in &diagnostics {
        println!("  - Line {}: {}", diag.range.start.line, diag.message);
    }

    // Get document symbols
    let symbols = lsp.get_document_symbols(&uri);
    println!("\n✓ Document symbols: {} found", symbols.len());
    for symbol in &symbols {
        println!("  - {} ({:?})", symbol.name, symbol.kind);
        if let Some(container) = &symbol.container_name {
            println!("    in {}", container);
        }
    }

    // Get completions
    let completions = lsp.get_completions(&uri, LspPosition { line: 25, character: 10 });
    println!("\n✓ Completions at line 25: {} suggestions", completions.len());
    let sample_completions: Vec<_> = completions.iter().take(10).map(|c| &c.label).collect();
    println!("  - Sample: {:?}", sample_completions);

    // Simulate an edit
    let change = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: LspPosition { line: 25, character: 18 },
            end: LspPosition { line: 25, character: 19 },
        }),
        text: "10".to_string(),
    };

    lsp.did_change(uri.clone(), vec![change], 2);
    println!("\n✓ Document updated (changed 5 to 10)");

    // Check diagnostics after edit
    let new_diagnostics = lsp.get_diagnostics(&uri);
    println!("✓ Diagnostics after edit: {} issues", new_diagnostics.len());

    // Demonstrate error handling
    let error_uri = "file:///error.pl".to_string();
    let error_source = r#"
my $x = ;  # Missing value
if ($x {   # Missing closing paren
    print "test";
}
"#;

    lsp.did_open(error_uri.clone(), error_source.to_string(), 1);
    let error_diagnostics = lsp.get_diagnostics(&error_uri);
    println!("\n✓ Error detection: {} errors found", error_diagnostics.len());
    for diag in &error_diagnostics {
        println!("  - Line {}: {}", diag.range.start.line, diag.message);
    }
}

// Helper function to display edit information
#[allow(dead_code)]
fn display_edit_info(edit: &Edit) {
    println!("Edit info:");
    println!("  - Byte range: {} -> {}", edit.start_byte, edit.old_end_byte);
    println!(
        "  - Position: {}:{} -> {}:{}",
        edit.start_position.line,
        edit.start_position.column,
        edit.old_end_position.line,
        edit.old_end_position.column
    );
}
