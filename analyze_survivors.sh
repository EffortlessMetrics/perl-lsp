#!/bin/bash

echo "Analyzing Surviving Mutations from Substitution Operator Testing"
echo "================================================================"

echo ""
echo "🔍 ANALYZING MUT_002: Logical operator change in replacement parsing"
echo "Original: if !is_paired && !rest1.is_empty()"
echo "Mutated:  if !is_paired || !rest1.is_empty()"
echo ""
echo "This mutation changes the condition that determines how replacement"
echo "text is parsed for substitution operators. The mutation survived"
echo "because our current tests don't cover the specific edge case where:"
echo "- is_paired = true (paired delimiters like s{pattern}{replacement})"
echo "- rest1.is_empty() = true (empty replacement part)"
echo ""
echo "Missing test cases:"
echo "1. s{pattern}{} - paired delimiters with empty replacement"
echo "2. s[pattern][] - paired delimiters with empty replacement"
echo "3. s<pattern><> - paired delimiters with empty replacement"
echo ""
echo "Impact: This logical change could cause incorrect parsing of"
echo "paired delimiter substitutions with empty replacement parts."

echo ""
echo "🔍 ANALYZING MUT_005: Modifier validation change"
echo "Original: 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'"
echo "Mutated:  'z' | 'q' | 'w' | 'n' | 'p' | 'k' | 'l' | 'v'"
echo ""
echo "This mutation changes which characters are accepted as valid"
echo "substitution modifiers. The mutation survived because our tests"
echo "don't validate that INVALID modifiers are properly rejected."
echo ""
echo "Missing test cases:"
echo "1. s/pattern/replacement/z - should fail or ignore invalid 'z'"
echo "2. s/pattern/replacement/q - should fail or ignore invalid 'q'"
echo "3. s/pattern/replacement/xyz - should only parse valid chars"
echo "4. Error handling for invalid modifier combinations"
echo ""
echo "Impact: This change could allow invalid modifiers to be accepted"
echo "as valid, potentially causing runtime errors or incorrect behavior."

echo ""
echo "📋 RECOMMENDATIONS FOR TEST ENHANCEMENT"
echo "======================================="
echo ""
echo "🎯 HIGH PRIORITY - Route to test-hardener agent:"
echo "The surviving mutations are highly localizable and indicate specific"
echo "gaps in edge case testing rather than fundamental input validation issues."
echo ""
echo "🔧 SPECIFIC TEST ADDITIONS NEEDED:"
echo "1. Paired delimiter edge cases:"
echo "   - Empty replacement with paired delimiters: s{a}{}, s[a][], s<a><>"
echo "   - Whitespace handling between paired delimiters"
echo "   - Nested delimiters in replacement part"
echo ""
echo "2. Modifier validation tests:"
echo "   - Invalid modifiers should be rejected or ignored"
echo "   - Mixed valid/invalid modifier combinations"
echo "   - Error message validation for invalid modifiers"
echo ""
echo "3. Logic pathway coverage:"
echo "   - Test all combinations of is_paired and rest1.is_empty()"
echo "   - Verify correct parsing path selection"
echo ""
echo "📊 MUTATION SCORE ANALYSIS:"
echo "Current score: 60% (3/5 mutations killed)"
echo "Target score: 90%+ for substitution operator implementation"
echo "Gap: 2 specific, localizable mutations need targeted test coverage"
echo ""
echo "🚀 NEXT STEPS:"
echo "1. Pass findings to test-hardener agent for targeted test creation"
echo "2. Focus on edge cases in delimiter handling and modifier validation"
echo "3. Add boundary condition tests for logical operator combinations"
echo "4. Verify error handling paths for malformed substitution operators"

echo ""
echo "💡 SAMPLE ENHANCED TEST CODE:"
echo "============================"
cat << 'EOF'
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
EOF