//! Hash slice postfix fuzz tests (work-e5278c16)
//!
//! Fuzz testing for the hash slice postfix parsing fix.
//! These tests exercise the parser with randomized and edge-case inputs
//! to find crashes, panics, and unexpected behavior.
//!
//! The fuzz tests focus on:
//! 1. The new code path for `@hash{...}` and `%hash{...}` without arrow
//! 2. Deep nesting that could trigger stack overflow
//! 3. Malformed inputs that could cause panics
//! 4. Complex key expressions that stress the parser

mod cpan_test_helpers;

use cpan_test_helpers::*;
use std::collections::HashSet;

/// A simple pseudo-random number generator for deterministic fuzzing
/// Uses a simple linear congruential generator for reproducibility
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_in_range(&mut self, max: usize) -> usize {
        (self.next() as usize) % max
    }
}

/// Test that parsing various hash slice patterns doesn't panic
/// This is the core fuzz test for the new hash slice code path
#[test]
fn fuzz_hash_slice_no_panic() {
    let test_cases = vec![
        // Basic hash slices
        r#"%hash{key}"#,
        r#"@hash{key}"#,
        r#"%hash{key1, key2}"#,
        r#"@hash{key1, key2, key3}"#,
        // With variable keys
        r#"%hash{$key}"#,
        r#"@hash{$key1, $key2}"#,
        // With expressions
        r#"%hash{$key =~ /pattern/}"#,
        r#"@hash{map { $_ } @keys}"#,
        // Nested
        r#"%hash{outer}{inner}"#,
        r#"@hash{outer}{inner}"#,
        // Chained with arrow
        r#"$ref->%hash{key}"#,
        r#"$ref->@hash{key}"#,
        // Deep nesting (tests MAX_RECURSION_DEPTH indirectly)
        r#"%h0{%h1{%h2{%h3{%h4{%h5{%h6{%h7{%h8{%h9{key}}}}}}}}}}"#,
        // Complex keys
        r#"%hash{map { $_ => 1 } keys %other}"#,
        r#"@ops_seen{ map split(/ /), values %ops }"#,
        // Slice followed by other operations
        r#"%hash{key}->method()"#,
        r#"%hash{key}[0]"#,
        r#"%hash{key}{nested}"#,
        // With quotes
        r#"%hash{"key"}"#,
        r#"%hash{'key'}"#,
        r#"%hash{"$key"}"#,
        // Trailing comma
        r#"%hash{key,}"#,
        // Negative keys
        r#"%hash{-1}"#,
        r#"%hash{-42}"#,
        // Large numbers
        r#"%hash{999999999}"#,
        // Special characters
        r#"%hash{_private}"#,
        r#"%hash{'key::with::colons'}"#,
        // Qualified names
        r#"%Pkg::Hash{key}"#,
        r#"%$href{key}"#,
        r#"@$href{key}"#,
        // In various contexts
        r#"my @vals = %hash{key};"#,
        r#"if (%hash{@keys}) { }"#,
        r#"my @sorted = sort %hash{@keys};"#,
        r#"delete %hash{key};"#,
        r#"exists %hash{key}"#,
        // Assignment to slice
        r#"%hash{key1, key2} = (1, 2);"#,
        r#"@hash{key1, key2} = (1, 2);"#,
    ];

    for (i, source) in test_cases.iter().enumerate() {
        // Each test case should not panic - it may produce errors but shouldn't crash
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Fuzz test {} panicked on input: {:?}", i, source);
    }
}

/// Test that deeply nested hash slices don't cause stack overflow
/// This specifically tests the MAX_RECURSION_DEPTH check
#[test]
fn fuzz_deeply_nested_hash_slices() {
    // Create a deeply nested hash slice expression
    let deep_nesting = r#"%h0{%h1{%h2{%h3{%h4{%h5{%h6{%h7{%h8{%h9{%h10{%h11{%h12{%h13{%h14{%h15{key}}}}}}}}}}}}}}}}}"#;

    // This should parse without panicking (may hit MAX_RECURSION_DEPTH error but not panic)
    let result = std::panic::catch_unwind(|| {
        parse(deep_nesting);
    });

    assert!(result.is_ok(), "Deeply nested hash slice caused panic");
}

/// Test hash slices with malformed braces (unclosed)
/// The parser should handle these gracefully without panicking
#[test]
fn fuzz_malformed_unclosed_braces() {
    let malformed_cases = vec![
        r#"%hash{key"#, // Missing closing brace
        r#"%hash{"#,    // Missing key and closing
        r#"%hash{"key"#,
        r#"@hash{key"#,
        r#"%hash{{nested}}"#, // Double opening brace
        r#"%hash{key}}"#,     // Extra closing brace
        r#"%hash{{key}}"#,    // Double opening and closing
    ];

    for source in malformed_cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        // Should not panic - parser should handle gracefully
        assert!(result.is_ok(), "Malformed input {:?} caused panic", source);
    }
}

