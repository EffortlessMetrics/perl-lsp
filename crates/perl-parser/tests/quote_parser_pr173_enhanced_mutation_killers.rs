/// Enhanced Mutation Killer Tests for PR #173 - Targeting ~66% → 80%+ Mutation Score
///
/// This test file specifically addresses the 28+ surviving mutants identified in PR #173
/// mutation testing analysis. It focuses on killing specific mutation types that survived
/// previous test passes:
///
/// 1. FnValue mutations: String::new() returns in extract_regex_parts()
/// 2. BinaryOperator mutations: && to || logic mutations in boundary conditions
/// 3. Position arithmetic mutations: += to -=, + to - in delimiter parsing
/// 4. Match guard mutations: c == closing boundary detection failures
/// 5. UnaryOperator mutations: ! operator removal in character detection
///
/// Target: Achieve ≥80% mutation score by systematically eliminating survivors
/// through comprehensive edge case validation and boundary condition testing.
///
/// Labels: tests:mutation-hardening, tests:pr173-enhanced, tests:comprehensive-coverage
use perl_parser::quote_parser::*;

// MUTATION KILLER TARGET: extract_regex_parts() FnValue mutations
// Specifically targets String::new() return value mutations that survive existing tests
#[test]
fn test_kill_extract_regex_parts_string_new_mutations() {
    // Test case 1: Empty qr without delimiter returns empty strings - validate this behavior
    // This validates the correct empty behavior and kills mutations that would return non-empty
    let (pattern, _body, modifiers) = extract_regex_parts("qr");
    assert_eq!(pattern, "", "Pattern should be empty string for incomplete 'qr'");
    assert_eq!(modifiers, "", "Modifiers should be empty for incomplete qr");

    // Test case 2: Single 'm' character handling - critical boundary case
    // Kills mutations where String::new() is returned instead of proper parsing
    let (pattern, _body, modifiers) = extract_regex_parts("m");
    assert_ne!(
        pattern, "",
        "Pattern should not be empty for single 'm' - kills String::new() FnValue mutation"
    );
    assert_eq!(pattern, "mm", "Single 'm' should return 'mm' as pattern");
    assert_eq!(modifiers, "", "Modifiers should be empty for single 'm'");

    // Test case 3: Edge case with 'm' followed by alphabetic character
    // Targets specific boolean logic where character detection could be mutated
    let (pattern, _body, modifiers) = extract_regex_parts("ma");
    assert_ne!(pattern, "", "Pattern must not be empty - kills String::new() return mutation");
    assert_eq!(pattern, "mam", "Expected 'mam' pattern for 'ma' input");
    assert_eq!(modifiers, "", "No modifiers expected for 'ma'");

    // Test case 4: Boundary between valid and invalid regex patterns
    // This specifically tests the boundary where text.len() > 1 conditions could be mutated
    let (pattern, _body, modifiers) = extract_regex_parts("m/");
    assert_ne!(pattern, "", "Non-empty pattern required - kills String::new() mutation");
    assert_eq!(pattern, "//", "Pattern should be '//' for 'm/' input");
    assert_eq!(modifiers, "", "No modifiers for 'm/' input");

    // Test case 5: Complex regex with multiple components - ensures no String::new() returns
    let (pattern, _body, modifiers) = extract_regex_parts("qr{test.*}ims");
    assert_ne!(
        pattern, "",
        "Complex pattern must not be empty - kills String::new() FnValue mutation"
    );
    assert_ne!(
        modifiers, "",
        "Modifiers must not be empty - kills String::new() modifiers mutation"
    );
    assert_eq!(pattern, "{test.*}", "Pattern extraction failed");
    assert_eq!(modifiers, "ims", "Modifiers extraction failed");
}

