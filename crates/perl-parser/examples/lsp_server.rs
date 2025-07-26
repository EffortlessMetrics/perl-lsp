//! Example of a simple LSP server using perl-parser
//!
//! This example demonstrates how to build a basic Language Server Protocol
//! implementation for Perl using the perl-parser's IDE features.

use perl_parser::{LanguageServer, lsp};
use std::io::{self, BufRead, Write};

fn main() {
    println!("=== Perl LSP Server Demo ===");
    println!("This is a simple demonstration of LSP capabilities.");
    println!("Commands:");
    println!("  open <filename> - Open a file");
    println!("  goto <line> <col> - Go to definition at position");
    println!("  hover <line> <col> - Show hover info at position");
    println!("  refs <line> <col> - Find all references at position");
    println!("  symbols - Show document outline");
    println!("  highlight - Show semantic tokens");
    println!("  quit - Exit");
    println!();

    let mut server = LanguageServer::new();
    let mut current_file = None;
    
    // Demo file content
    let demo_code = r#"#!/usr/bin/perl
use strict;
use warnings;

package Calculator;

# Basic arithmetic operations
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

sub subtract {
    my ($a, $b) = @_;
    return $a - $b;
}

sub multiply {
    my ($a, $b) = @_;
    return $a * $b;
}

sub divide {
    my ($a, $b) = @_;
    die "Division by zero!" if $b == 0;
    return $a / $b;
}

# Calculate compound interest
sub compound_interest {
    my ($principal, $rate, $time, $n) = @_;
    $n ||= 1;  # Default to annual compounding
    
    my $amount = $principal * (1 + $rate / $n) ** ($n * $time);
    return $amount - $principal;
}

package main;

# Test the calculator
my $x = 10;
my $y = 5;

print "Addition: ", Calculator::add($x, $y), "\n";
print "Subtraction: ", Calculator::subtract($x, $y), "\n";
print "Multiplication: ", Calculator::multiply($x, $y), "\n";
print "Division: ", Calculator::divide($x, $y), "\n";

my $principal = 1000;
my $rate = 0.05;
my $time = 10;

my $interest = Calculator::compound_interest($principal, $rate, $time);
print "Compound interest on \$$principal at $rate% for $time years: \$$interest\n";
"#;

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    loop {
        print!("> ");
        stdout.flush().unwrap();
        
        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap();
        
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        match parts[0] {
            "open" => {
                if parts.len() > 1 {
                    let filename = parts[1];
                    server.open_document(
                        format!("file:///{}", filename),
                        1,
                        demo_code.to_string()
                    );
                    current_file = Some(format!("file:///{}", filename));
                    println!("Opened {} with demo content", filename);
                    
                    // Show some stats
                    if let Some(uri) = &current_file {
                        if let Some(doc) = server.documents.get(uri) {
                            if let Some(analyzer) = &doc.analyzer {
                                let symbol_count = analyzer.symbol_table().symbols.len();
                                let token_count = analyzer.semantic_tokens().len();
                                println!("  Found {} symbols and {} semantic tokens", symbol_count, token_count);
                            }
                        }
                    }
                } else {
                    println!("Usage: open <filename>");
                }
            }
            
            "goto" => {
                if parts.len() >= 3 {
                    if let Some(uri) = &current_file {
                        if let (Ok(line), Ok(col)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>()) {
                            let pos = lsp::Position { line: line - 1, character: col - 1 };
                            
                            if let Some(location) = server.goto_definition(uri, pos) {
                                println!("Definition found at line {}, column {}",
                                    location.target_range.start.line + 1,
                                    location.target_range.start.character + 1
                                );
                                
                                // Show a snippet of the definition
                                if let Some(doc) = server.documents.get(uri) {
                                    let lines: Vec<&str> = doc.text.lines().collect();
                                    if location.target_range.start.line < lines.len() {
                                        println!("  {}", lines[location.target_range.start.line].trim());
                                    }
                                }
                            } else {
                                println!("No definition found at that position");
                            }
                        } else {
                            println!("Invalid line or column number");
                        }
                    } else {
                        println!("No file open. Use 'open <filename>' first");
                    }
                } else {
                    println!("Usage: goto <line> <column>");
                }
            }
            
            "hover" => {
                if parts.len() >= 3 {
                    if let Some(uri) = &current_file {
                        if let (Ok(line), Ok(col)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>()) {
                            let pos = lsp::Position { line: line - 1, character: col - 1 };
                            
                            if let Some(hover) = server.hover(uri, pos) {
                                println!("Hover info:");
                                println!("{}", hover.contents);
                            } else {
                                println!("No hover information at that position");
                            }
                        }
                    } else {
                        println!("No file open");
                    }
                } else {
                    println!("Usage: hover <line> <column>");
                }
            }
            
            "refs" => {
                if parts.len() >= 3 {
                    if let Some(uri) = &current_file {
                        if let (Ok(line), Ok(col)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>()) {
                            let pos = lsp::Position { line: line - 1, character: col - 1 };
                            
                            let refs = server.find_references(uri, pos, true);
                            println!("Found {} references:", refs.len());
                            
                            for (i, reference) in refs.iter().enumerate() {
                                println!("  {}: Line {}, column {}",
                                    i + 1,
                                    reference.range.start.line + 1,
                                    reference.range.start.character + 1
                                );
                                
                                // Show snippet
                                if let Some(doc) = server.documents.get(uri) {
                                    let lines: Vec<&str> = doc.text.lines().collect();
                                    if reference.range.start.line < lines.len() {
                                        let line = lines[reference.range.start.line];
                                        println!("     {}", line.trim());
                                    }
                                }
                            }
                        }
                    } else {
                        println!("No file open");
                    }
                } else {
                    println!("Usage: refs <line> <column>");
                }
            }
            
            "symbols" => {
                if let Some(uri) = &current_file {
                    let symbols = server.document_symbols(uri);
                    println!("Document symbols:");
                    
                    for symbol in symbols {
                        let icon = match symbol.kind {
                            lsp::SymbolKindEnum::Function => "ƒ",
                            lsp::SymbolKindEnum::Variable => "v",
                            lsp::SymbolKindEnum::Module => "M",
                            lsp::SymbolKindEnum::Constant => "c",
                            _ => "•",
                        };
                        
                        println!("  {} {} (line {})",
                            icon,
                            symbol.name,
                            symbol.range.start.line + 1
                        );
                        
                        if let Some(detail) = symbol.detail {
                            println!("    {}", detail);
                        }
                    }
                } else {
                    println!("No file open");
                }
            }
            
            "highlight" => {
                if let Some(uri) = &current_file {
                    if let Some(tokens) = server.semantic_tokens(uri) {
                        println!("Semantic tokens (first 20):");
                        
                        let token_types = ["variable", "parameter", "function", "method", 
                                         "class", "namespace", "type", "keyword", 
                                         "modifier", "number", "string", "regex",
                                         "comment", "operator", "punctuation", "label"];
                        
                        for (i, token) in tokens.iter().take(20).enumerate() {
                            let type_name = token_types.get(token.token_type as usize)
                                .unwrap_or(&"unknown");
                            
                            println!("  {}: +{}:{} len={} type={}{}",
                                i + 1,
                                token.delta_line,
                                token.delta_start,
                                token.length,
                                type_name,
                                if token.token_modifiers != 0 {
                                    format!(" mods={:b}", token.token_modifiers)
                                } else {
                                    String::new()
                                }
                            );
                        }
                    }
                } else {
                    println!("No file open");
                }
            }
            
            "quit" | "exit" => {
                println!("Goodbye!");
                break;
            }
            
            _ => {
                println!("Unknown command: {}", parts[0]);
                println!("Type 'help' for available commands");
            }
        }
    }
}

// Note: In a real LSP implementation, you would:
// 1. Use the LSP protocol over JSON-RPC
// 2. Handle proper LSP messages (initialize, textDocument/didOpen, etc.)
// 3. Support incremental document updates
// 4. Implement workspace-wide features
// 5. Add diagnostics and code actions
// 6. Support multi-file projects

// This example just demonstrates the core functionality in a simple REPL format.