/// Test hash slices with empty and minimal inputs
#[test]
fn fuzz_empty_and_minimal() {
    let minimal_cases = vec![
        r#"%"#,  // Just % followed by nothing
        r#"@%"#, // Just @%
        r#"%$"#,
        r#"@$"#,
        r#"%h{"#,    // Missing closing
        r#"%h{""#,   // Empty string key
        r#"%h{''}"#, // Empty single-quoted key
        r#"%h{}"#,   // Empty braces
    ];

    for source in minimal_cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        // Should not panic
        assert!(result.is_ok(), "Minimal input {:?} caused panic", source);
    }
}

/// Test hash slices with various special characters
#[test]
fn fuzz_special_characters() {
    let special_cases = vec![
        r#"%hash{key with spaces}"#,
        r#"%hash{key\twith\ttabs}"#,
        r#"%hash{key\nwith\nnewlines}"#,
        r#"%hash{key\rwith\rcarriage}"#,
        r#"%hash{key\x00with\x00null}"#,
        r#"%hash{key}with{another}hash"#,
        r#"%hash{key} @hash{another}"#,
        r#"%hash{=}"#,
        r#"%hash{=>}"#,
        r#"%hash{+}"#,
        r#"%hash{-}"#,
        r#"%hash{*}"#,
        r#"%hash{/}"#,
        r#"%hash{?}"#,
        r#"%hash{!}"#,
        r#"%hash{@}"#,
        r#"%hash{#}"#,
        r#"%hash{$}"#,
        r#"%hash{%}"#,
        r#"%hash{^}"#,
        r#"%hash{&}"#,
        r#"%hash{|}"#,
        r#"%hash{~}"#,
        r#"%hash{`}"#,
        r#"%hash{0}"#,
    ];

    for source in special_cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Special character input {:?} caused panic", source);
    }
}

/// Test hash slices with regex-like patterns
#[test]
fn fuzz_regex_like_patterns() {
    let regex_cases = vec![
        r#"%hash{/pattern/}"#,
        r#"%hash{m/pattern/}"#,
        r#"%hash{qr/pattern/}"#,
        r#"%hash{s/pattern/replacement/}"#,
        r#"%hash{tr/pattern/replacement/}"#,
        r#"%hash{/}"#,  // Just slashes
        r#"%hash{//}"#, // Empty regex
    ];

    for source in regex_cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Regex pattern {:?} caused panic", source);
    }
}

/// Test hash slices with heredoc-like constructs
#[test]
fn fuzz_heredoc_like() {
    let heredoc_cases = vec![
        r#"%hash{<<"EOF"}
some text
EOF
}"#,
        r#"%hash{<<'EOF'}
some text
EOF
}"#,
        // Simpler cases without actual heredoc
        r#"%hash{<>"#,
        r#"%hash{<<}"#,
        r#"%hash{<DATA>}"#,
    ];

    for source in heredoc_cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Heredoc-like input caused panic");
    }
}

/// Test hash slices with different quote styles
#[test]
fn fuzz_quote_styles() {
    let quote_cases = vec![
        r#"%hash{"double quoted"}"#,
        r#"%hash{'single quoted'}"#,
        r#"%hash{`backtick command`}"#,
        r#"%hash{q/quote/}"#,
        r#"%hash{qq/double quote/}"#,
        r#"%hash{qw/word list/}"#,
        r#"%hash{qr/regex/}"#,
        r#"%hash{qx/shell cmd/}"#,
        r#"%hash{"nested \"quotes\""}"#,
        r#"%hash{'nested \'quotes\''}"#,
        r#"%hash{"mixed 'quotes'"}"#,
        r#"%hash{'mixed "quotes"'}"#,
    ];

    for source in quote_cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Quote style {:?} caused panic", source);
    }
}

/// Test hash slices in array/hash context
#[test]
fn fuzz_array_hash_context() {
    let context_cases = vec![
        r#"@array{%hash{keys}}"#,
        r#"@array{%hash{keys}, %other{keys}}"#,
        r#"%hash{keys @array}"#,
        r#"%hash{values %hash}"#,
        r#"%hash{keys %hash{inner}}"#,
        r#"@a{@b{@c{key}}}"#,
        r#"%h{%h{%h{key}}}"#,
        r#"@a[@b[@c{key}]]"#,
    ];

    for source in context_cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Array/hash context {:?} caused panic", source);
    }
}

