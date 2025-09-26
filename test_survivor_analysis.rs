#!/usr/bin/env rust-script

//! Analysis of surviving mutations to understand test coverage gaps
//! This script creates targeted tests for the specific mutations that survived

use std::fs;
use std::process::Command;

fn main() {
    println!("Analyzing Surviving Mutations from Substitution Operator Testing");
    println!("================================================================");

    analyze_survivor_mut_002();
    analyze_survivor_mut_005();
    generate_recommendations();
}

/// Analyze MUT_002: Change && to || in replacement parsing condition
/// Original: if !is_paired && !rest1.is_empty()
/// Mutated:  if !is_paired || !rest1.is_empty()
fn analyze_survivor_mut_002() {
    println!("\n🔍 ANALYZING MUT_002: Logical operator change in replacement parsing");
    println!("Original: if !is_paired && !rest1.is_empty()");
    println!("Mutated:  if !is_paired || !rest1.is_empty()");

    println!("\nThis mutation changes the condition that determines how replacement");
    println!("text is parsed for substitution operators. The mutation survived");
    println!("because our current tests don't cover the specific edge case where:");
    println!("- is_paired = true (paired delimiters like s{}{})");
    println!("- rest1.is_empty() = true (empty replacement part)");

    println!("\nMissing test cases:");
    println!("1. s{pattern}{} - paired delimiters with empty replacement");
    println!("2. s[pattern][] - paired delimiters with empty replacement");
    println!("3. s<pattern><> - paired delimiters with empty replacement");

    println!("\nImpact: This logical change could cause incorrect parsing of");
    println!("paired delimiter substitutions with empty replacement parts.");
}

/// Analyze MUT_005: Change valid modifier characters
/// Original: 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'
/// Mutated:  'z' | 'q' | 'w' | 'n' | 'p' | 'k' | 'l' | 'v'
fn analyze_survivor_mut_005() {
    println!("\n🔍 ANALYZING MUT_005: Modifier validation change");
    println!("Original: 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'");
    println!("Mutated:  'z' | 'q' | 'w' | 'n' | 'p' | 'k' | 'l' | 'v'");

    println!("\nThis mutation changes which characters are accepted as valid");
    println!("substitution modifiers. The mutation survived because our tests");
    println!("don't validate that INVALID modifiers are properly rejected.");

    println!("\nMissing test cases:");
    println!("1. s/pattern/replacement/z - should fail or ignore invalid 'z'");
    println!("2. s/pattern/replacement/q - should fail or ignore invalid 'q'");
    println!("3. s/pattern/replacement/xyz - should only parse valid chars");
    println!("4. Error handling for invalid modifier combinations");

    println!("\nImpact: This change could allow invalid modifiers to be accepted");
    println!("as valid, potentially causing runtime errors or incorrect behavior.");
}

fn generate_recommendations() {
    println!("\n📋 RECOMMENDATIONS FOR TEST ENHANCEMENT");
    println!("=======================================");

    println!("\n🎯 HIGH PRIORITY - Route to test-hardener agent:");
    println!("The surviving mutations are highly localizable and indicate specific");
    println!("gaps in edge case testing rather than fundamental input validation issues.");

    println!("\n🔧 SPECIFIC TEST ADDITIONS NEEDED:");
    println!("1. Paired delimiter edge cases:");
    println!("   - Empty replacement with paired delimiters: s{a}{}, s[a][], s<a><>");
    println!("   - Whitespace handling between paired delimiters");
    println!("   - Nested delimiters in replacement part");

    println!("\n2. Modifier validation tests:");
    println!("   - Invalid modifiers should be rejected or ignored");
    println!("   - Mixed valid/invalid modifier combinations");
    println!("   - Error message validation for invalid modifiers");

    println!("\n3. Logic pathway coverage:");
    println!("   - Test all combinations of is_paired and rest1.is_empty()");
    println!("   - Verify correct parsing path selection");

    println!("\n📊 MUTATION SCORE ANALYSIS:");
    println!("Current score: 60% (3/5 mutations killed)");
    println!("Target score: 90%+ for substitution operator implementation");
    println!("Gap: 2 specific, localizable mutations need targeted test coverage");

    println!("\n🚀 NEXT STEPS:");
    println!("1. Pass findings to test-hardener agent for targeted test creation");
    println!("2. Focus on edge cases in delimiter handling and modifier validation");
    println!("3. Add boundary condition tests for logical operator combinations");
    println!("4. Verify error handling paths for malformed substitution operators");
}

#[allow(dead_code)]
fn create_enhanced_test_cases() {
    let enhanced_tests = r#"
/// Enhanced test cases to catch surviving mutations
#[test]
fn test_paired_delimiters_empty_replacement() {
    // These should catch MUT_002 (logical operator change)
    let test_cases = vec![
        "s{pattern}{}",     // Empty replacement with braces
        "s[pattern][]",     // Empty replacement with brackets
        "s<pattern><>",     // Empty replacement with angle brackets
        "s(pattern)()",     // Empty replacement with parentheses
    ];

    for code in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");

        // Verify empty replacement is handled correctly
        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { replacement, .. } = &expression.kind {
                    assert_eq!(replacement, "", "Empty replacement should be empty string for {}", code);
                }
            }
        }
    }
}

#[test]
fn test_invalid_modifiers_rejected() {
    // These should catch MUT_005 (modifier validation change)
    let invalid_modifiers = vec!["z", "q", "w", "n", "p", "k", "l", "v"];

    for modifier in invalid_modifiers {
        let code = format!("s/pattern/replacement/{}", modifier);
        let mut parser = Parser::new(&code);
        let ast = parser.parse().expect("parse");

        // Verify invalid modifiers are not included
        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { modifiers, .. } = &expression.kind {
                    assert!(!modifiers.contains(modifier), "Invalid modifier '{}' should not be accepted", modifier);
                }
            }
        }
    }
}
"#;

    println!("\n💡 SAMPLE ENHANCED TEST CODE:");
    println!("{}", enhanced_tests);
}