//! Comprehensive substitution operator parsing test fixtures
//!
//! Complete test coverage for substitution operator parsing including all delimiter styles,
//! balanced delimiters, alternative delimiters, and complex patterns with comprehensive
//! edge case validation.
//!
//! Features:
//! - All delimiter styles: //, ||, ##, @@, %%, !!
//! - Balanced delimiters: {}, [], <>, ()
//! - Single-quote substitution delimiters: s'pattern'replacement'
//! - Complex regex patterns with escape sequences
//! - Transliteration operators (tr///, y///)
//! - Performance validation with parsing accuracy metrics

#[cfg(test)]
pub struct SubstitutionFixture {
    pub name: &'static str,
    pub perl_code: &'static str,
    pub expected_ast_nodes: usize,
    pub delimiter_type: DelimiterType,
    pub pattern_complexity: PatternComplexity,
    pub modifier_usage: ModifierUsage,
    pub parsing_accuracy: f32,
    pub performance_category: PerformanceCategory,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum DelimiterType {
    Standard,         // s/pattern/replacement/
    Balanced,         // s{pattern}{replacement}, s[pattern][replacement]
    Alternative,      // s|pattern|replacement|, s#pattern#replacement#
    SingleQuote,      // s'pattern'replacement'
    Mixed,           // Different delimiters in same context
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum PatternComplexity {
    Simple,          // Basic string patterns
    Regex,           // Regular expression patterns
    Complex,         // Complex regex with groups, lookahead, etc.
    Escaped,         // Patterns with escaped delimiters
    Unicode,         // Unicode patterns and replacements
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum ModifierUsage {
    None,            // No modifiers
    Global,          // g modifier
    CaseInsensitive, // i modifier
    Extended,        // x modifier
    Multiple,        // Multiple modifiers (gi, gx, etc.)
    Evaluation,      // e modifier (code evaluation)
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceCategory {
    Fast,            // < 50us parsing time
    Medium,          // 50-150us parsing time
    Complex,         // > 150us parsing time
}

/// Standard delimiter substitution fixtures
#[cfg(test)]
pub fn load_standard_delimiter_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "basic_standard_delimiters",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $text = "hello world test string";

# Basic substitution operations with standard delimiters
$text =~ s/hello/hi/g;
$text =~ s/world/universe/g;
$text =~ s/test/exam/g;

# Case-insensitive substitutions
$text =~ s/HELLO/greetings/gi;
$text =~ s/WORLD/cosmos/gi;

# Substitution with word boundaries
$text =~ s/\btest\b/quiz/g;
$text =~ s/\bstring\b/text/g;

# Multiple substitutions on same variable
$text =~ s/hello/hi/g;
$text =~ s/hi/greetings/g;

print "Standard delimiter substitutions: $text\n";
"#,
            expected_ast_nodes: 48,
            delimiter_type: DelimiterType::Standard,
            pattern_complexity: PatternComplexity::Simple,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 99.8,
            performance_category: PerformanceCategory::Fast,
        },

        SubstitutionFixture {
            name: "complex_regex_patterns",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $data = "user@example.com phone:123-456-7890 date:2023-12-25";

# Complex regex patterns with capture groups
$data =~ s/(\w+)@(\w+\.\w+)/Email: $1 at $2/g;
$data =~ s/phone:(\d{3})-(\d{3})-(\d{4})/Phone: ($1) $2-$3/g;
$data =~ s/date:(\d{4})-(\d{2})-(\d{2})/Date: $3\/$2\/$1/g;

# Lookahead and lookbehind assertions
$data =~ s/(?<=\w)@(?=\w)/[AT]/g;
$data =~ s/(?<!\d)-(?!\d)/ dash /g;

# Character classes and quantifiers
$data =~ s/\d{2,4}/[NUMBER]/g;
$data =~ s/[A-Z]{2,}/[UPPER]/g;

# Non-capturing groups and alternatives
$data =~ s/(?:phone|telephone|tel)/PHONE/gi;
$data =~ s/(?:email|e-mail|mail)/EMAIL/gi;

print "Complex regex processing: $data\n";
"#,
            expected_ast_nodes: 65,
            delimiter_type: DelimiterType::Standard,
            pattern_complexity: PatternComplexity::Complex,
            modifier_usage: ModifierUsage::Multiple,
            parsing_accuracy: 98.5,
            performance_category: PerformanceCategory::Medium,
        },
    ]
}

/// Balanced delimiter substitution fixtures
#[cfg(test)]
pub fn load_balanced_delimiter_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "comprehensive_balanced_delimiters",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $text = "test string with various patterns";

# Brace delimiters
$text =~ s{test}{exam}g;
$text =~ s{string}{text}g;

# Bracket delimiters
$text =~ s[with][containing]g;
$text =~ s[various][different]g;

# Angle bracket delimiters
$text =~ s<patterns><designs>g;
$text =~ s<exam><quiz>g;

# Parentheses delimiters
$text =~ s(containing)(having)g;
$text =~ s(different)(varied)g;

# Nested balanced delimiters with escaping
$text =~ s{quiz\}test}{exam\[result\]}g;
$text =~ s[having\[content\]][containing\{data\}]g;