// MUTATION KILLER TARGET: extract_substitution_parts() Boolean Logic mutations
// Targets && to || mutations and boundary condition logic failures
#[test]
fn test_kill_extract_substitution_parts_boolean_logic_mutations() {
    // Test case 1: Paired delimiter logic - specifically targets is_paired && conditions
    // If && is mutated to ||, the logic fails for paired delimiters
    let (pattern, replacement, modifiers) = extract_substitution_parts("s{test}{replace}g");
    assert_eq!(
        pattern, "test",
        "Pattern parsing failed - kills && to || mutation in paired delimiter detection"
    );
    assert_eq!(
        replacement, "replace",
        "Replacement parsing failed - validates boolean logic integrity"
    );
    assert_eq!(modifiers, "g", "Modifiers extraction failed");

    // Test case 2: Non-paired delimiter with complex escaping
    // Tests !is_paired && !rest1.is_empty() boundary conditions
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/test\\/ed/repl\\/ace/gi");
    assert_eq!(pattern, "test\\/ed", "Escaped pattern failed - kills boolean logic mutations");
    assert_eq!(
        replacement, "repl\\/ace",
        "Escaped replacement failed - validates && logic integrity"
    );
    assert_eq!(modifiers, "gi", "Modifiers should be 'gi'");

    // Test case 3: Edge case where second delimiter might be missing
    // Targets the trimmed.starts_with(delimiter) && logic paths
    let (pattern, replacement, __modifiers) = extract_substitution_parts("s[test]replacement]");
    assert_eq!(pattern, "test", "Pattern should be 'test' - kills boolean logic mutations");
    // This is a malformed case but should be handled gracefully
    assert_ne!(replacement, "", "Replacement should not be empty - validates boundary logic");

    // Test case 4: Parentheses special case handling
    // Tests specific if delimiter == '(' && logic branches
    let (pattern, replacement, __modifiers) = extract_substitution_parts("s(test)");
    assert_eq!(pattern, "test", "Pattern extraction failed for parentheses");
    // For malformed parentheses cases, replacement should be empty per the logic
    assert_eq!(replacement, "", "Parentheses special case handling failed");

    // Test case 5: Complex nested delimiters with boolean boundary conditions
    let (pattern, replacement, modifiers) =
        extract_substitution_parts("s{test{nested}}{repl{nested}}g");
    assert_eq!(
        pattern, "test{nested}",
        "Nested pattern parsing failed - kills complex boolean mutations"
    );
    assert_eq!(
        replacement, "repl{nested}",
        "Nested replacement parsing failed - validates boolean logic"
    );
    assert_eq!(modifiers, "g", "Modifiers should be 'g'");
}

// MUTATION KILLER TARGET: extract_transliteration_parts() Boundary Condition mutations
// Focuses on arithmetic and position calculation mutations
#[test]
fn test_kill_extract_transliteration_parts_boundary_mutations() {
    // Test case 1: Basic transliteration with position-critical parsing
    // Targets arithmetic mutations in position calculations (+ to -, += to -=)
    let (search, replacement, modifiers) = extract_transliteration_parts("tr/abc/xyz/d");
    assert_eq!(search, "abc", "Search pattern failed - kills position arithmetic mutations");
    assert_eq!(
        replacement, "xyz",
        "Replacement pattern failed - validates position calculation integrity"
    );
    assert_eq!(modifiers, "d", "Modifiers should be 'd'");

    // Test case 2: Paired delimiters with complex nesting
    // Tests position calculations where depth += 1 could be mutated to depth -= 1
    let (search, replacement, modifiers) = extract_transliteration_parts("tr{a{b}c}{x{y}z}s");
    assert_eq!(search, "a{b}c", "Nested search failed - kills depth arithmetic mutations");
    assert_eq!(
        replacement, "x{y}z",
        "Nested replacement failed - validates depth calculation logic"
    );
    assert_eq!(modifiers, "s", "Modifiers should be 's'");

    // Test case 3: Escaped delimiter handling with position arithmetic
    // Targets end_pos = i + ch.len_utf8() arithmetic that could be mutated
    let (search, replacement, modifiers) = extract_transliteration_parts("tr/a\\/b/x\\/y/");
    assert_eq!(search, "a\\/b", "Escaped search failed - kills position increment mutations");
    assert_eq!(replacement, "x\\/y", "Escaped replacement failed - validates position arithmetic");
    assert_eq!(modifiers, "", "No modifiers expected");

    // Test case 4: Edge case with missing second delimiter for paired delimiters
    // Tests boundary conditions where position arithmetic is critical
    let (search, replacement, __modifiers) = extract_transliteration_parts("tr{test}incomplete");
    assert_eq!(
        search, "test",
        "Search pattern should be 'test' - kills boundary arithmetic mutations"
    );
    // For incomplete paired delimiters, replacement should be empty per logic
    assert_eq!(replacement, "", "Incomplete paired delimiter should result in empty replacement");

    // Test case 5: Y prefix with complex character ranges
    let (search, replacement, modifiers) = extract_transliteration_parts("y/a-z/A-Z/r");
    assert_eq!(search, "a-z", "Character range search failed - kills position boundary mutations");
    assert_eq!(
        replacement, "A-Z",
        "Character range replacement failed - validates arithmetic integrity"
    );
    assert_eq!(modifiers, "r", "Modifiers should be 'r'");
}

