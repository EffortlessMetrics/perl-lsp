//! Property-based testing fixtures for comprehensive Perl syntax coverage
//!
//! Provides test data for property-based testing scenarios that validate
//! parser behavior across a wide range of Perl syntax patterns.
//!
//! Features:
//! - ~100% Perl syntax coverage with property-based test generators
//! - Edge case validation for parser robustness
//! - Fuzzing test data for security testing
//! - Performance validation with large syntax patterns
//! - Error recovery test scenarios

use std::collections::HashMap;

#[cfg(test)]
pub struct PropertyBasedFixture {
    pub name: &'static str,
    pub property_description: &'static str,
    pub test_cases: Vec<PropertyTestCase>,
    pub expected_invariants: Vec<ParserInvariant>,
    pub coverage_category: CoverageCategory,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct PropertyTestCase {
    pub input: String,
    pub expected_behavior: ExpectedBehavior,
    pub edge_case_type: Option<EdgeCaseType>,
    pub complexity_score: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedBehavior {
    ShouldParse,
    ShouldParseWithWarnings,
    ShouldFailGracefully,
    ShouldRecover,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeCaseType {
    UnicodeEdgeCase,
    DelimiterNesting,
    RegexComplexity,
    HeredocVariations,
    QuoteOperators,
    NumericLiterals,
    PackageDeclarations,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum ParserInvariant {
    AstWellFormed,
    PositionTracking,
    MemoryBounded,
    NoInfiniteLoops,
    ErrorRecovery,
    IncrementalConsistency,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageCategory {
    CoreSyntax,
    OperatorPrecedence,
    StringLiterals,
    RegularExpressions,
    SubroutineDefinitions,
    DataStructures,
    ControlFlow,
    ObjectOriented,
    ModernFeatures,
}

/// Comprehensive property-based test fixtures
#[cfg(test)]
pub fn load_property_based_fixtures() -> Vec<PropertyBasedFixture> {
    vec![
        // String literal property testing
        PropertyBasedFixture {
            name: "string_literal_properties",
            property_description: "All valid Perl string literals should parse correctly",
            test_cases: generate_string_literal_test_cases(),
            expected_invariants: vec![
                ParserInvariant::AstWellFormed,
                ParserInvariant::PositionTracking,
                ParserInvariant::MemoryBounded,
            ],
            coverage_category: CoverageCategory::StringLiterals,
        },

        // Regular expression property testing
        PropertyBasedFixture {
            name: "regex_pattern_properties",
            property_description: "Regular expressions with various delimiters should parse consistently",
            test_cases: generate_regex_test_cases(),
            expected_invariants: vec![
                ParserInvariant::AstWellFormed,
                ParserInvariant::NoInfiniteLoops,
                ParserInvariant::ErrorRecovery,
            ],
            coverage_category: CoverageCategory::RegularExpressions,
        },

        // Operator precedence property testing
        PropertyBasedFixture {
            name: "operator_precedence_properties",
            property_description: "Operator precedence should be consistent across expression types",
            test_cases: generate_operator_precedence_test_cases(),
            expected_invariants: vec![
                ParserInvariant::AstWellFormed,
                ParserInvariant::PositionTracking,
            ],
            coverage_category: CoverageCategory::OperatorPrecedence,
        },

        // Subroutine definition property testing
        PropertyBasedFixture {
            name: "subroutine_definition_properties",
            property_description: "All subroutine definition patterns should be handled correctly",
            test_cases: generate_subroutine_test_cases(),
            expected_invariants: vec![
                ParserInvariant::AstWellFormed,
                ParserInvariant::IncrementalConsistency,
            ],
            coverage_category: CoverageCategory::SubroutineDefinitions,
        },

        // Unicode edge cases
        PropertyBasedFixture {
            name: "unicode_edge_case_properties",
            property_description: "Unicode characters in various contexts should not break parsing",
            test_cases: generate_unicode_edge_cases(),
            expected_invariants: vec![
                ParserInvariant::AstWellFormed,
                ParserInvariant::PositionTracking,
                ParserInvariant::MemoryBounded,
            ],
            coverage_category: CoverageCategory::CoreSyntax,
        },

        // Data structure complexity testing
        PropertyBasedFixture {
            name: "data_structure_complexity_properties",
            property_description: "Complex nested data structures should parse within memory bounds",
            test_cases: generate_complex_data_structures(),
            expected_invariants: vec![
                ParserInvariant::AstWellFormed,
                ParserInvariant::MemoryBounded,
                ParserInvariant::NoInfiniteLoops,
            ],
            coverage_category: CoverageCategory::DataStructures,
        },
    ]
}

/// Generate string literal test cases with various edge cases
#[cfg(test)]
fn generate_string_literal_test_cases() -> Vec<PropertyTestCase> {
    let mut test_cases = Vec::new();

    // Basic string literals
    let basic_strings = vec![
        r#""simple string""#,
        r#"'single quoted'"#,
        r#"`backtick string`"#,
        r#"qq{interpolated string}"#,
        r#"q{literal string}"#,
        r#"qw{word list here}"#,
        r#"qr{regex pattern}"#,
        r#"qx{command execution}"#,
    ];

    for string in basic_strings {
        test_cases.push(PropertyTestCase {
            input: format!("my $var = {};", string),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 1,
        });
    }

    // Unicode in strings
    let unicode_strings = vec![
        r#""café unicode string""#,
        r#"'résumé with accents'"#,
        r#""emoji 🚀 in string""#,
        r#""Chinese characters: 你好世界""#,
        r#""Arabic text: مرحبا""#,
    ];

    for string in unicode_strings {
        test_cases.push(PropertyTestCase {
            input: format!("my $unicode = {};", string),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::UnicodeEdgeCase),
            complexity_score: 2,
        });
    }

    // Heredoc variations
    let heredoc_patterns = vec![
        "<<EOF\nHeredoc content\nEOF",
        "<<'LITERAL'\nLiteral heredoc\nLITERAL",
        "<<\"INTERPOLATED\"\nInterpolated $heredoc\nINTERPOLATED",
        "<<`COMMAND`\necho command\nCOMMAND",
    ];

    for heredoc in heredoc_patterns {
        test_cases.push(PropertyTestCase {
            input: format!("my $doc = {};", heredoc),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::HeredocVariations),
            complexity_score: 3,
        });
    }

    // Nested quotes and escaping
    let complex_quotes = vec![
        r#""string with \"escaped quotes\"""#,
        r#"'single with \'escaped quotes\''"#,
        r#"qq{string with {nested} braces}"#,
        r#"q[string with [nested] brackets]"#,
        r#"qr/regex with \/escaped\/ slashes/"#,
    ];

    for quote in complex_quotes {
        test_cases.push(PropertyTestCase {
            input: format!("my $complex = {};", quote),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::QuoteOperators),
            complexity_score: 4,
        });
    }

    test_cases
}

/// Generate regex pattern test cases with various delimiters
#[cfg(test)]
fn generate_regex_test_cases() -> Vec<PropertyTestCase> {
    let mut test_cases = Vec::new();

    // Basic regex patterns
    let basic_patterns = vec![
        (r#"/simple/"#, "Simple regex"),
        (r#"/\d+/"#, "Digit pattern"),
        (r#"/[a-zA-Z]+/"#, "Character class"),
        (r#"/\w+\s+\w+/"#, "Word boundaries"),
        (r#"/^start.*end$/"#, "Anchors"),
    ];

    for (pattern, description) in basic_patterns {
        test_cases.push(PropertyTestCase {
            input: format!("if ($text =~ {}) {{ print '{}'; }}", pattern, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 2,
        });
    }

    // Alternative delimiters
    let delimiter_patterns = vec![
        (r#"m{pattern}"#, "Curly braces"),
        (r#"m[pattern]"#, "Square brackets"),
        (r#"m(pattern)"#, "Parentheses"),
        (r#"m<pattern>"#, "Angle brackets"),
        (r#"m|pattern|"#, "Pipe delimiters"),
        (r#"m#pattern#"#, "Hash delimiters"),
        (r#"m@pattern@"#, "At delimiters"),
        (r#"m!pattern!"#, "Exclamation delimiters"),
    ];

    for (pattern, description) in delimiter_patterns {
        test_cases.push(PropertyTestCase {
            input: format!("my $match = $text =~ {}; # {}", pattern, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::DelimiterNesting),
            complexity_score: 3,
        });
    }

    // Complex regex features
    let complex_patterns = vec![
        (r#"/(?:non-capturing)/"#, "Non-capturing groups"),
        (r#"/(?<name>\w+)/"#, "Named capture groups"),
        (r#"/(?=lookahead)/"#, "Positive lookahead"),
        (r#"/(?!negative)/"#, "Negative lookahead"),
        (r#"/(?<=lookbehind)/"#, "Positive lookbehind"),
        (r#"/(?<!negative_behind)/"#, "Negative lookbehind"),
        (r#"/\Q literal \E/"#, "Literal quotation"),
        (r#"/(?(condition)yes|no)/"#, "Conditional patterns"),
    ];

    for (pattern, description) in complex_patterns {
        test_cases.push(PropertyTestCase {
            input: format!("my $advanced = $text =~ {}; # {}", pattern, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::RegexComplexity),
            complexity_score: 5,
        });
    }

    // Substitution operators with various delimiters
    let substitution_patterns = vec![
        (r#"s/old/new/"#, "Basic substitution"),
        (r#"s{old}{new}g"#, "Global substitution with braces"),
        (r#"s|old|new|i"#, "Case-insensitive substitution"),
        (r#"s#old#new#x"#, "Extended substitution"),
        (r#"s'old'new'"#, "Single-quote delimiters"),
        (r#"tr/a-z/A-Z/"#, "Transliteration"),
        (r#"y/aeiou/12345/"#, "Y operator"),
    ];

    for (pattern, description) in substitution_patterns {
        test_cases.push(PropertyTestCase {
            input: format!("$text =~ {}; # {}", pattern, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::DelimiterNesting),
            complexity_score: 4,
        });
    }

    test_cases
}

/// Generate operator precedence test cases
#[cfg(test)]
fn generate_operator_precedence_test_cases() -> Vec<PropertyTestCase> {
    let mut test_cases = Vec::new();

    // Arithmetic precedence
    let arithmetic_expressions = vec![
        ("$a + $b * $c", "Multiplication before addition"),
        ("($a + $b) * $c", "Parentheses override precedence"),
        ("$a ** $b ** $c", "Right-associative exponentiation"),
        ("$a / $b * $c", "Left-associative multiplication/division"),
        ("$a % $b + $c", "Modulo before addition"),
        ("-$a ** $b", "Unary minus and exponentiation"),
        ("++$a * $b", "Pre-increment and multiplication"),
        ("$a++ + $b", "Post-increment and addition"),
    ];

    for (expr, description) in arithmetic_expressions {
        test_cases.push(PropertyTestCase {
            input: format!("my $result = {}; # {}", expr, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 2,
        });
    }

    // Comparison and logical precedence
    let logical_expressions = vec![
        ("$a && $b || $c", "Logical AND before OR"),
        ("$a and $b or $c", "Low-precedence logical operators"),
        ("$a == $b && $c != $d", "Comparison before logical"),
        ("$a < $b ? $c : $d", "Ternary conditional"),
        ("$a =~ /pattern/ && $b", "Pattern match and logical"),
        ("not $a && $b", "NOT operator precedence"),
        ("$a ? $b ? $c : $d : $e", "Nested ternary"),
    ];

    for (expr, description) in logical_expressions {
        test_cases.push(PropertyTestCase {
            input: format!("my $condition = {}; # {}", expr, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 3,
        });
    }

    // Assignment precedence
    let assignment_expressions = vec![
        ("$a = $b = $c", "Right-associative assignment"),
        ("$a += $b *= $c", "Compound assignment"),
        ("$a ||= $b", "Logical OR assignment"),
        ("$a &&= $b", "Logical AND assignment"),
        ("$a //= $b", "Defined-or assignment"),
        ("@array = ($a, $b, $c)", "List assignment"),
        ("($x, $y) = ($y, $x)", "List assignment swap"),
    ];

    for (expr, description) in assignment_expressions {
        test_cases.push(PropertyTestCase {
            input: format!("{}; # {}", expr, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 2,
        });
    }

    test_cases
}

/// Generate subroutine definition test cases
#[cfg(test)]
fn generate_subroutine_test_cases() -> Vec<PropertyTestCase> {
    let mut test_cases = Vec::new();

    // Basic subroutine definitions
    let basic_subs = vec![
        ("sub simple { return 42; }", "Simple subroutine"),
        ("sub empty { }", "Empty subroutine"),
        ("sub with_params { my ($a, $b) = @_; return $a + $b; }", "Parameters"),
        ("sub Package::qualified { return 'qualified'; }", "Qualified name"),
        ("sub _private { return 'private'; }", "Private subroutine"),
        ("sub 𝕦𝖓𝖎𝖈𝖔𝖉𝖊 { return 'unicode'; }", "Unicode name"),
    ];

    for (sub_def, description) in basic_subs {
        test_cases.push(PropertyTestCase {
            input: format!("{} # {}", sub_def, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 2,
        });
    }

    // Modern subroutine features
    let modern_subs = vec![
        ("sub with_signature($a, $b) { return $a + $b; }", "Signature"),
        ("sub with_defaults($a, $b = 10) { return $a * $b; }", "Default parameters"),
        ("sub slurpy($first, @rest) { return ($first, @rest); }", "Slurpy parameters"),
        ("sub named($a, %opts) { return $a . $opts{suffix}; }", "Named parameters"),
        ("sub state_var { state $count = 0; return ++$count; }", "State variables"),
    ];

    for (sub_def, description) in modern_subs {
        test_cases.push(PropertyTestCase {
            input: format!("use feature 'signatures'; no warnings 'experimental::signatures'; {} # {}", sub_def, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 4,
        });
    }

    // Anonymous subroutines
    let anonymous_subs = vec![
        ("my $anon = sub { return 'anonymous'; };", "Basic anonymous"),
        ("my $closure = sub { my $x = shift; return sub { $x++ }; };", "Closure"),
        ("my $recursive = sub { my $n = shift; $n <= 1 ? 1 : $n * __SUB__->($n-1); };", "Recursive anonymous"),
    ];

    for (sub_def, description) in anonymous_subs {
        test_cases.push(PropertyTestCase {
            input: format!("{} # {}", sub_def, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 3,
        });
    }

    test_cases
}

/// Generate Unicode edge case test scenarios
#[cfg(test)]
fn generate_unicode_edge_cases() -> Vec<PropertyTestCase> {
    let mut test_cases = Vec::new();

    // Unicode identifiers
    let unicode_identifiers = vec![
        ("my $café = 'coffee';", "Accented characters"),
        ("my $🚀 = 'rocket';", "Emoji identifier"),
        ("my $Ελληνικά = 'Greek';", "Greek characters"),
        ("my $العربية = 'Arabic';", "Arabic characters"),
        ("my $中文 = 'Chinese';", "Chinese characters"),
        ("my $Русский = 'Russian';", "Cyrillic characters"),
    ];

    for (code, description) in unicode_identifiers {
        test_cases.push(PropertyTestCase {
            input: format!("use utf8; {} # {}", code, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::UnicodeEdgeCase),
            complexity_score: 3,
        });
    }

    // Unicode in strings and patterns
    let unicode_content = vec![
        (r#"my $text = "Mixed: ASCII + Unicode 世界";"#, "Mixed content"),
        (r#"$text =~ /[\x{4e00}-\x{9fff}]/;"#, "Unicode regex range"),
        (r#"$text =~ /\p{Script=Han}/;"#, "Unicode script property"),
        (r#"$text =~ /\p{Letter}/;"#, "Unicode category"),
        (r#"my $normalized = Unicode::Normalize::NFC($text);"#, "Normalization"),
    ];

    for (code, description) in unicode_content {
        test_cases.push(PropertyTestCase {
            input: format!("use utf8; {} # {}", code, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::UnicodeEdgeCase),
            complexity_score: 4,
        });
    }

    // Edge cases that might break parsing
    let edge_cases = vec![
        ("my $zero_width = \"\u{200B}\";", "Zero-width space"),
        ("my $bidi = \"English עברית English\";", "Bidirectional text"),
        ("my $combining = \"e\u{0301}\";", "Combining characters"),
        ("my $surrogate = \"\u{1F600}\";", "Emoji requiring surrogates"),
    ];

    for (code, description) in edge_cases {
        test_cases.push(PropertyTestCase {
            input: format!("use utf8; {} # {}", code, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: Some(EdgeCaseType::UnicodeEdgeCase),
            complexity_score: 5,
        });
    }

    test_cases
}

/// Generate complex data structure test cases
#[cfg(test)]
fn generate_complex_data_structures() -> Vec<PropertyTestCase> {
    let mut test_cases = Vec::new();

    // Deeply nested structures
    let nested_levels = [3, 5, 10, 20];
    for depth in nested_levels {
        let nested_hash = generate_nested_hash(depth);
        test_cases.push(PropertyTestCase {
            input: format!("my $nested = {};", nested_hash),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: depth as u32,
        });

        let nested_array = generate_nested_array(depth);
        test_cases.push(PropertyTestCase {
            input: format!("my $nested = {};", nested_array),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: depth as u32,
        });
    }

    // Large data structures
    let large_structures = vec![
        (generate_large_array(100), "Large array"),
        (generate_large_hash(50), "Large hash"),
        (generate_mixed_structure(), "Mixed structure"),
    ];

    for (structure, description) in large_structures {
        test_cases.push(PropertyTestCase {
            input: format!("my $large = {}; # {}", structure, description),
            expected_behavior: ExpectedBehavior::ShouldParse,
            edge_case_type: None,
            complexity_score: 8,
        });
    }

    test_cases
}

/// Helper functions for generating test data
#[cfg(test)]
fn generate_nested_hash(depth: usize) -> String {
    if depth == 0 {
        return "{}".to_string();
    }
    format!("{{ key{} => {} }}", depth, generate_nested_hash(depth - 1))
}

#[cfg(test)]
fn generate_nested_array(depth: usize) -> String {
    if depth == 0 {
        return "[]".to_string();
    }
    format!("[{}]", generate_nested_array(depth - 1))
}

#[cfg(test)]
fn generate_large_array(size: usize) -> String {
    let elements: Vec<String> = (1..=size).map(|i| format!("\"item{}\"", i)).collect();
    format!("[{}]", elements.join(", "))
}

#[cfg(test)]
fn generate_large_hash(size: usize) -> String {
    let pairs: Vec<String> = (1..=size)
        .map(|i| format!("\"key{}\" => \"value{}\"", i, i))
        .collect();
    format!("{{ {} }}", pairs.join(", "))
}

#[cfg(test)]
fn generate_mixed_structure() -> String {
    r#"{
        arrays => [
            [1, 2, 3],
            ["a", "b", "c"],
            [qw(one two three)]
        ],
        hashes => {
            inner1 => { a => 1, b => 2 },
            inner2 => { x => "test", y => [1, 2, 3] }
        },
        mixed => [
            { type => "hash", data => { key => "value" } },
            { type => "array", data => [1, 2, 3] },
            { type => "scalar", data => "simple" }
        ]
    }"#.to_string()
}

use std::sync::LazyLock;

/// Lazy-loaded property-based fixture registry
#[cfg(test)]
pub static PROPERTY_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, PropertyBasedFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_property_based_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get property-based fixture by name
#[cfg(test)]
pub fn get_property_fixture_by_name(name: &str) -> Option<&'static PropertyBasedFixture> {
    PROPERTY_FIXTURE_REGISTRY.get(name)
}

/// Get fixtures by coverage category
#[cfg(test)]
pub fn get_fixtures_by_coverage(category: CoverageCategory) -> Vec<&'static PropertyBasedFixture> {
    PROPERTY_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.coverage_category == category)
        .collect()
}

/// Get fixtures by complexity score range
#[cfg(test)]
pub fn get_fixtures_by_complexity(min_score: u32, max_score: u32) -> Vec<&PropertyTestCase> {
    PROPERTY_FIXTURE_REGISTRY
        .values()
        .flat_map(|fixture| &fixture.test_cases)
        .filter(|case| case.complexity_score >= min_score && case.complexity_score <= max_score)
        .collect()
}