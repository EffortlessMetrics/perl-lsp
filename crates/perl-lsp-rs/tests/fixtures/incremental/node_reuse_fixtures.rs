//! Incremental parsing test fixtures with node reuse validation
//!
//! Comprehensive test data for incremental parsing validation with <1ms update scenarios
//! and 70-99% node reuse efficiency testing. Supports performance validation for
//! revolutionary LSP parsing improvements.
//!
//! Features:
//! - Edit operation simulation with realistic code change patterns
//! - Node reuse efficiency validation (70-99% reuse targets)
//! - Update time benchmarking (<1ms requirement validation)
//! - AST delta tracking with minimal change detection
//! - Position tracking with UTF-16 boundary preservation

#[cfg(test)]
pub struct IncrementalParsingFixture {
    pub name: &'static str,
    pub description: &'static str,
    pub initial_perl_source: &'static str,
    pub edit_operations: Vec<EditOperation>,
    pub expected_reuse_percentage: f32,
    pub update_time_ms: f32,
    pub node_count_delta: i32,
    pub reuse_efficiency_target: f32,
    pub utf16_safe: bool,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct EditOperation {
    pub operation_type: EditType,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub new_text: String,
    pub expected_affected_nodes: usize,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum EditType {
    Insert,
    Delete,
    Replace,
    Comment,
    Rename,
    Refactor,
}

/// Small edit operations with high node reuse efficiency
#[cfg(test)]
pub fn load_small_edit_fixtures() -> Vec<IncrementalParsingFixture> {
    vec![
        IncrementalParsingFixture {
            name: "variable_name_change",
            description: "Simple variable name change with high reuse efficiency",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

my $original_variable = "test value";
my $other_var = calculate_result($original_variable);

sub calculate_result {
    my ($input) = @_;
    return length($input) * 2;
}

print "Result: $other_var\n";
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 3,
                    start_column: 3,
                    end_line: 3,
                    end_column: 20,
                    new_text: "$renamed_variable".to_string(),
                    expected_affected_nodes: 2,
                },
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 4,
                    start_column: 35,
                    end_line: 4,
                    end_column: 52,
                    new_text: "$renamed_variable".to_string(),
                    expected_affected_nodes: 1,
                },
            ],
            expected_reuse_percentage: 95.5,
            update_time_ms: 0.3,
            node_count_delta: 0,
            reuse_efficiency_target: 95.0,
            utf16_safe: true,
        },

        IncrementalParsingFixture {
            name: "comment_addition",
            description: "Adding comments with minimal AST impact",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

sub process_data {
    my ($data) = @_;
    my $result = transform($data);
    return $result;
}

sub transform {
    my ($input) = @_;
    return uc($input);
}
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Insert,
                    start_line: 4,
                    start_column: 0,
                    end_line: 4,
                    end_column: 0,
                    new_text: "# Process input data and return transformed result\n".to_string(),
                    expected_affected_nodes: 0,
                },
                EditOperation {
                    operation_type: EditType::Insert,
                    start_line: 11,
                    start_column: 0,
                    end_line: 11,
                    end_column: 0,
                    new_text: "# Transform input to uppercase\n".to_string(),
                    expected_affected_nodes: 0,
                },
            ],
            expected_reuse_percentage: 99.2,
            update_time_ms: 0.15,
            node_count_delta: 0,
            reuse_efficiency_target: 99.0,
            utf16_safe: true,
        },

        IncrementalParsingFixture {
            name: "string_literal_modification",
            description: "Modifying string literals with localized impact",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

my $message = "Hello world";
my $greeting = "Welcome to the application";
my $status = "Processing complete";

print "$message\n";
print "$greeting\n";
print "$status\n";
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 4,
                    start_column: 15,
                    end_line: 4,
                    end_column: 28,
                    new_text: "\"Hi there!\"".to_string(),
                    expected_affected_nodes: 1,
                },
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 5,
                    start_column: 16,
                    end_line: 5,
                    end_column: 42,
                    new_text: "\"Welcome to our system\"".to_string(),
                    expected_affected_nodes: 1,
                },
            ],
            expected_reuse_percentage: 92.8,
            update_time_ms: 0.4,
            node_count_delta: 0,
            reuse_efficiency_target: 90.0,
            utf16_safe: true,
        },
    ]
}