// MUTATION KILLER TARGET: Match Guard mutations (c == closing)
// Specifically targets match guard conditions that control delimiter detection
#[test]
fn test_kill_match_guard_closing_delimiter_mutations() {
    // Test case 1: Critical closing delimiter detection in substitution
    // If c == closing is mutated to true/false, parsing fails catastrophically
    let (pattern, replacement, __modifiers) = extract_substitution_parts("s/test/replace/");
    assert_eq!(
        pattern, "test",
        "Pattern parsing failed - kills c == closing match guard mutations"
    );
    assert_eq!(
        replacement, "replace",
        "Replacement parsing failed - validates closing delimiter detection"
    );

    // Test case 2: Multiple closing delimiters where only the right one should match
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/te/st/rep/lace/g");
    assert_eq!(pattern, "te", "First delimiter section failed - kills premature closing detection");
    assert_eq!(replacement, "st", "Second delimiter section failed - validates match guard logic");
    // Note: modifiers are extracted from text after third delimiter, filtered to alphabetic chars
    assert_eq!(modifiers, "rep", "Modifiers extracted from remaining text (stops at next /)");

    // Test case 3: Transliteration with critical closing delimiter detection
    let (search, replacement, modifiers) = extract_transliteration_parts("tr/a/b/c/x/y/z/");
    assert_eq!(
        search, "a",
        "Search parsing stopped at first delimiter - validates closing delimiter detection"
    );
    assert_eq!(
        replacement, "b",
        "Replacement parsing follows delimiter logic - validates match guard"
    );
    assert_eq!(
        modifiers, "c",
        "Modifiers extracted from remaining text - validates position tracking"
    );

    // Test case 4: Regex with closing delimiter in content
    let (pattern, _body, modifiers) = extract_regex_parts("m/test\\/regex/i");
    assert_eq!(
        pattern, "/test\\/regex/",
        "Regex with escaped delimiter failed - kills match guard mutations"
    );
    assert_eq!(modifiers, "i", "Modifiers should be 'i'");

    // Test case 5: Paired delimiters where closing delimiter appears in nested context
    let (pattern, replacement, modifiers) =
        extract_substitution_parts("s<test<nested>><repl<nested>>g");
    assert_eq!(
        pattern, "test<nested>",
        "Nested angle brackets failed - kills closing delimiter match guard mutations"
    );
    assert_eq!(
        replacement, "repl<nested>",
        "Nested replacement failed - validates nested delimiter handling"
    );
    assert_eq!(modifiers, "g", "Modifiers should be 'g'");
}

// MUTATION KILLER TARGET: Unary Operator mutations (! removal)
// Targets .is_alphabetic() negation and other unary operator mutations
#[test]
fn test_kill_unary_operator_mutations() {
    // Test case 1: Alphabetic character detection where ! could be removed
    // !text.chars().nth(1).unwrap().is_alphabetic() mutations
    let (pattern, _body, modifiers) = extract_regex_parts("ma");
    assert_eq!(pattern, "mam", "Alphabetic detection failed - kills ! operator removal mutation");
    assert_eq!(modifiers, "", "No modifiers expected for 'ma'");

    // Test case 2: Non-alphabetic character that should be processed differently
    let (pattern, _body, modifiers) = extract_regex_parts("m/");
    assert_eq!(pattern, "//", "Non-alphabetic processing failed - validates ! operator integrity");
    assert_eq!(modifiers, "", "No modifiers for 'm/'");

    // Test case 3: Edge case with numeric character after 'm'
    let (pattern, _body, _modifiers) = extract_regex_parts("m1");
    assert_eq!(pattern, "11", "Numeric character handling failed - kills ! operator mutations");

    // Test case 4: Special character handling
    let (pattern, _body, _modifiers) = extract_regex_parts("m#");
    assert_eq!(pattern, "##", "Special character processing failed - validates ! operator logic");

    // Test case 5: Unicode character boundary testing
    let (pattern, _body, _modifiers) = extract_regex_parts("mα");
    assert_eq!(pattern, "mαm", "Unicode alphabetic failed - kills ! operator removal for unicode");
}

