//! Comprehensive Perl syntax test fixtures for executeCommand testing
//!
//! Provides realistic Perl code samples covering ~100% of Perl 5 syntax patterns
//! for testing LSP executeCommand functionality, particularly perl.runCritic.
//!
//! Features:
//! - Enhanced builtin function parsing (map/grep/sort with {} blocks)
//! - Comprehensive substitution operator patterns (s/// with all delimiter styles)
//! - Dual indexing validation scenarios (Package::function + bare function)
//! - Unicode support with proper UTF-8/UTF-16 boundary testing
//! - Cross-file navigation test scenarios
//! - Error handling and edge case validation

#[cfg(test)]
pub struct PerlSyntaxFixture {
    pub name: &'static str,
    pub perl_code: &'static str,
    pub expected_violations: usize,
    pub expected_ast_nodes: usize,
    pub parsing_time_us: Option<u64>,
    pub syntax_category: SyntaxCategory,
    pub dual_indexing: bool,
    pub unicode_safe: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxCategory {
    BasicSyntax,
    BuiltinFunctions,
    SubstitutionOperators,
    PackageNavigation,
    UnicodeSupport,
    ErrorScenarios,
    PerformanceBenchmark,
}

/// Comprehensive Perl syntax test data with ~100% language coverage
#[cfg(test)]
pub fn load_comprehensive_syntax_fixtures() -> Vec<PerlSyntaxFixture> {
    vec![
        // Basic Perl syntax with policy violations for perl.runCritic
        PerlSyntaxFixture {
            name: "basic_policy_violations",
            perl_code: r#"#!/usr/bin/perl
# This file contains common Perl::Critic policy violations

my $variable = 42;
print "Value: $variable\n";

sub calculate {
    my ($a, $b) = @_;
    $a + $b;  # Missing explicit return
}

# File operation without 3-arg open
open FILE, "test.txt";
print FILE "Hello\n";
close FILE;

# C-style for loop
for (my $i = 0; $i < 10; $i++) {
    print "Index: $i\n";
}

# Postfix condition that could be improved
if ($variable > 0) {
    print "Positive";
}

my @array = (1, 2, 3, 4, 5);
my $element = $array[0];  # Could use first element access
"#,
            expected_violations: 8,
            expected_ast_nodes: 45,
            parsing_time_us: Some(150),
            syntax_category: SyntaxCategory::BasicSyntax,
            dual_indexing: false,
            unicode_safe: true,
        },

        // Enhanced builtin function parsing with {} blocks
        PerlSyntaxFixture {
            name: "enhanced_builtin_functions",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

# Enhanced map/grep/sort with {} blocks (deterministic parsing)
my @data = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);

# Map with complex {} block
my @squared = map { $_ * $_ } @data;

# Grep with condition {} block
my @evens = grep { $_ % 2 == 0 } @data;

# Sort with comparison {} block
my @sorted = sort { $a <=> $b } @data;

# Nested builtin functions with {} blocks
my @complex = map {
    my $val = $_;
    $val > 5 ? $val * 2 : $val
} grep {
    $_ % 2 == 1
} sort {
    $b <=> $a
} @data;

# Chained operations with mixed delimiters
my @result = map($_ + 1, grep($_ > 3, sort @data));

# Empty {} blocks for edge case testing
my @empty_map = map {} @data;
my @empty_grep = grep {} @data;
my @empty_sort = sort {} @data;
"#,
            expected_violations: 2,
            expected_ast_nodes: 85,
            parsing_time_us: Some(200),
            syntax_category: SyntaxCategory::BuiltinFunctions,
            dual_indexing: false,
            unicode_safe: true,
        },

        // Comprehensive substitution operator parsing
        PerlSyntaxFixture {
            name: "substitution_operators_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my $text = "hello world test string";

# Standard substitution with different delimiters
$text =~ s/hello/hi/g;
$text =~ s|world|universe|g;
$text =~ s#test#exam#g;

# Balanced delimiters (comprehensive coverage)
$text =~ s{hello}{hi}g;
$text =~ s[world][universe]g;
$text =~ s<test><exam>g;
$text =~ s(hello)(hi)g;

# Single-quote substitution delimiters
$text =~ s'hello'hi'g;
$text =~ s'world'universe'g;

# Complex patterns with modifiers
$text =~ s/(\w+)\s+(\w+)/$2 $1/gi;
$text =~ s/[aeiou]/*/gi;

# Substitution with code evaluation
$text =~ s/(\d+)/ $1 * 2 /ge;

# Transliteration operator
$text =~ tr/a-z/A-Z/;
$text =~ y/aeiou/12345/;

# Alternative delimiter styles
$text =~ s@pattern@replacement@g;
$text =~ s%pattern%replacement%g;
$text =~ s!pattern!replacement!g;

# Nested and escaped delimiters
$text =~ s/pat\/tern/repl\/acement/g;
$text =~ s{pat\{tern}{repl\}acement}g;
"#,
            expected_violations: 1,
            expected_ast_nodes: 72,
            parsing_time_us: Some(180),
            syntax_category: SyntaxCategory::SubstitutionOperators,
            dual_indexing: false,
            unicode_safe: true,
        },

        // Cross-file navigation with dual indexing patterns
        PerlSyntaxFixture {
            name: "cross_file_navigation_dual_indexing",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

# Package declaration for dual indexing
package MyModule::Utils;

# Qualified subroutine definition
sub MyModule::Utils::process_data {
    my ($data) = @_;
    return transform($data);
}

# Bare subroutine definition (dual indexing target)
sub transform {
    my ($input) = @_;
    return validate_input($input);
}

sub validate_input {
    my ($value) = @_;
    return length($value) > 0;
}

# Package-qualified calls (dual indexing source)
my $result1 = MyModule::Utils::process_data("test");
my $result2 = MyModule::Utils::transform("data");

# Bare function calls (dual indexing source)
my $result3 = transform("another");
my $result4 = validate_input("input");

# Cross-package references
use Another::Module;
my $external = Another::Module::helper("value");
my $bare_external = helper("value");

# Complex dual indexing scenario
package Another::Module;

sub helper {
    my ($param) = @_;
    return MyModule::Utils::transform($param);
}

# Method calls (also indexed)
my $obj = MyModule::Utils->new();
my $method_result = $obj->process_data("test");

1;
"#,
            expected_violations: 3,
            expected_ast_nodes: 95,
            parsing_time_us: Some(250),
            syntax_category: SyntaxCategory::PackageNavigation,
            dual_indexing: true,
            unicode_safe: true,
        },

        // Unicode support with UTF-8/UTF-16 boundary testing
        PerlSyntaxFixture {
            name: "unicode_comprehensive_support",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use utf8;

# Unicode identifiers and strings
my $café = "coffee ☕";
my $naïve_approach = "test";
my $résumé = "curriculum vitae";

# Unicode subroutine names
sub analyze_café_data {
    my ($données) = @_;
    return length($données);
}

# Unicode regex patterns
my $text = "Hello 世界 World";
$text =~ s/世界/World/g;
$text =~ /[\x{4e00}-\x{9fff}]/;  # Chinese characters

# Emoji support in identifiers and strings
my $🚀 = "rocket";
my $status_🎉 = "celebration";
print "Status: $status_🎉 $🚀\n";

# Mixed UTF-8/UTF-16 boundary scenarios
my @unicode_array = ("café", "naïve", "résumé", "世界", "🚀");
foreach my $item (@unicode_array) {
    print "Item: $item (length: " . length($item) . ")\n";
}

# Complex Unicode processing
my $combined = join("_", @unicode_array);
my @split_result = split(/_/, $combined);

# Unicode in package names and method calls
package Café::Module;

sub process_données {
    my ($self, $données) = @_;
    return $self->transform_café($données);
}

sub transform_café {
    my ($self, $input) = @_;
    return uc($input);
}

1;
"#,
            expected_violations: 2,
            expected_ast_nodes: 78,
            parsing_time_us: Some(220),
            syntax_category: SyntaxCategory::UnicodeSupport,
            dual_indexing: true,
            unicode_safe: true,
        },

        // Error scenarios and edge cases
        PerlSyntaxFixture {
            name: "error_scenarios_comprehensive",
            perl_code: r#"#!/usr/bin/perl
# Syntax errors and edge cases for error handling testing

use strict;
use warnings;

# Incomplete statement (missing semicolon)
my $incomplete = "test"
# This should cause a syntax error

# Unclosed string literal
my $unclosed = "this string is not closed

# Malformed subroutine
sub broken_function {
    my ($param = @_;  # Syntax error in parameters
    return $param;
}

# Undefined variable usage
sub test_undefined {
    print "Using undefined: $undefined_var\n";
    return $another_undefined * 2;
}

# Malformed regex
if ($text =~ /unclosed_regex) {
    print "This won't work\n";
}

# Invalid operator usage
my $invalid = 5 ** ** 2;  # Double exponentiation

# Incomplete block structure
if ($condition) {
    print "Missing closing brace";
# Intentionally missing }

# Malformed hash reference
my $hash_ref = { incomplete => };

# Invalid package declaration
package ;  # Empty package name

# Circular reference potential
sub circular_a {
    return circular_b();
}

sub circular_b {
    return circular_a();
}
"#,
            expected_violations: 15,
            expected_ast_nodes: 35,
            parsing_time_us: Some(300),
            syntax_category: SyntaxCategory::ErrorScenarios,
            dual_indexing: false,
            unicode_safe: false,
        },

        // Performance benchmark fixture for large files
        PerlSyntaxFixture {
            name: "performance_benchmark_large",
            perl_code: &format!(r#"#!/usr/bin/perl
use strict;
use warnings;

# Large file for performance testing (<1ms parsing requirement)
{}

package Performance::Test;

{}

{}

# Complex nested structures for parsing stress test
my %complex_hash = (
    level1 => {{
        level2 => {{
            level3 => [1, 2, 3, 4, 5],
            data => "test"
        }},
        another => "value"
    }},
    array_ref => [
        {{ key1 => "value1" }},
        {{ key2 => "value2" }},
        {{ key3 => "value3" }}
    ]
);

1;
"#,
    "# Repeated content for file size\n".repeat(50),
    generate_performance_subroutines(25),
    generate_performance_data_structures(10)
),
            expected_violations: 5,
            expected_ast_nodes: 500,
            parsing_time_us: Some(800),
            syntax_category: SyntaxCategory::PerformanceBenchmark,
            dual_indexing: true,
            unicode_safe: true,
        },
    ]
}

/// Generate subroutines for performance testing
#[cfg(test)]
fn generate_performance_subroutines(count: usize) -> String {
    (0..count)
        .map(|i| {
            format!(
                r#"
sub performance_function_{} {{
    my ($param1, $param2, $param3) = @_;
    my $result = $param1 + $param2 * $param3;

    # Complex processing
    for my $item (1..10) {{
        $result += $item if $item % 2 == 0;
    }}

    return $result > 100 ? $result : 0;
}}
"#,
                i
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate data structures for performance testing
#[cfg(test)]
fn generate_performance_data_structures(count: usize) -> String {
    (0..count)
        .map(|i| {
            format!(
                r#"
my @array_{} = qw(item1 item2 item3 item4 item5);
my %hash_{} = (
    key1 => "value1",
    key2 => "value2",
    key3 => "value3"
);

# Processing array_{}
my @processed_{} = map {{ $_ . "_processed" }} @array_{};
my @filtered_{} = grep {{ length($_) > 5 }} @processed_{};
"#,
                i, i, i, i, i, i, i
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Good practices Perl code (minimal violations expected)
#[cfg(test)]
pub fn load_good_practices_fixture() -> PerlSyntaxFixture {
    PerlSyntaxFixture {
        name: "good_practices_comprehensive",
        perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use utf8;

# Well-structured Perl code following best practices

sub calculate_result {
    my ($input_a, $input_b) = @_;

    # Input validation
    return 0 unless defined $input_a && defined $input_b;
    return 0 unless looks_like_number($input_a) && looks_like_number($input_b);

    my $result = $input_a + $input_b;
    return $result;
}

sub process_file_safely {
    my ($filename) = @_;

    # 3-arg open with error checking
    open my $fh, '<', $filename
        or die "Cannot open file '$filename': $!";

    my @lines = <$fh>;

    close $fh
        or warn "Cannot close file '$filename': $!";

    return @lines;
}

# Modern Perl iteration
my @data = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
foreach my $item (@data) {
    print "Processing: $item\n";
}

# Proper error handling
eval {
    my $result = risky_operation();
    process_result($result);
};
if ($@) {
    warn "Operation failed: $@";
}

# Clean subroutine definitions
sub risky_operation {
    my $value = rand(100);
    die "Random failure" if $value < 10;
    return $value;
}

sub process_result {
    my ($value) = @_;
    return $value * 2;
}

1;
"#,
        expected_violations: 1,
        expected_ast_nodes: 68,
        parsing_time_us: Some(120),
        syntax_category: SyntaxCategory::BasicSyntax,
        dual_indexing: false,
        unicode_safe: true,
    }
}

/// Load specific syntax category fixtures
#[cfg(test)]
pub fn load_fixtures_by_category(category: SyntaxCategory) -> Vec<PerlSyntaxFixture> {
    load_comprehensive_syntax_fixtures()
        .into_iter()
        .filter(|fixture| fixture.syntax_category == category)
        .collect()
}

/// Load dual indexing test fixtures only
#[cfg(test)]
pub fn load_dual_indexing_fixtures() -> Vec<PerlSyntaxFixture> {
    load_comprehensive_syntax_fixtures()
        .into_iter()
        .filter(|fixture| fixture.dual_indexing)
        .collect()
}

/// Load Unicode-safe fixtures for UTF-8/UTF-16 testing
#[cfg(test)]
pub fn load_unicode_safe_fixtures() -> Vec<PerlSyntaxFixture> {
    load_comprehensive_syntax_fixtures()
        .into_iter()
        .filter(|fixture| fixture.unicode_safe)
        .collect()
}

use std::sync::LazyLock;
use Scalar::Util qw(looks_like_number);

/// Lazy-loaded fixture registry for efficient test execution
#[cfg(test)]
pub static FIXTURE_REGISTRY: LazyLock<std::collections::HashMap<&'static str, PerlSyntaxFixture>> =
    LazyLock::new(|| {
        let mut registry = std::collections::HashMap::new();

        for fixture in load_comprehensive_syntax_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry.insert("good_practices", load_good_practices_fixture());

        registry
    });

/// Get fixture by name with efficient lookup
#[cfg(test)]
pub fn get_fixture_by_name(name: &str) -> Option<&'static PerlSyntaxFixture> {
    FIXTURE_REGISTRY.get(name)
}