/// Medium complexity edit operations
#[cfg(test)]
pub fn load_medium_edit_fixtures() -> Vec<IncrementalParsingFixture> {
    vec![
        IncrementalParsingFixture {
            name: "function_parameter_addition",
            description: "Adding parameters to function with moderate AST impact",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

sub calculate {
    my ($value) = @_;
    return $value * 2;
}

sub process {
    my ($data) = @_;
    my $result = calculate($data);
    return format_result($result);
}

sub format_result {
    my ($num) = @_;
    return "Result: $num";
}

my $output = process(42);
print "$output\n";
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 5,
                    start_column: 4,
                    end_line: 5,
                    end_column: 18,
                    new_text: "my ($value, $multiplier) = @_;".to_string(),
                    expected_affected_nodes: 3,
                },
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 6,
                    start_column: 11,
                    end_line: 6,
                    end_column: 23,
                    new_text: "$value * $multiplier".to_string(),
                    expected_affected_nodes: 2,
                },
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 12,
                    start_column: 21,
                    end_line: 12,
                    end_column: 36,
                    new_text: "calculate($data, 3)".to_string(),
                    expected_affected_nodes: 1,
                },
            ],
            expected_reuse_percentage: 85.2,
            update_time_ms: 0.7,
            node_count_delta: 4,
            reuse_efficiency_target: 80.0,
            utf16_safe: true,
        },

        IncrementalParsingFixture {
            name: "control_structure_modification",
            description: "Modifying control structures with controlled reuse impact",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

my @data = (1, 2, 3, 4, 5);

for my $item (@data) {
    if ($item % 2 == 0) {
        print "Even: $item\n";
    } else {
        print "Odd: $item\n";
    }
}

print "Processing completed\n";
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 6,
                    start_column: 0,
                    end_line: 12,
                    end_column: 1,
                    new_text: r#"foreach my $element (@data) {
    if ($element % 2 == 0) {
        print "Even number: $element\n";
    } elsif ($element % 3 == 0) {
        print "Multiple of 3: $element\n";
    } else {
        print "Odd number: $element\n";
    }
}"#.to_string(),
                    expected_affected_nodes: 15,
                },
            ],
            expected_reuse_percentage: 78.5,
            update_time_ms: 0.9,
            node_count_delta: 8,
            reuse_efficiency_target: 75.0,
            utf16_safe: true,
        },
    ]
}

/// Complex edit operations with substantial changes
#[cfg(test)]
pub fn load_complex_edit_fixtures() -> Vec<IncrementalParsingFixture> {
    vec![
        IncrementalParsingFixture {
            name: "function_refactoring",
            description: "Major function refactoring with multiple changes",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

sub process_file {
    my ($filename) = @_;

    open my $fh, '<', $filename or die "Cannot open $filename: $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    $content =~ s/\r\n/\n/g;
    $content =~ s/\s+$//gm;

    return $content;
}

sub save_file {
    my ($filename, $content) = @_;

    open my $fh, '>', $filename or die "Cannot write $filename: $!";
    print $fh $content;
    close $fh;
}

my $data = process_file("input.txt");
save_file("output.txt", $data);
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 4,
                    start_column: 0,
                    end_line: 15,
                    end_column: 1,
                    new_text: r#"sub process_file {
    my ($filename, $options) = @_;
    $options ||= {};

    return read_file_content($filename, $options);
}

sub read_file_content {
    my ($filename, $options) = @_;

    open my $fh, '<', $filename or die "Cannot open $filename: $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    if ($options->{normalize}) {
        $content = normalize_content($content);
    }

    return $content;
}

sub normalize_content {
    my ($content) = @_;
    $content =~ s/\r\n/\n/g;
    $content =~ s/\s+$//gm;
    return $content;
}"#.to_string(),
                    expected_affected_nodes: 25,
                },
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 25,
                    start_column: 12,
                    end_line: 25,
                    end_column: 38,
                    new_text: r#"process_file("input.txt", {normalize => 1})"#.to_string(),
                    expected_affected_nodes: 3,
                },
            ],
            expected_reuse_percentage: 72.8,
            update_time_ms: 1.2,
            node_count_delta: 20,
            reuse_efficiency_target: 70.0,
            utf16_safe: true,
        },

        IncrementalParsingFixture {
            name: "package_structure_change",
            description: "Major package structure modification with cross-file impact",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

package MyModule;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process {
    my ($self, $data) = @_;
    return $self->transform($data);
}

sub transform {
    my ($self, $input) = @_;
    return uc($input);
}

package main;

my $module = MyModule->new();
my $result = $module->process("test data");
print "Result: $result\n";
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 4,
                    start_column: 0,
                    end_line: 19,
                    end_column: 1,
                    new_text: r#"package MyModule::Processor;

our $VERSION = '1.0.0';

sub new {
    my ($class, $config) = @_;
    return bless {
        config => $config || {},
        processed_count => 0
    }, $class;
}

sub process {
    my ($self, $data) = @_;
    $self->{processed_count}++;
    return $self->transform($data);
}

sub transform {
    my ($self, $input) = @_;
    my $method = $self->{config}->{transform_method} || 'uppercase';

    if ($method eq 'uppercase') {
        return uc($input);
    } elsif ($method eq 'lowercase') {
        return lc($input);
    } else {
        return $input;
    }
}

sub get_stats {
    my ($self) = @_;
    return {
        processed_count => $self->{processed_count}
    };
}"#.to_string(),
                    expected_affected_nodes: 35,
                },
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 23,
                    start_column: 12,
                    end_line: 23,
                    end_column: 28,
                    new_text: r#"MyModule::Processor->new({transform_method => 'uppercase'})"#.to_string(),
                    expected_affected_nodes: 2,
                },
            ],
            expected_reuse_percentage: 68.5,
            update_time_ms: 1.5,
            node_count_delta: 25,
            reuse_efficiency_target: 65.0,
            utf16_safe: true,
        },
    ]
}