// MUTATION KILLER TARGET: Arithmetic Operator mutations in position calculations
// Focuses on += to -=, + to -, and similar arithmetic mutations
#[test]
fn test_kill_arithmetic_position_mutations() {
    // Test case 1: Complex nested structure requiring accurate position tracking
    let (pattern, replacement, _modifiers) = extract_substitution_parts("s{a{b{c}d}e}{x{y{z}w}v}g");
    assert_eq!(
        pattern, "a{b{c}d}e",
        "Complex nesting failed - kills arithmetic position mutations"
    );
    assert_eq!(
        replacement, "x{y{z}w}v",
        "Complex replacement failed - validates position arithmetic"
    );

    // Test case 2: Long content with multiple escape sequences requiring position accuracy
    let (pattern, replacement, _modifiers) =
        extract_substitution_parts("s/a\\\\b\\/c\\\\d/x\\\\y\\/z\\\\w/");
    assert_eq!(
        pattern, "a\\\\b\\/c\\\\d",
        "Escape sequences failed - kills position increment mutations"
    );
    assert_eq!(
        replacement, "x\\\\y\\/z\\\\w",
        "Replacement escapes failed - validates arithmetic integrity"
    );

    // Test case 3: Transliteration with character position tracking
    let (search, replacement, _modifiers) =
        extract_transliteration_parts("tr{abc{def}ghi}{xyz{uvw}rst}");
    assert_eq!(
        search, "abc{def}ghi",
        "Position tracking in search failed - kills arithmetic mutations"
    );
    assert_eq!(
        replacement, "xyz{uvw}rst",
        "Position tracking in replacement failed - validates calculations"
    );

    // Test case 4: Regex with multiple delimiter styles testing position arithmetic
    let (pattern, _body, modifiers) = extract_regex_parts("qr<test<inner>content>ims");
    assert_eq!(
        pattern, "<test<inner>content>",
        "Nested angle brackets failed - kills position arithmetic mutations"
    );
    assert_eq!(modifiers, "ims", "Modifiers should be 'ims'");

    // Test case 5: Edge case with zero-length content requiring precise position handling
    let (pattern, replacement, modifiers) = extract_substitution_parts("s{}{}g");
    assert_eq!(pattern, "", "Empty pattern handling failed - kills zero-length position mutations");
    assert_eq!(
        replacement, "",
        "Empty replacement handling failed - validates position boundary arithmetic"
    );
    assert_eq!(modifiers, "g", "Modifiers should be 'g'");
}

