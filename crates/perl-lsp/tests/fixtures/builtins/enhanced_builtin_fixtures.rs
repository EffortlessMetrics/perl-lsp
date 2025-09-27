//! Enhanced builtin function parsing test fixtures
//!
//! Comprehensive test data for enhanced builtin function parsing including
//! deterministic parsing of map/grep/sort functions with {} blocks and
//! comprehensive delimiter coverage for parsing accuracy validation.
//!
//! Features:
//! - Deterministic parsing scenarios with {} blocks
//! - Empty block edge case validation
//! - Nested builtin function combinations
//! - Performance benchmarks for <1ms parsing requirements
//! - Mixed delimiter styles (parentheses, braces, mixed)

#[cfg(test)]
pub struct BuiltinFunctionFixture {
    pub name: &'static str,
    pub perl_code: &'static str,
    pub expected_ast_nodes: usize,
    pub function_type: BuiltinType,
    pub block_parsing: BlockParsingMode,
    pub delimiter_style: DelimiterStyle,
    pub parsing_time_us: Option<u64>,
    pub deterministic: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinType {
    Map,
    Grep,
    Sort,
    Mixed,
    Nested,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum BlockParsingMode {
    BraceBlocks,      // { ... }
    ParenthesesOnly,  // ( ... )
    Mixed,            // Both styles
    Empty,            // Empty blocks {}
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum DelimiterStyle {
    Standard,         // Standard delimiters
    Balanced,         // Balanced delimiters like [], {}, <>
    Alternative,      // Alternative delimiters like |, #, @
    Mixed,           // Mixed delimiter usage
}

/// Enhanced map function parsing fixtures
#[cfg(test)]
pub fn load_map_function_fixtures() -> Vec<BuiltinFunctionFixture> {
    vec![
        BuiltinFunctionFixture {
            name: "map_basic_brace_blocks",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @numbers = (1, 2, 3, 4, 5);

# Basic map with brace blocks
my @doubled = map { $_ * 2 } @numbers;
my @squared = map { $_ * $_ } @numbers;

# Complex map with multiple statements
my @processed = map {
    my $val = $_;
    $val > 3 ? $val * 2 : $val
} @numbers;

# Map with function calls in block
my @formatted = map { sprintf("Number: %d", $_) } @numbers;

print "Map processing completed\n";
"#,
            expected_ast_nodes: 42,
            function_type: BuiltinType::Map,
            block_parsing: BlockParsingMode::BraceBlocks,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(85),
            deterministic: true,
        },

        BuiltinFunctionFixture {
            name: "map_empty_blocks_edge_case",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @data = (1, 2, 3, 4, 5);

# Empty map blocks (edge case for parser robustness)
my @empty_map = map {} @data;

# Map with just return statement
my @identity = map { $_ } @data;

# Map with minimal processing
my @incremented = map { $_ + 1 } @data;

print "Empty block edge cases handled\n";
"#,
            expected_ast_nodes: 28,
            function_type: BuiltinType::Map,
            block_parsing: BlockParsingMode::Empty,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(55),
            deterministic: true,
        },

        BuiltinFunctionFixture {
            name: "map_mixed_delimiters",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @values = (1, 2, 3, 4, 5);

# Map with parentheses (traditional style)
my @traditional = map($_ * 2, @values);

# Map with brace blocks (modern style)
my @modern = map { $_ * 3 } @values;

# Mixed usage in same context
my @combined = map {
    $_ % 2 == 0 ? $_ * 2 : map($_ + 1, ($_))
} @values;

print "Mixed delimiter styles processed\n";
"#,
            expected_ast_nodes: 48,
            function_type: BuiltinType::Map,
            block_parsing: BlockParsingMode::Mixed,
            delimiter_style: DelimiterStyle::Mixed,
            parsing_time_us: Some(95),
            deterministic: true,
        },
    ]
}

/// Enhanced grep function parsing fixtures
#[cfg(test)]
pub fn load_grep_function_fixtures() -> Vec<BuiltinFunctionFixture> {
    vec![
        BuiltinFunctionFixture {
            name: "grep_complex_conditions",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @numbers = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);

# Grep with complex conditions
my @evens = grep { $_ % 2 == 0 } @numbers;
my @odds = grep { $_ % 2 == 1 } @numbers;

# Grep with multiple condition checks
my @filtered = grep {
    my $val = $_;
    $val > 3 && $val < 8 && $val % 2 == 0
} @numbers;

# Grep with function calls in condition
my @valid = grep { defined $_ && length("$_") > 0 } @numbers;

# Chained grep operations
my @complex = grep { $_ > 2 } grep { $_ % 2 == 0 } @numbers;

print "Grep filtering completed\n";
"#,
            expected_ast_nodes: 58,
            function_type: BuiltinType::Grep,
            block_parsing: BlockParsingMode::BraceBlocks,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(105),
            deterministic: true,
        },

        BuiltinFunctionFixture {
            name: "grep_empty_and_minimal",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @test_data = (0, 1, undef, 2, "", 3);

# Empty grep block (edge case)
my @empty_grep = grep {} @test_data;

# Minimal grep conditions
my @truthy = grep { $_ } @test_data;
my @defined_vals = grep { defined $_ } @test_data;

# Single character conditions
my @non_zero = grep { $_ } @test_data;

print "Minimal grep cases processed\n";
"#,
            expected_ast_nodes: 32,
            function_type: BuiltinType::Grep,
            block_parsing: BlockParsingMode::Empty,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(48),
            deterministic: true,
        },
    ]
}

/// Enhanced sort function parsing fixtures
#[cfg(test)]
pub fn load_sort_function_fixtures() -> Vec<BuiltinFunctionFixture> {
    vec![
        BuiltinFunctionFixture {
            name: "sort_comparison_blocks",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @numbers = (5, 2, 8, 1, 9, 3);
my @strings = ("apple", "banana", "cherry", "date");

# Numeric sort with comparison block
my @sorted_nums = sort { $a <=> $b } @numbers;
my @reverse_nums = sort { $b <=> $a } @numbers;

# String sort with comparison block
my @sorted_strings = sort { $a cmp $b } @strings;
my @reverse_strings = sort { $b cmp $a } @strings;

# Complex sort with custom comparison
my @complex_sort = sort {
    # Multi-criteria sorting
    length($a) <=> length($b) || $a cmp $b
} @strings;

# Sort by custom function
my @custom_sort = sort {
    my $val_a = calculate_weight($a);
    my $val_b = calculate_weight($b);
    $val_a <=> $val_b
} @strings;

sub calculate_weight {
    my ($str) = @_;
    return length($str) * ord(substr($str, 0, 1));
}

print "Sort operations completed\n";
"#,
            expected_ast_nodes: 85,
            function_type: BuiltinType::Sort,
            block_parsing: BlockParsingMode::BraceBlocks,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(125),
            deterministic: true,
        },

        BuiltinFunctionFixture {
            name: "sort_empty_and_default",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @data = qw(zebra apple banana cherry);
my @numbers = (42, 1, 17, 3);

# Default sort (no comparison block)
my @default_sort = sort @data;

# Empty sort block (edge case)
my @empty_sort = sort {} @data;

# Minimal comparison blocks
my @minimal_string = sort { $a cmp $b } @data;
my @minimal_numeric = sort { $a <=> $b } @numbers;

print "Basic sort cases processed\n";
"#,
            expected_ast_nodes: 38,
            function_type: BuiltinType::Sort,
            block_parsing: BlockParsingMode::Empty,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(62),
            deterministic: true,
        },
    ]
}

/// Nested and chained builtin function fixtures
#[cfg(test)]
pub fn load_nested_builtin_fixtures() -> Vec<BuiltinFunctionFixture> {
    vec![
        BuiltinFunctionFixture {
            name: "nested_map_grep_sort_complex",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;

my @data = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);

# Complex nested operations
my @result = map {
    $_ * 2
} grep {
    $_ % 2 == 0
} sort {
    $b <=> $a
} @data;

# Multiple levels of nesting
my @complex_nested = map {
    my $outer = $_;
    map {
        my $inner = $_;
        grep {
            $_ > $inner
        } ($outer, $outer * 2, $outer * 3)
    } (1, 2, 3)
} grep {
    $_ > 5
} @data;

# Chained operations with different block styles
my @chained =
    sort { $a <=> $b }
    map { $_ + 10 }
    grep { defined $_ }
    map { $_ > 0 ? $_ : undef }
    @data;

print "Nested operations completed\n";
"#,
            expected_ast_nodes: 95,
            function_type: BuiltinType::Nested,
            block_parsing: BlockParsingMode::BraceBlocks,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(180),
            deterministic: true,
        },

        BuiltinFunctionFixture {
            name: "performance_stress_builtins",
            perl_code: &format!(r#"#!/usr/bin/perl
use strict;
use warnings;

# Large dataset for performance testing
my @large_dataset = (1..1000);

{}

# Performance-critical nested operations
my @final_result = sort {{ $a <=> $b }}
                   map {{ $_ * 3 }}
                   grep {{ $_ % 7 == 0 }}
                   @large_dataset;

print "Performance stress test completed\n";
"#, generate_performance_builtin_operations(50)),
            expected_ast_nodes: 750,
            function_type: BuiltinType::Mixed,
            block_parsing: BlockParsingMode::BraceBlocks,
            delimiter_style: DelimiterStyle::Standard,
            parsing_time_us: Some(850),
            deterministic: true,
        },
    ]
}

/// Generate repeated builtin operations for performance testing
#[cfg(test)]
fn generate_performance_builtin_operations(count: usize) -> String {
    (0..count)
        .map(|i| {
            format!(
                r#"
# Performance test operation {}
my @step_{} = map {{ $_ + {} }} grep {{ $_ % {} == 0 }} sort {{ $a <=> $b }} @large_dataset;
"#,
                i, i, i, (i % 10) + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Load all enhanced builtin function fixtures
#[cfg(test)]
pub fn load_all_builtin_fixtures() -> Vec<BuiltinFunctionFixture> {
    let mut all_fixtures = Vec::new();

    all_fixtures.extend(load_map_function_fixtures());
    all_fixtures.extend(load_grep_function_fixtures());
    all_fixtures.extend(load_sort_function_fixtures());
    all_fixtures.extend(load_nested_builtin_fixtures());

    all_fixtures
}

/// Load fixtures by builtin function type
#[cfg(test)]
pub fn load_fixtures_by_type(builtin_type: BuiltinType) -> Vec<BuiltinFunctionFixture> {
    load_all_builtin_fixtures()
        .into_iter()
        .filter(|fixture| fixture.function_type == builtin_type)
        .collect()
}

/// Load fixtures by block parsing mode
#[cfg(test)]
pub fn load_fixtures_by_parsing_mode(mode: BlockParsingMode) -> Vec<BuiltinFunctionFixture> {
    load_all_builtin_fixtures()
        .into_iter()
        .filter(|fixture| fixture.block_parsing == mode)
        .collect()
}

/// Load deterministic parsing fixtures only
#[cfg(test)]
pub fn load_deterministic_fixtures() -> Vec<BuiltinFunctionFixture> {
    load_all_builtin_fixtures()
        .into_iter()
        .filter(|fixture| fixture.deterministic)
        .collect()
}

/// Load performance benchmark fixtures (parsing time > 100us)
#[cfg(test)]
pub fn load_performance_fixtures() -> Vec<BuiltinFunctionFixture> {
    load_all_builtin_fixtures()
        .into_iter()
        .filter(|fixture| {
            fixture.parsing_time_us.map_or(false, |time| time > 100)
        })
        .collect()
}

use std::sync::LazyLock;
use std::collections::HashMap;

/// Lazy-loaded builtin function fixture registry
#[cfg(test)]
pub static BUILTIN_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, BuiltinFunctionFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_all_builtin_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get builtin fixture by name
#[cfg(test)]
pub fn get_builtin_fixture_by_name(name: &str) -> Option<&'static BuiltinFunctionFixture> {
    BUILTIN_FIXTURE_REGISTRY.get(name)
}