# Mixed balanced delimiters in complex patterns
$text =~ s{(quiz|exam)\s+(\w+)}{Test: $1 Type: $2}g;
$text =~ s[(\w+)\{(\w+)\}][$1<$2>]g;

print "Balanced delimiter processing: $text\n";
"#,
            expected_ast_nodes: 72,
            delimiter_type: DelimiterType::Balanced,
            pattern_complexity: PatternComplexity::Escaped,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 98.2,
            performance_category: PerformanceCategory::Medium,
        },

        SubstitutionFixture {
            name: "nested_balanced_complexity",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $code = "function(arg1, arg2) { return value; }";

# Complex nested balanced delimiter parsing
$code =~ s{function\(([^)]+)\)\s*\{([^}]+)\}}{sub $1 { $2 }}g;
$code =~ s[(\w+)\(([^)]*)\)]{$1<$2>}g;
$code =~ s<return\s+(\w+);><yield $1;>g;

# Deeply nested structures
my $nested = "{{inner{deep}content}}";
$nested =~ s{\{(\{[^}]*\{[^}]*\}[^}]*\})\}}{[$1]}g;

# Mixed nesting with escape sequences
my $escaped = "test\\{content\\}more";
$escaped =~ s{test\\(\{[^}]*\\)\}}{result\\$1}g;

print "Nested balanced processing complete\n";
"#,
            expected_ast_nodes: 58,
            delimiter_type: DelimiterType::Balanced,
            pattern_complexity: PatternComplexity::Complex,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 97.8,
            performance_category: PerformanceCategory::Complex,
        },
    ]
}

/// Alternative delimiter substitution fixtures
#[cfg(test)]
pub fn load_alternative_delimiter_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "alternative_delimiter_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $text = "sample text for alternative delimiter testing";

# Pipe delimiters
$text =~ s|sample|example|g;
$text =~ s|text|content|g;

# Hash delimiters
$text =~ s#for#with#g;
$text =~ s#alternative#different#g;

# At symbol delimiters
$text =~ s@delimiter@separator@g;
$text =~ s@testing@validation@g;

# Percent delimiters
$text =~ s%example%specimen%g;
$text =~ s%content%material%g;

# Exclamation mark delimiters
$text =~ s!with!using!g;
$text =~ s!different!various!g;

# Mixed alternative delimiters in sequence
$text =~ s|separator|divider|g;
$text =~ s#validation#verification#g;
$text =~ s@specimen@sample@g;

print "Alternative delimiter processing: $text\n";
"#,
            expected_ast_nodes: 55,
            delimiter_type: DelimiterType::Alternative,
            pattern_complexity: PatternComplexity::Simple,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 99.5,
            performance_category: PerformanceCategory::Fast,
        },

        SubstitutionFixture {
            name: "alternative_with_escaping",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $path = "/usr/local/bin/perl";
my $url = "https://example.com/path?param=value";

# Hash delimiters with path-like strings (avoiding conflict)
$path =~ s#/usr/local#/opt#g;
$path =~ s#/bin#/sbin#g;

# Pipe delimiters with URL processing
$url =~ s|https://|http://|g;
$url =~ s|example\.com|test\.org|g;

# Exclamation delimiters with special characters
my $special = "test!important@critical#data";
$special =~ s!test\!important!very\!important!g;
$special =~ s!@critical!@vital!g;
$special =~ s!#data!#info!g;

# Escaped delimiters in patterns and replacements
my $escaped = "path#with#hashes";
$escaped =~ s#path\#with\#hashes#route\#containing\#symbols#g;

print "Alternative delimiter escaping: $escaped\n";
"#,
            expected_ast_nodes: 62,
            delimiter_type: DelimiterType::Alternative,
            pattern_complexity: PatternComplexity::Escaped,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 97.9,
            performance_category: PerformanceCategory::Medium,
        },
    ]
}

