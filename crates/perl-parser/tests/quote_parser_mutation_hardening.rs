/// Mutation hardening tests for quote_parser.rs
/// These tests target specific edge cases and boundary conditions
/// that could be mutation survivors in quote parsing logic.
use perl_parser::quote_parser::{
    extract_regex_parts, extract_substitution_parts, extract_transliteration_parts,
};

/// Test edge cases in regex extraction that could be mutation survivors
#[test]
fn test_regex_extraction_edge_cases() {
    // Test empty patterns
    assert_eq!(extract_regex_parts(""), ("".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("qr"), ("".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("m"), ("mm".to_string(), "".to_string()));

    // Test single character patterns (includes delimiters in pattern)
    assert_eq!(extract_regex_parts("/a/"), ("/a/".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("qr/a/"), ("/a/".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("m/a/"), ("/a/".to_string(), "".to_string()));

    // Test patterns with just delimiters
    assert_eq!(extract_regex_parts("//"), ("//".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("qr//"), ("//".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("m()"), ("()".to_string(), "".to_string()));

    // Test malformed patterns that could cause edge cases
    assert_eq!(extract_regex_parts("/"), ("//".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("qr/"), ("//".to_string(), "".to_string()));
    assert_eq!(extract_regex_parts("m("), ("()".to_string(), "".to_string()));
}

/// Test m operator with non-alphabetic second character
#[test]
fn test_m_operator_second_char_validation() {
    // Valid m operators (non-alphabetic second char)
    let valid_cases = vec![
        ("m/pattern/", ("pattern", "")),
        ("m(pattern)", ("pattern", "")),
        ("m{pattern}", ("pattern", "")),
        ("m[pattern]", ("pattern", "")),
        ("m<pattern>", ("pattern", "")),
        ("m#pattern#", ("pattern", "")),
        ("m!pattern!", ("pattern", "")),
        ("m|pattern|", ("pattern", "")),
        ("m~pattern~", ("pattern", "")),
        ("m@pattern@", ("pattern", "")),
        ("m%pattern%", ("pattern", "")),
        ("m^pattern^", ("pattern", "")),
        ("m&pattern&", ("pattern", "")),
        ("m*pattern*", ("pattern", "")),
        ("m-pattern-", ("pattern", "")),
        ("m=pattern=", ("pattern", "")),
        ("m+pattern+", ("pattern", "")),
        ("m:pattern:", ("pattern", "")),
        ("m;pattern;", ("pattern", "")),
        ("m,pattern,", ("pattern", "")),
        ("m.pattern.", ("pattern", "")),
        ("m?pattern?", ("pattern", "")),
    ];

    for (input, (expected_pattern, expected_mods)) in valid_cases {
        let (pattern, mods) = extract_regex_parts(input);
        // Extract the actual pattern without delimiters for comparison
        let actual_pattern =
            if pattern.len() >= 2 { &pattern[1..pattern.len() - 1] } else { &pattern };
        assert_eq!(actual_pattern, expected_pattern, "Failed for input: {}", input);
        assert_eq!(mods, expected_mods, "Failed modifiers for input: {}", input);
    }

    // Invalid m operators (alphabetic second char) - these actually still get parsed with the closing delimiter
    let invalid_cases = vec![("ma", "mam"), ("mb", "mbm"), ("mc", "mcm")];

    for (input, expected_pattern) in invalid_cases {
        let (pattern, mods) = extract_regex_parts(input);
        assert_eq!(pattern, expected_pattern, "Pattern for invalid m operator: {}", input);
        assert_eq!(mods, "", "Should have no modifiers for invalid m operator: {}", input);
    }
}

/// Test substitution parsing edge cases
#[test]
fn test_substitution_edge_cases() {
    // Empty substitutions
    assert_eq!(extract_substitution_parts("s"), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(extract_substitution_parts("s//"), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(
        extract_substitution_parts("s///"),
        ("".to_string(), "".to_string(), "".to_string())
    );

    // Single character patterns/replacements
    assert_eq!(
        extract_substitution_parts("s/a/b/"),
        ("a".to_string(), "b".to_string(), "".to_string())
    );
    assert_eq!(
        extract_substitution_parts("s{a}{b}"),
        ("a".to_string(), "b".to_string(), "".to_string())
    );

    // Patterns with escaped delimiters (escapes are preserved in the output)
    assert_eq!(
        extract_substitution_parts("s/a\\/b/c/"),
        ("a\\/b".to_string(), "c".to_string(), "".to_string())
    );
    assert_eq!(
        extract_substitution_parts("s{a\\}b}{c}"),
        ("a\\}b".to_string(), "c".to_string(), "".to_string())
    );

    // Malformed substitutions
    assert_eq!(extract_substitution_parts("s/"), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(extract_substitution_parts("s{"), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(
        extract_substitution_parts("s/pattern"),
        ("pattern".to_string(), "".to_string(), "".to_string())
    );
}

/// Test substitution with all valid modifiers
#[test]
fn test_substitution_modifiers() {
    let test_cases = vec![
        ("s/a/b/g", "g"),
        ("s/a/b/i", "i"),
        ("s/a/b/m", "m"),
        ("s/a/b/s", "s"),
        ("s/a/b/x", "x"),
        ("s/a/b/o", "o"),
        ("s/a/b/e", "e"),
        ("s/a/b/r", "r"),
        ("s/a/b/gim", "gim"),
        ("s/a/b/gims", "gims"),
        ("s/a/b/gimsx", "gimsx"),
        ("s/a/b/gimsxoer", "gimsxoer"),
        // Invalid modifiers should be filtered out
        ("s/a/b/giz", "gi"),
        ("s/a/b/123", ""),
        ("s/a/b/g1i2m3", "g"), // Numbers stop the parsing
    ];

    for (input, expected_mods) in test_cases {
        let (_, _, mods) = extract_substitution_parts(input);
        assert_eq!(mods, expected_mods, "Failed modifiers for input: {}", input);
    }
}

/// Test paired delimiter handling in substitutions
#[test]
fn test_paired_delimiter_substitutions() {
    let paired_cases = vec![
        // Basic paired delimiters
        ("s(old)(new)", ("old", "new", "")),
        ("s{old}{new}", ("old", "new", "")),
        ("s[old][new]", ("old", "new", "")),
        ("s<old><new>", ("old", "new", "")),
        // Nested delimiters
        ("s{o(l)d}{n(e)w}", ("o(l)d", "n(e)w", "")),
        ("s(o{l}d)(n{e}w)", ("o{l}d", "n{e}w", "")),
        ("s[o(l)d][n(e)w]", ("o(l)d", "n(e)w", "")),
        ("s<o{l}d><n{e}w>", ("o{l}d", "n{e}w", "")),
        // With modifiers
        ("s{old}{new}g", ("old", "new", "g")),
        ("s(old)(new)gi", ("old", "new", "gi")),
        // Empty parts
        ("s{}{}", ("", "", "")),
        ("s(){}", ("", "", "")),
        ("s{}()", ("", "", "")),
        // Missing second delimiter (should result in empty replacement)
        ("s{pattern}", ("pattern", "", "")),
        ("s(pattern)", ("pattern", "", "")),
    ];

    for (input, (expected_pattern, expected_replacement, expected_mods)) in paired_cases {
        let (pattern, replacement, mods) = extract_substitution_parts(input);
        assert_eq!(pattern, expected_pattern, "Pattern failed for input: {}", input);
        assert_eq!(replacement, expected_replacement, "Replacement failed for input: {}", input);
        assert_eq!(mods, expected_mods, "Modifiers failed for input: {}", input);
    }
}

/// Test transliteration parsing (previously broken with "xyzzy" stub)
#[test]
fn test_transliteration_parsing() {
    // Basic tr operations
    assert_eq!(
        extract_transliteration_parts("tr/abc/xyz/"),
        ("abc".to_string(), "xyz".to_string(), "".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("y/abc/xyz/"),
        ("abc".to_string(), "xyz".to_string(), "".to_string())
    );

    // With modifiers
    assert_eq!(
        extract_transliteration_parts("tr/abc/xyz/d"),
        ("abc".to_string(), "xyz".to_string(), "d".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("tr/abc/xyz/cds"),
        ("abc".to_string(), "xyz".to_string(), "cds".to_string())
    );

    // Paired delimiters
    assert_eq!(
        extract_transliteration_parts("tr{abc}{xyz}"),
        ("abc".to_string(), "xyz".to_string(), "".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("y(abc)(xyz)"),
        ("abc".to_string(), "xyz".to_string(), "".to_string())
    );

    // Edge cases
    assert_eq!(
        extract_transliteration_parts("tr"),
        ("".to_string(), "".to_string(), "".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("y"),
        ("".to_string(), "".to_string(), "".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("tr//"),
        ("".to_string(), "".to_string(), "".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("tr/abc/"),
        ("abc".to_string(), "".to_string(), "".to_string())
    );

    // Invalid modifiers should be filtered out
    assert_eq!(
        extract_transliteration_parts("tr/abc/xyz/cdsx"),
        ("abc".to_string(), "xyz".to_string(), "cds".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("tr/abc/xyz/123"),
        ("abc".to_string(), "xyz".to_string(), "".to_string())
    );
}

/// Test valid transliteration modifiers
#[test]
fn test_transliteration_modifiers() {
    let modifier_cases = vec![
        ("tr/a/b/c", "c"),
        ("tr/a/b/d", "d"),
        ("tr/a/b/s", "s"),
        ("tr/a/b/r", "r"),
        ("tr/a/b/cd", "cd"),
        ("tr/a/b/cds", "cds"),
        ("tr/a/b/cdsr", "cdsr"),
        // Invalid modifiers filtered out
        ("tr/a/b/cgdi", "cd"),
        ("tr/a/b/xyz", ""),
        ("tr/a/b/123", ""),
        ("tr/a/b/c1d2s3", "c"), // Numbers stop parsing
    ];

    for (input, expected_mods) in modifier_cases {
        let (_, _, mods) = extract_transliteration_parts(input);
        assert_eq!(mods, expected_mods, "Failed modifiers for input: {}", input);
    }
}

/// Test escaped characters in all quote types
#[test]
fn test_escaped_characters() {
    // Regex escapes
    let (pattern, _) = extract_regex_parts("qr/\\//");
    assert!(pattern.contains("/"), "Should preserve escaped slash in regex");

    let (pattern, _) = extract_regex_parts("qr{\\}}");
    assert!(pattern.contains("}"), "Should preserve escaped brace in regex");

    // Substitution escapes (escapes are preserved)
    let (pattern, replacement, _) = extract_substitution_parts("s/\\/path/\\/new/");
    assert_eq!(pattern, "\\/path", "Should preserve escaped slashes in substitution pattern");
    assert_eq!(
        replacement, "\\/new",
        "Should preserve escaped slashes in substitution replacement"
    );

    let (pattern, replacement, _) = extract_substitution_parts("s{\\}old\\{}{\\}new\\{}");
    assert_eq!(pattern, "\\}old\\{", "Should preserve escaped braces in substitution pattern");
    assert_eq!(
        replacement, "\\}new\\{",
        "Should preserve escaped braces in substitution replacement"
    );

    // Transliteration escapes (escapes are preserved)
    let (search, replace, _) = extract_transliteration_parts("tr/\\//\\\\/");
    assert_eq!(search, "\\/", "Should preserve escaped slash in transliteration search");
    assert_eq!(replace, "\\\\", "Should preserve escaped backslash in transliteration replace");
}

/// Test complex nested delimiter scenarios
#[test]
fn test_complex_nested_delimiters() {
    // Deep nesting
    let (pattern, replacement, _) = extract_substitution_parts("s{(({test}))}{{new}}");
    assert_eq!(pattern, "(({test}))", "Should handle deeply nested delimiters in pattern");
    assert_eq!(replacement, "{new}", "Should handle nested delimiters in replacement");

    // Mixed nested delimiters
    let (pattern, replacement, _) = extract_substitution_parts("s{[(<test>)]}{[(<new>)]}");
    assert_eq!(pattern, "[(<test>)]", "Should handle mixed nested delimiters");
    assert_eq!(replacement, "[(<new>)]", "Should handle mixed nested delimiters in replacement");

    // Unbalanced delimiters (should still parse to closing)
    let (pattern, replacement, _) = extract_substitution_parts("s{(test}{new)}");
    assert_eq!(pattern, "(test", "Should parse until matching closing delimiter");
    assert_eq!(replacement, "new)", "Should handle unbalanced nested delimiters");
}

/// Test whitespace handling in paired delimiters
#[test]
fn test_whitespace_in_paired_delimiters() {
    // No whitespace
    let (pattern, replacement, _) = extract_substitution_parts("s{old}{new}");
    assert_eq!(pattern, "old");
    assert_eq!(replacement, "new");

    // Whitespace between delimiters
    let (pattern, replacement, _) = extract_substitution_parts("s{old} {new}");
    assert_eq!(pattern, "old");
    assert_eq!(replacement, "new");

    let (pattern, replacement, _) = extract_substitution_parts("s{old}  {new}");
    assert_eq!(pattern, "old");
    assert_eq!(replacement, "new");

    let (pattern, replacement, _) = extract_substitution_parts("s{old}\t{new}");
    assert_eq!(pattern, "old");
    assert_eq!(replacement, "new");

    let (pattern, replacement, _) = extract_substitution_parts("s{old}\n{new}");
    assert_eq!(pattern, "old");
    assert_eq!(replacement, "new");

    // Whitespace with no second delimiter
    let (pattern, replacement, _) = extract_substitution_parts("s{old} ");
    assert_eq!(pattern, "old");
    assert_eq!(replacement, "");
}

/// Test boundary conditions that could trigger mutations
#[test]
fn test_boundary_conditions() {
    // Single character inputs
    assert_eq!(extract_regex_parts("/"), ("//".to_string(), "".to_string()));
    assert_eq!(extract_substitution_parts("s"), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(
        extract_transliteration_parts("t"),
        ("".to_string(), "".to_string(), "".to_string())
    );

    // Two character inputs
    assert_eq!(extract_regex_parts("//"), ("//".to_string(), "".to_string()));
    assert_eq!(extract_substitution_parts("s/"), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(
        extract_transliteration_parts("tr"),
        ("".to_string(), "".to_string(), "".to_string())
    );

    // Maximum reasonable patterns (should not crash)
    let long_pattern = "a".repeat(1000);
    let long_input = format!("s/{}/{}/g", long_pattern, long_pattern);
    let (pattern, replacement, mods) = extract_substitution_parts(&long_input);
    assert_eq!(pattern, long_pattern);
    assert_eq!(replacement, long_pattern);
    assert_eq!(mods, "g");
}

/// Test Unicode in quote operators
#[test]
fn test_unicode_in_quotes() {
    // Unicode patterns
    let (pattern, _) = extract_regex_parts("qr/测试/");
    assert_eq!(pattern, "/测试/");

    let (pattern, replacement, _) = extract_substitution_parts("s/café/naïve/");
    assert_eq!(pattern, "café");
    assert_eq!(replacement, "naïve");

    let (search, replace, _) = extract_transliteration_parts("tr/αβγ/abc/");
    assert_eq!(search, "αβγ");
    assert_eq!(replace, "abc");

    // Unicode delimiters (exotic but possible)
    let (pattern, _) = extract_regex_parts("qr«pattern»");
    assert!(pattern.contains("pattern"), "Should handle Unicode delimiters");

    // Emoji patterns
    let (pattern, replacement, _) = extract_substitution_parts("s/😀/😁/");
    assert_eq!(pattern, "😀");
    assert_eq!(replacement, "😁");
}