// MUTATION KILLER TARGET: Comprehensive integration test for multiple mutation types
// Tests combinations of mutations that might survive individual targeted tests
#[test]
fn test_kill_comprehensive_mutation_combinations() {
    // Test case 1: Complex scenario combining boolean logic, arithmetic, and match guards
    let test_cases = vec![
        // (input, expected_pattern, expected_replacement, expected_modifiers, description)
        (
            "s/a\\/b\\/c/x\\/y\\/z/gi",
            "a\\/b\\/c",
            "x\\/y\\/z",
            "gi",
            "Multiple escapes with boolean logic",
        ),
        ("s{a{b}c}{x{y}z}g", "a{b}c", "x{y}z", "g", "Paired delimiters with arithmetic"),
        ("tr[a-z][A-Z]d", "a-z", "A-Z", "d", "Character ranges with position tracking"),
        ("qr(test(nested))ims", "(test(nested))", "ims", "", "Complex regex with match guards"),
        (
            "s|test\\|pipe|repl\\|pipe|g",
            "test\\|pipe",
            "repl\\|pipe",
            "g",
            "Pipe delimiters with escaping",
        ),
    ];

    for (input, expected_pattern, expected_replacement, expected_modifiers, description) in
        test_cases
    {
        if input.starts_with("s") {
            let (pattern, replacement, modifiers) = extract_substitution_parts(input);
            assert_eq!(pattern, expected_pattern, "Pattern failed for {}: {}", description, input);
            assert_eq!(
                replacement, expected_replacement,
                "Replacement failed for {}: {}",
                description, input
            );
            assert_eq!(
                modifiers, expected_modifiers,
                "Modifiers failed for {}: {}",
                description, input
            );
        } else if input.starts_with("tr") {
            let (search, replacement, modifiers) = extract_transliteration_parts(input);
            assert_eq!(search, expected_pattern, "Search failed for {}: {}", description, input);
            assert_eq!(
                replacement, expected_replacement,
                "Replacement failed for {}: {}",
                description, input
            );
            assert_eq!(
                modifiers, expected_modifiers,
                "Modifiers failed for {}: {}",
                description, input
            );
        } else if input.starts_with("qr") || input.starts_with("m") {
            let (pattern, _body, modifiers) = extract_regex_parts(input);
            assert_eq!(pattern, expected_pattern, "Pattern failed for {}: {}", description, input);
            assert_eq!(
                modifiers, expected_replacement,
                "Modifiers failed for {}: {}",
                description, input
            ); // Note: reusing replacement field for modifiers
        }
    }
}

// MUTATION KILLER TARGET: Boundary condition stress testing
// Extreme edge cases designed to catch surviving boundary mutations
#[test]
fn test_kill_boundary_condition_stress_cases() {
    // Test case 1: Maximum nesting depth to stress arithmetic position calculations
    let (pattern, replacement, _modifiers) =
        extract_substitution_parts("s{{{{{test}}}}}{{{{{repl}}}}}g");
    assert_eq!(pattern, "{{{{test}}}}", "Deep nesting failed - kills depth arithmetic mutations");
    assert_eq!(
        replacement, "{{{{repl}}}}",
        "Deep replacement failed - validates depth calculation integrity"
    );

    // Test case 2: Alternating escape patterns to stress position arithmetic
    let (pattern, replacement, _modifiers) = extract_substitution_parts("s/\\\\\\//\\\\\\//");
    assert_eq!(
        pattern, "\\\\\\/",
        "Alternating escapes failed - kills position increment mutations"
    );
    assert_eq!(
        replacement, "\\\\\\/",
        "Replacement escapes failed - validates position calculations"
    );

    // Test case 3: Single character boundaries for all delimiter types
    let single_char_tests = vec![
        ("s/a/b/", "a", "b", ""),
        ("tr/a/b/", "a", "b", ""),
        ("s{a}{b}", "a", "b", ""),
        ("s[a][b]", "a", "b", ""),
        ("s<a><b>", "a", "b", ""),
        ("s(a)(b)", "a", "b", ""),
    ];

    for (input, expected_pattern, expected_replacement, expected_modifiers) in single_char_tests {
        if input.starts_with("s") {
            let (pattern, replacement, modifiers) = extract_substitution_parts(input);
            assert_eq!(pattern, expected_pattern, "Single char pattern failed for: {}", input);
            assert_eq!(
                replacement, expected_replacement,
                "Single char replacement failed for: {}",
                input
            );
            assert_eq!(
                modifiers, expected_modifiers,
                "Single char modifiers failed for: {}",
                input
            );
        } else if input.starts_with("tr") {
            let (search, replacement, modifiers) = extract_transliteration_parts(input);
            assert_eq!(search, expected_pattern, "Single char search failed for: {}", input);
            assert_eq!(
                replacement, expected_replacement,
                "Single char replacement failed for: {}",
                input
            );
            assert_eq!(
                modifiers, expected_modifiers,
                "Single char modifiers failed for: {}",
                input
            );
        }
    }

    // Test case 4: Empty content boundaries
    let (pattern, replacement, modifiers) = extract_substitution_parts("s///g");
    assert_eq!(pattern, "", "Empty pattern boundary failed - kills boundary condition mutations");
    assert_eq!(replacement, "", "Empty replacement boundary failed - validates boundary handling");
    assert_eq!(modifiers, "g", "Modifiers should be 'g' for empty substitution");
}