/// Single-quote delimiter substitution fixtures
#[cfg(test)]
pub fn load_single_quote_delimiter_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "single_quote_delimiters",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $text = "hello world with single quotes";

# Single-quote delimited substitutions
$text =~ s'hello'greetings'g;
$text =~ s'world'universe'g;
$text =~ s'with'containing'g;
$text =~ s'single'individual'g;
$text =~ s'quotes'marks'g;

# Single quotes with literal interpretation (no variable interpolation)
my $literal = "test \$variable content";
$literal =~ s'test'example'g;
$literal =~ s'\$variable'$replacement'g;

# Mixed single-quote and standard delimiters
$text =~ s'greetings'hello'g;
$text =~ s/universe/world/g;
$text =~ s'containing'with'g;

print "Single-quote delimiter processing: $text\n";
"#,
            expected_ast_nodes: 45,
            delimiter_type: DelimiterType::SingleQuote,
            pattern_complexity: PatternComplexity::Simple,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 99.2,
            performance_category: PerformanceCategory::Fast,
        },
    ]
}

/// Transliteration operator fixtures (tr///, y///)
#[cfg(test)]
pub fn load_transliteration_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "transliteration_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $text = "Hello World Testing";

# Basic transliteration operations
$text =~ tr/a-z/A-Z/;          # Uppercase conversion
my $lower = $text;
$lower =~ tr/A-Z/a-z/;         # Lowercase conversion

# Character class transliteration
my $vowels = "aeiou AEIOU";
$vowels =~ tr/aeiouAEIOU/12345/;

# Y operator (synonym for tr)
my $y_test = "abcdef";
$y_test =~ y/abcdef/123456/;

# Transliteration with deletion
my $delete_test = "abc123def456";
$delete_test =~ tr/0-9//d;     # Delete digits

# Transliteration with complement
my $complement = "Hello123World";
$complement =~ tr/A-Za-z//cd;  # Keep only letters

# Squeeze repeated characters
my $squeeze = "hello    world";
$squeeze =~ tr/ //s;           # Squeeze multiple spaces

# Complex transliteration with ranges
my $complex = "Test123Data456";
$complex =~ tr/0-9A-Z/a-z/;

print "Transliteration processing completed\n";
"#,
            expected_ast_nodes: 52,
            delimiter_type: DelimiterType::Standard,
            pattern_complexity: PatternComplexity::Simple,
            modifier_usage: ModifierUsage::Multiple,
            parsing_accuracy: 98.8,
            performance_category: PerformanceCategory::Fast,
        },
    ]
}

/// Unicode and international character fixtures
#[cfg(test)]
pub fn load_unicode_substitution_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "unicode_substitution_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use utf8;

# Unicode strings with substitution operations
my $unicode_text = "café naïve résumé 世界 🚀";

# Unicode character substitutions
$unicode_text =~ s/café/coffee/g;
$unicode_text =~ s/naïve/simple/g;
$unicode_text =~ s/résumé/cv/g;

# Unicode regex patterns
$unicode_text =~ s/世界/world/g;
$unicode_text =~ s/🚀/rocket/g;

# Unicode character classes
my $mixed = "Test123データ456世界";
$mixed =~ s/[\x{30A0}-\x{30FF}]/[KATAKANA]/g;  # Katakana
$mixed =~ s/[\x{4E00}-\x{9FFF}]/[CJK]/g;       # CJK characters

# Unicode normalization in substitutions
my $accented = "résumé naïve café";
$accented =~ s/é/e/g;
$accented =~ s/ï/i/g;

# Emoji and symbol substitutions
my $symbols = "test ☕ more 🎉 content";
$symbols =~ s/☕/coffee/g;
$symbols =~ s/🎉/party/g;

print "Unicode substitution completed\n";
"#,
            expected_ast_nodes: 68,
            delimiter_type: DelimiterType::Standard,
            pattern_complexity: PatternComplexity::Unicode,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 96.5,
            performance_category: PerformanceCategory::Medium,
        },
    ]
}

/// Code evaluation substitution fixtures (e modifier)
#[cfg(test)]
pub fn load_evaluation_substitution_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "code_evaluation_substitutions",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $text = "The price is 100 dollars and 50 cents";

# Code evaluation in replacement (e modifier)
$text =~ s/(\d+)/($1 * 1.1)/ge;  # Increase numbers by 10%

# Multiple evaluation substitutions
my $formula = "2 + 3 * 4";
$formula =~ s/(\d+\s*[+*]\s*\d+)/eval($1)/ge;

# Complex evaluation with functions
my $data = "length:hello width:world";
$data =~ s/length:(\w+)/length($1)/ge;
$data =~ s/width:(\w+)/length($1)/ge;