/// Unicode and UTF-16 boundary safe edit operations
#[cfg(test)]
pub fn load_unicode_safe_edit_fixtures() -> Vec<IncrementalParsingFixture> {
    vec![
        IncrementalParsingFixture {
            name: "unicode_string_modification",
            description: "Unicode string modifications with proper boundary handling",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;
use utf8;

my $café = "coffee shop ☕";
my $naïve_approach = "simple method";
my $résumé = "curriculum vitae";

print "Location: $café\n";
print "Method: $naïve_approach\n";
print "Document: $résumé\n";
"#,
            edit_operations: vec![
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 5,
                    start_column: 11,
                    end_line: 5,
                    end_column: 28,
                    new_text: "\"restaurant 🍽️\"".to_string(),
                    expected_affected_nodes: 1,
                },
                EditOperation {
                    operation_type: EditType::Replace,
                    start_line: 6,
                    start_column: 22,
                    end_line: 6,
                    end_column: 37,
                    new_text: "\"advanced technique\"".to_string(),
                    expected_affected_nodes: 1,
                },
            ],
            expected_reuse_percentage: 94.2,
            update_time_ms: 0.5,
            node_count_delta: 0,
            reuse_efficiency_target: 90.0,
            utf16_safe: true,
        },
    ]
}

/// Performance stress test fixtures for incremental parsing
#[cfg(test)]
pub fn load_performance_stress_fixtures() -> Vec<IncrementalParsingFixture> {
    vec![
        IncrementalParsingFixture {
            name: "large_file_incremental_edit",
            description: "Performance test with large file and multiple edits",
            initial_perl_source: &generate_large_perl_file(500),
            edit_operations: generate_multiple_edits(20),
            expected_reuse_percentage: 88.5,
            update_time_ms: 0.8,
            node_count_delta: 15,
            reuse_efficiency_target: 85.0,
            utf16_safe: true,
        },
    ]
}

/// Generate large Perl file for performance testing
#[cfg(test)]
fn generate_large_perl_file(lines: usize) -> String {
    let mut content = String::from(r#"#!/usr/bin/perl
use strict;
use warnings;

"#);

    for i in 0..lines {
        content.push_str(&format!(
            r#"
sub function_{} {{
    my ($param) = @_;
    my $result = $param * {};
    return $result > 100 ? $result : 0;
}}

my $value_{} = function_{}({});
"#,
            i, i + 1, i, i, i * 2
        ));
    }

    content.push_str("\nprint \"Large file processing completed\\n\";\n");
    content
}

/// Generate multiple edit operations for performance testing
#[cfg(test)]
fn generate_multiple_edits(count: usize) -> Vec<EditOperation> {
    (0..count)
        .map(|i| EditOperation {
            operation_type: if i % 2 == 0 { EditType::Replace } else { EditType::Insert },
            start_line: (i * 5) + 10,
            start_column: 4,
            end_line: (i * 5) + 10,
            end_column: 20,
            new_text: format!("# Modified function {}\n", i),
            expected_affected_nodes: 1,
        })
        .collect()
}

/// Load all incremental parsing fixtures
#[cfg(test)]
pub fn load_all_incremental_fixtures() -> Vec<IncrementalParsingFixture> {
    let mut all_fixtures = Vec::new();

    all_fixtures.extend(load_small_edit_fixtures());
    all_fixtures.extend(load_medium_edit_fixtures());
    all_fixtures.extend(load_complex_edit_fixtures());
    all_fixtures.extend(load_unicode_safe_edit_fixtures());
    all_fixtures.extend(load_performance_stress_fixtures());

    all_fixtures
}

/// Load fixtures by edit complexity
#[cfg(test)]
pub fn load_fixtures_by_reuse_efficiency(min_efficiency: f32) -> Vec<IncrementalParsingFixture> {
    load_all_incremental_fixtures()
        .into_iter()
        .filter(|fixture| fixture.expected_reuse_percentage >= min_efficiency)
        .collect()
}

/// Load fixtures by update time performance
#[cfg(test)]
pub fn load_fixtures_by_update_time(max_time_ms: f32) -> Vec<IncrementalParsingFixture> {
    load_all_incremental_fixtures()
        .into_iter()
        .filter(|fixture| fixture.update_time_ms <= max_time_ms)
        .collect()
}

/// Load UTF-16 safe fixtures only
#[cfg(test)]
pub fn load_utf16_safe_fixtures() -> Vec<IncrementalParsingFixture> {
    load_all_incremental_fixtures()
        .into_iter()
        .filter(|fixture| fixture.utf16_safe)
        .collect()
}

use std::sync::LazyLock;
use std::collections::HashMap;

/// Lazy-loaded incremental parsing fixture registry
#[cfg(test)]
pub static INCREMENTAL_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, IncrementalParsingFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_all_incremental_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get incremental parsing fixture by name
#[cfg(test)]
pub fn get_incremental_fixture_by_name(name: &str) -> Option<&'static IncrementalParsingFixture> {
    INCREMENTAL_FIXTURE_REGISTRY.get(name)
}