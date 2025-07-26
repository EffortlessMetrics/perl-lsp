//! Example demonstrating IDE features of the perl-parser
//!
//! This example shows how to use the symbol extraction, semantic analysis,
//! and LSP features for building IDE functionality.

use perl_parser::{Parser, SymbolExtractor, SemanticAnalyzer, LanguageServer, lsp};

fn main() {
    let code = r#"
package MyModule;

use strict;
use warnings;

# Calculate the factorial of a number
sub factorial {
    my ($n) = @_;
    
    if ($n <= 1) {
        return 1;
    }
    
    return $n * factorial($n - 1);
}

# Main program
my $num = 5;
my $result = factorial($num);

print "The factorial of $num is $result\n";

# Another example with more complex scoping
sub process_data {
    my ($data) = @_;
    my @results;
    
    foreach my $item (@$data) {
        my $processed = $item * 2;
        push @results, $processed;
    }
    
    return \@results;
}

my @numbers = (1, 2, 3, 4, 5);
my $processed = process_data(\@numbers);
"#;

    println!("=== Perl IDE Features Demo ===\n");

    // 1. Parse the code
    let mut parser = Parser::new(code);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return;
        }
    };

    println!("✓ Successfully parsed the code\n");

    // 2. Extract symbols
    println!("=== Symbol Table ===");
    let symbol_table = SymbolExtractor::new().extract(&ast);
    
    println!("Symbols found:");
    for (name, symbols) in &symbol_table.symbols {
        for symbol in symbols {
            println!("  {} {} - {:?} at {}:{}",
                symbol.kind.sigil().unwrap_or(""),
                name,
                symbol.kind,
                symbol.location.start,
                symbol.location.end
            );
            if let Some(decl) = &symbol.declaration {
                println!("    Declaration: {}", decl);
            }
            if !symbol.attributes.is_empty() {
                println!("    Attributes: {}", symbol.attributes.join(", "));
            }
        }
    }
    
    println!("\nScopes:");
    for (id, scope) in &symbol_table.scopes {
        println!("  Scope {} - {:?}", id, scope.kind);
        if !scope.symbols.is_empty() {
            println!("    Symbols: {}", scope.symbols.iter().cloned().collect::<Vec<_>>().join(", "));
        }
    }

    // 3. Semantic analysis
    println!("\n=== Semantic Analysis ===");
    let analyzer = SemanticAnalyzer::analyze(&ast);
    
    println!("Semantic tokens (first 10):");
    for (i, token) in analyzer.semantic_tokens().iter().take(10).enumerate() {
        println!("  {}: {:?} at {}:{} {}",
            i,
            token.token_type,
            token.location.start,
            token.location.end,
            if token.modifiers.is_empty() { 
                String::new() 
            } else { 
                format!("(modifiers: {:?})", token.modifiers)
            }
        );
    }

    // 4. Language Server demonstration
    println!("\n=== Language Server Features ===");
    let mut lsp = LanguageServer::new();
    lsp.open_document("file:///example.pl".to_string(), 1, code.to_string());

    // Test go-to-definition
    // Find position of $num in "factorial($num)"
    let lines: Vec<&str> = code.lines().collect();
    let mut target_line = 0;
    let mut target_col = 0;
    
    for (line_idx, line) in lines.iter().enumerate() {
        if line.contains("factorial($num)") {
            target_line = line_idx;
            if let Some(pos) = line.find("$num") {
                target_col = pos;
                break;
            }
        }
    }
    
    let position = lsp::Position { line: target_line, character: target_col };
    
    println!("\nTesting go-to-definition for $num at line {}, column {}:", target_line + 1, target_col + 1);
    if let Some(location) = lsp.goto_definition("file:///example.pl", position) {
        println!("  → Definition found at line {}, column {}",
            location.target_range.start.line + 1,
            location.target_range.start.character + 1
        );
    } else {
        println!("  → No definition found");
    }

    // Test hover
    println!("\nTesting hover information:");
    if let Some(hover) = lsp.hover("file:///example.pl", position) {
        println!("  Hover info: {}", hover.contents);
    }

    // Test find references
    println!("\nTesting find-all-references for $num:");
    let references = lsp.find_references("file:///example.pl", position, true);
    println!("  Found {} references:", references.len());
    for reference in references {
        println!("    - Line {}, column {}",
            reference.range.start.line + 1,
            reference.range.start.character + 1
        );
    }

    // Test document symbols (outline)
    println!("\n=== Document Symbols (Outline) ===");
    let symbols = lsp.document_symbols("file:///example.pl");
    println!("Document structure:");
    for symbol in symbols {
        println!("  {} {} - {:?}",
            match symbol.kind {
                lsp::SymbolKindEnum::Function => "⚡",
                lsp::SymbolKindEnum::Variable => "📦",
                lsp::SymbolKindEnum::Module => "📁",
                _ => "•",
            },
            symbol.name,
            symbol.kind
        );
        if let Some(detail) = symbol.detail {
            println!("    {}", detail);
        }
    }

    // Test semantic tokens for syntax highlighting
    println!("\n=== Semantic Tokens for Syntax Highlighting ===");
    if let Some(tokens) = lsp.semantic_tokens("file:///example.pl") {
        println!("First 5 semantic tokens:");
        for token in tokens.iter().take(5) {
            println!("  Line +{}, char +{}, len {}, type {}, modifiers {}",
                token.delta_line,
                token.delta_start,
                token.length,
                token.token_type,
                token.token_modifiers
            );
        }
    }

    println!("\n✓ IDE features demonstration complete!");
}