# Evaluation with captured groups
my $calculations = "sum(5,10) multiply(3,7)";
$calculations =~ s/sum\((\d+),(\d+)\)/($1 + $2)/ge;
$calculations =~ s/multiply\((\d+),(\d+)\)/($1 * $2)/ge;

print "Code evaluation substitutions completed\n";
"#,
            expected_ast_nodes: 58,
            delimiter_type: DelimiterType::Standard,
            pattern_complexity: PatternComplexity::Complex,
            modifier_usage: ModifierUsage::Evaluation,
            parsing_accuracy: 95.8,
            performance_category: PerformanceCategory::Complex,
        },
    ]
}

/// Performance stress test fixtures
#[cfg(test)]
pub fn load_performance_stress_fixtures() -> Vec<SubstitutionFixture> {
    vec![
        SubstitutionFixture {
            name: "substitution_performance_stress",
            perl_code: &format!(r#"#!/usr/bin/perl
use strict;
use warnings;

# Large text for performance testing
my $large_text = "{}";

{}

# Complex chained substitutions for performance validation
$large_text =~ s/test/exam/g;
$large_text =~ s|pattern|template|g;
$large_text =~ s#content#material#g;
$large_text =~ s{{data}}{{information}}g;

print "Performance stress test completed\n";
"#,
    "word test pattern content data ".repeat(200),
    generate_performance_substitutions(100)
),
            expected_ast_nodes: 350,
            delimiter_type: DelimiterType::Mixed,
            pattern_complexity: PatternComplexity::Simple,
            modifier_usage: ModifierUsage::Global,
            parsing_accuracy: 98.0,
            performance_category: PerformanceCategory::Complex,
        },
    ]
}

/// Generate performance test substitution operations
#[cfg(test)]
fn generate_performance_substitutions(count: usize) -> String {
    (0..count)
        .map(|i| {
            let delims = ["//", "||", "##", "@@", "%%"];
            let delim = delims[i % delims.len()];
            // delims are static 2-char strings, first/last char always available
            let first_char = delim.chars().next().unwrap_or('/');
            let last_char = delim.chars().last().unwrap_or('/');
            format!(
                "$large_text =~ s{}word{}term{}g;\n",
                first_char,
                first_char,
                last_char
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Load all comprehensive substitution fixtures
#[cfg(test)]
pub fn load_all_substitution_fixtures() -> Vec<SubstitutionFixture> {
    let mut all_fixtures = Vec::new();

    all_fixtures.extend(load_standard_delimiter_fixtures());
    all_fixtures.extend(load_balanced_delimiter_fixtures());
    all_fixtures.extend(load_alternative_delimiter_fixtures());
    all_fixtures.extend(load_single_quote_delimiter_fixtures());
    all_fixtures.extend(load_transliteration_fixtures());
    all_fixtures.extend(load_unicode_substitution_fixtures());
    all_fixtures.extend(load_evaluation_substitution_fixtures());
    all_fixtures.extend(load_performance_stress_fixtures());

    all_fixtures
}

/// Load fixtures by delimiter type
#[cfg(test)]
pub fn load_fixtures_by_delimiter_type(delimiter_type: DelimiterType) -> Vec<SubstitutionFixture> {
    load_all_substitution_fixtures()
        .into_iter()
        .filter(|fixture| fixture.delimiter_type == delimiter_type)
        .collect()
}

/// Load fixtures by pattern complexity
#[cfg(test)]
pub fn load_fixtures_by_complexity(complexity: PatternComplexity) -> Vec<SubstitutionFixture> {
    load_all_substitution_fixtures()
        .into_iter()
        .filter(|fixture| fixture.pattern_complexity == complexity)
        .collect()
}

/// Load high-accuracy fixtures (>98% parsing accuracy)
#[cfg(test)]
pub fn load_high_accuracy_fixtures() -> Vec<SubstitutionFixture> {
    load_all_substitution_fixtures()
        .into_iter()
        .filter(|fixture| fixture.parsing_accuracy > 98.0)
        .collect()
}

use std::sync::LazyLock;
use std::collections::HashMap;

/// Lazy-loaded substitution fixture registry
#[cfg(test)]
pub static SUBSTITUTION_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, SubstitutionFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_all_substitution_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get substitution fixture by name
#[cfg(test)]
pub fn get_substitution_fixture_by_name(name: &str) -> Option<&'static SubstitutionFixture> {
    SUBSTITUTION_FIXTURE_REGISTRY.get(name)
}