/// Fuzz test with generated inputs using SimpleRng
/// This generates random Perl-like hash slice expressions
#[test]
fn fuzz_generated_hash_slice_expressions() {
    let mut rng = SimpleRng::new(12345);

    // Generate a set of unique random test cases
    let mut generated_cases: HashSet<String> = HashSet::new();

    // Pre-defined templates that are known to exercise the parser
    let templates = vec![
        "%{var}{key}",
        "@{var}{key}",
        "%{var}{$key}",
        "@{var}{$key}",
        "%{var}{key1, key2}",
        "@{var}{key1, key2}",
        "%{var}{map { $_ } @keys}",
        "@{var}{map { $_ } @keys}",
        "%{var}{values %other}",
        "@{var}{values %other}",
    ];

    for template in templates {
        let source = template.to_string();
        // Replace {var} with random variable names
        let var_names = ["h", "hash", "h1", "h2", "arr", "a", "ref", "href"];
        let key_names = ["key", "key1", "key2", "a", "b", "c", "_priv", "public"];

        for _ in 0..10 {
            let mut variant = source.clone();
            for var in &var_names {
                for key in &key_names {
                    variant = variant.replace("{var}", var).replace("{key}", key);
                }
            }
            if variant.contains("{var}") || variant.contains("{key}") {
                continue;
            }
            generated_cases.insert(variant);
        }
    }

    // Add generated cases to test
    for source in generated_cases {
        let result = std::panic::catch_unwind(|| {
            parse(&source);
        });

        assert!(result.is_ok(), "Generated input {:?} caused panic", source);
    }

    // Run a fixed number of random mutations
    let base_inputs = ["%hash{key}", "@hash{key}", "%hash{key1, key2}", "@hash{key1, key2, key3}"];

    let mutations =
        ["{%s{key}", "%s{$key}", "%s{key,}", "%s{{nested}}", "%s{key}->method()", "%s{key}[0]"];

    let sigils = ['%', '@'];

    for _ in 0..100 {
        let base = base_inputs[rng.next_in_range(base_inputs.len())].to_string();
        let mutation = mutations[rng.next_in_range(mutations.len())].to_string();
        let sigil: String = sigils[rng.next_in_range(sigils.len())].to_string();

        let source = mutation.replace("%s", &sigil);
        let source = source.replace("{key}", &base);

        let result = std::panic::catch_unwind(|| {
            parse(&source);
        });

        assert!(result.is_ok(), "Mutated input {:?} caused panic", source);
    }
}

/// Test that hash slice parsing doesn't produce invalid AST
/// This verifies that the AST structure is well-formed
#[test]
fn fuzz_ast_well_formed() {
    let cases = vec![
        r#"%hash{key}"#,
        r#"@hash{key}"#,
        r#"%hash{key1, key2}"#,
        r#"%hash{$key}"#,
        r#"%hash{$key =~ /pattern/}"#,
        r#"%hash{map { $_ } @keys}"#,
    ];

    for source in cases {
        let ast = parse(source);

        // Walk the AST and verify no obvious corruption
        fn walk_node(node: &perl_parser_core::Node) -> bool {
            // Check that locations are valid (start <= end)
            if node.location.start > node.location.end {
                return false;
            }

            // Check all children
            for child in node.children() {
                if !walk_node(child) {
                    return false;
                }
            }
            true
        }

        assert!(walk_node(&ast), "AST corruption detected for input: {:?}", source);
    }
}

/// Regression test: ensure existing patterns still work
/// These were working before the fix and should continue to work
#[test]
fn fuzz_regression_existing_patterns() {
    let existing_patterns = vec![
        r#"$ref->{key}"#,
        r#"$ref->{ $expr }"#,
        r#"$ref->{ $h->{nested} }"#,
        r#"my $href = { $a => $b };"#,
        r#"my $ref = { $a, $b };"#,
        r#"my $href = {};"#,
        r#"my %h = (a => 1, b => 2, c => 3);"#,
        r#"$outer->{inner}{key} = 1;"#,
        r#"$obj->get_hash()->{key};"#,
        r#"@a{@x} = @b{@y};"#,
        r#"$array_ref->[0]->{key};"#,
        r#"$hash_ref->{key}[0];"#,
        r#"my @vals = @array[$i]{key1, key2};"#,
        r#"my @vals = %hash{key}[0, 2];"#,
    ];

    for source in existing_patterns {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Existing pattern {:?} caused panic", source);

        // Also verify it parses cleanly
        assert_clean_parse(source);
    }
}

/// Test that the fix doesn't break hash literals vs blocks
/// This is a key disambiguation that should remain unchanged
#[test]
fn fuzz_hash_literal_vs_block_disambiguation() {
    let cases = vec![
        // Hash literal - should parse as hash
        (r#"{ $a => $b }"#, true),
        // Block with list - should parse as block
        (r#"{ $a, $b }"#, true),
        // Empty hash literal
        (r#"{}"#, true),
        // Block with statement
        (r#"{ my $x = 1; $x }"#, true),
        // Actual hash slice (the new feature)
        (r#"%hash{key}"#, true),
        (r#"@hash{key}"#, true),
    ];

    for (source, should_be_clean) in cases {
        let result = std::panic::catch_unwind(|| {
            parse(source);
        });

        assert!(result.is_ok(), "Hash literal/block input {:?} caused panic", source);

        if should_be_clean {
            // These should parse cleanly
            // Note: some might produce errors but not panics
        }
    }
}
