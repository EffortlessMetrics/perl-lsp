#!/usr/bin/env rust-script
//! Validate that Format, Glob, and Tie NodeKinds are actually produced
//!
//! This script parses the test corpus files and verifies that the expected
//! NodeKinds are present in the AST.
//!
//! ```cargo
//! [dependencies]
//! perl-parser = { path = "crates/perl-parser" }
//! ```

use perl_parser::{Parser, NodeKind};
use std::fs;
use std::collections::HashSet;

fn collect_nodekinds(node: &perl_parser::ast::Node, set: &mut HashSet<String>) {
    set.insert(node.kind.kind_name().to_string());
    node.for_each_child(|child| {
        collect_nodekinds(child, set);
    });
}

fn validate_file(path: &str, expected_nodekinds: &[&str]) -> Result<(), String> {
    println!("Validating: {}", path);

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;

    let mut parser = Parser::new(&content);
    let ast = parser.parse()
        .map_err(|e| format!("Failed to parse {}: {}", path, e))?;

    let mut nodekinds = HashSet::new();
    collect_nodekinds(&ast, &mut nodekinds);

    println!("  Found {} unique NodeKinds", nodekinds.len());

    let mut all_found = true;
    for expected in expected_nodekinds {
        if nodekinds.contains(*expected) {
            println!("  ✅ {}", expected);
        } else {
            println!("  ❌ {} NOT FOUND", expected);
            all_found = false;
        }
    }

    if all_found {
        Ok(())
    } else {
        Err(format!("Missing NodeKinds in {}", path))
    }
}

fn main() {
    println!("=== NodeKind Coverage Validation ===\n");

    let mut errors = Vec::new();

    // Validate Format
    println!("--- Format NodeKind ---");
    if let Err(e) = validate_file("test_corpus/format_statements.pl", &["Format"]) {
        errors.push(e);
    }
    println!();

    // Validate Glob
    println!("--- Glob NodeKind ---");
    if let Err(e) = validate_file("test_corpus/glob_expressions.pl", &["Glob"]) {
        errors.push(e);
    }
    println!();

    // Validate Tie
    println!("--- Tie NodeKind ---");
    if let Err(e) = validate_file("test_corpus/tie_interface.pl", &["Tie", "Untie"]) {
        errors.push(e);
    }
    println!();

    // Validate Variable (for Sigil)
    println!("--- Variable NodeKind (for Sigil verification) ---");
    if let Err(e) = validate_file("test_corpus/format_statements.pl", &["Variable"]) {
        errors.push(e);
    }
    println!();

    // Summary
    println!("=== Validation Summary ===");
    if errors.is_empty() {
        println!("✅ ALL VALIDATIONS PASSED");
        println!("\nAll NodeKinds from issue #446 are properly implemented:");
        println!("  ✅ Format - Found in AST");
        println!("  ✅ Glob   - Found in AST");
        println!("  ✅ Tie    - Found in AST (+ Untie)");
        println!("  ✅ Sigil  - Part of Variable NodeKind (intentional design)");
        std::process::exit(0);
    } else {
        println!("❌ VALIDATION FAILED");
        for error in &errors {
            println!("  - {}", error);
        }
        std::process::exit(1);
    }
}
