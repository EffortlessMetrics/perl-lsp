#!/usr/bin/env rust-script

//! Manual mutation testing script for substitution operator parsing
//! This performs targeted mutations on critical logic paths in substitution parsing

use std::process::Command;
use std::fs;
use std::path::Path;

/// Test results from a single mutation
#[derive(Debug)]
struct MutationResult {
    mutation_id: String,
    description: String,
    survived: bool,
    test_output: String,
}

fn main() {
    println!("Manual Mutation Testing for Substitution Operator Implementation");
    println!("================================================================");

    let mutations = define_critical_mutations();
    let mut results = Vec::new();

    for mutation in mutations {
        println!("\nTesting mutation: {}", mutation.description);
        let result = test_mutation(&mutation);
        println!("Result: {}", if result.survived { "SURVIVED" } else { "KILLED" });
        results.push(result);
    }

    generate_report(&results);
}

/// Define critical mutations for substitution parsing
fn define_critical_mutations() -> Vec<MutationSpec> {
    vec![
        MutationSpec {
            id: "MUT_001".to_string(),
            description: "Change delimiter != closing to == in extract_substitution_parts".to_string(),
            file: "crates/perl-parser/src/quote_parser.rs",
            line: 56,
            original: "let is_paired = delimiter != closing;",
            mutated: "let is_paired = delimiter == closing;",
        },
        MutationSpec {
            id: "MUT_002".to_string(),
            description: "Change && to || in non-paired delimiter check".to_string(),
            file: "crates/perl-parser/src/quote_parser.rs",
            line: 80,
            original: "let (replacement, modifiers_str) = if !is_paired && !rest1.is_empty() {",
            mutated: "let (replacement, modifiers_str) = if !is_paired || !rest1.is_empty() {",
        },
        MutationSpec {
            id: "MUT_003".to_string(),
            description: "Return empty strings in extract_substitution_parts".to_string(),
            file: "crates/perl-parser/src/quote_parser.rs",
            line: 46,
            original: "pub fn extract_substitution_parts(text: &str) -> (String, String, String) {",
            mutated: "pub fn extract_substitution_parts(_text: &str) -> (String, String, String) { return (String::new(), String::new(), String::new()); //",
        },
        MutationSpec {
            id: "MUT_004".to_string(),
            description: "Change escape check from == to != in delimiter parsing".to_string(),
            file: "crates/perl-parser/src/quote_parser.rs",
            line: 99,
            original: "c if c == closing => {",
            mutated: "c if c != closing => {",
        },
        MutationSpec {
            id: "MUT_005".to_string(),
            description: "Change valid modifier check in parse_substitution_parts".to_string(),
            file: "crates/perl-parser/src/parser_backup.rs",
            line: 4231,
            original: "'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' => {",
            mutated: "'z' | 'q' | 'w' | 'n' | 'p' | 'k' | 'l' | 'v' => {",
        },
    ]
}

#[derive(Debug)]
struct MutationSpec {
    id: String,
    description: String,
    file: &'static str,
    line: u32,
    original: &'static str,
    mutated: &'static str,
}

fn test_mutation(mutation: &MutationSpec) -> MutationResult {
    // Apply mutation
    let backup = backup_file(mutation.file);
    apply_mutation(mutation);

    // Run tests
    let test_output = run_substitution_tests();
    let survived = test_output.contains("test result: ok");

    // Restore original
    restore_file(mutation.file, &backup);

    MutationResult {
        mutation_id: mutation.id.clone(),
        description: mutation.description.clone(),
        survived,
        test_output,
    }
}

fn backup_file(file_path: &str) -> String {
    fs::read_to_string(file_path).expect("Failed to read file")
}

fn apply_mutation(mutation: &MutationSpec) {
    let content = fs::read_to_string(mutation.file).expect("Failed to read file");
    let mutated_content = content.replace(mutation.original, mutation.mutated);
    fs::write(mutation.file, mutated_content).expect("Failed to write mutated file");
}

fn restore_file(file_path: &str, backup: &str) {
    fs::write(file_path, backup).expect("Failed to restore file");
}

fn run_substitution_tests() -> String {
    let output = Command::new("cargo")
        .args(&["test", "--test", "substitution_operator_tests"])
        .output()
        .expect("Failed to run tests");

    format!("{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr))
}

fn generate_report(results: &[MutationResult]) {
    println!("\n\nMutation Testing Report");
    println!("======================");

    let total = results.len();
    let survivors = results.iter().filter(|r| r.survived).count();
    let killed = total - survivors;

    println!("Total mutations: {}", total);
    println!("Killed: {}", killed);
    println!("Survived: {}", survivors);
    println!("Mutation score: {:.1}%", (killed as f64 / total as f64) * 100.0);

    println!("\nSurviving mutations:");
    for result in results.iter().filter(|r| r.survived) {
        println!("- {}: {}", result.mutation_id, result.description);
    }

    println!("\nRouting Decision:");
    if survivors == 0 {
        println!("✅ Perfect mutation score - route to fuzz-tester for next validation phase");
    } else if survivors <= 2 && survivors_are_localizable(results) {
        println!("🔧 Route to test-hardener - survivors are localizable and targetable");
    } else {
        println!("🎯 Route to fuzz-tester - survivors suggest input-shape gaps");
    }
}

fn survivors_are_localizable(results: &[MutationResult]) -> bool {
    let survivors: Vec<_> = results.iter().filter(|r| r.survived).collect();

    // Check if survivors are clustered around specific logic areas
    survivors.len() <= 2 && survivors.iter().all(|s|
        s.description.contains("delimiter") ||
        s.description.contains("modifier") ||
        s.description.contains("escape")
    )
}