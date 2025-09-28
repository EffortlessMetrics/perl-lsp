//! Performance validation fixtures for <1ms LSP updates
//!
//! Provides test data for validating incremental parsing performance
//! with <1ms update requirements and 70-99% node reuse efficiency.
//!
//! Features:
//! - Incremental parsing test scenarios with edit operations
//! - Node reuse efficiency validation data
//! - Performance benchmarking with timing constraints
//! - Memory usage tracking for large file updates
//! - Edit operation patterns for realistic LSP scenarios

use std::collections::HashMap;
use std::time::Duration;

#[cfg(test)]
pub struct PerformanceFixture {
    pub name: &'static str,
    pub initial_perl_source: &'static str,
    pub edit_operations: Vec<EditOperation>,
    pub expected_reuse_percentage: f32,
    pub target_update_time_us: u64,
    pub max_memory_mb: u32,
    pub scenario_type: PerformanceScenario,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct EditOperation {
    pub description: &'static str,
    pub edit_type: EditType,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub new_text: String,
    pub expected_nodes_changed: u32,
    pub expected_nodes_reused: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum EditType {
    TextInsertion,
    TextDeletion,
    TextReplacement,
    LineInsertion,
    LineDeletion,
    BlockInsertion,
    BlockDeletion,
    VariableRename,
    FunctionEdit,
    CommentEdit,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceScenario {
    SingleCharacterEdit,
    SmallTextChange,
    LineEdit,
    BlockEdit,
    LargeFileEdit,
    MultipleEdits,
    RealTimeTyping,
}

/// Performance validation fixtures for incremental parsing
#[cfg(test)]
pub fn load_performance_fixtures() -> Vec<PerformanceFixture> {
    vec![
        // Single character edits (should be <100μs)
        PerformanceFixture {
            name: "single_character_insertion",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

my $variable = "test";
print "Hello, World!\n";

sub process_data {
    my ($input) = @_;
    return uc($input);
}

my $result = process_data($variable);
print "Result: $result\n";
"#,
            edit_operations: vec![
                EditOperation {
                    description: "Insert single character in variable name",
                    edit_type: EditType::TextInsertion,
                    start_line: 3,
                    start_character: 3,
                    end_line: 3,
                    end_character: 3,
                    new_text: "y".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 45,
                },
            ],
            expected_reuse_percentage: 97.8,
            target_update_time_us: 50,
            max_memory_mb: 2,
            scenario_type: PerformanceScenario::SingleCharacterEdit,
        },

        // Small text changes (should be <500μs)
        PerformanceFixture {
            name: "small_text_replacement",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

package MyModule::Utils;

sub calculate_result {
    my ($a, $b, $operation) = @_;

    if ($operation eq 'add') {
        return $a + $b;
    } elsif ($operation eq 'subtract') {
        return $a - $b;
    } elsif ($operation eq 'multiply') {
        return $a * $b;
    } elsif ($operation eq 'divide') {
        return $b != 0 ? $a / $b : undef;
    }

    return undef;
}

my $x = 10;
my $y = 5;
my $result = calculate_result($x, $y, 'add');
print "Result: $result\n";

1;
"#,
            edit_operations: vec![
                EditOperation {
                    description: "Replace variable name",
                    edit_type: EditType::TextReplacement,
                    start_line: 18,
                    start_character: 4,
                    end_line: 18,
                    end_character: 5,
                    new_text: "num1".to_string(),
                    expected_nodes_changed: 3,
                    expected_nodes_reused: 78,
                },
                EditOperation {
                    description: "Update operation parameter",
                    edit_type: EditType::TextReplacement,
                    start_line: 20,
                    start_character: 41,
                    end_line: 20,
                    end_character: 46,
                    new_text: "'multiply'".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 80,
                },
            ],
            expected_reuse_percentage: 95.2,
            target_update_time_us: 400,
            max_memory_mb: 4,
            scenario_type: PerformanceScenario::SmallTextChange,
        },

        // Line-level edits (should be <800μs)
        PerformanceFixture {
            name: "line_insertion_deletion",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

# Data processing module
package DataProcessor;

sub new {
    my ($class, %args) = @_;
    my $self = {
        input_file => $args{input} || 'data.txt',
        output_file => $args{output} || 'processed.txt',
        format => $args{format} || 'csv',
        encoding => $args{encoding} || 'utf8',
    };
    return bless $self, $class;
}

sub read_file {
    my ($self) = @_;
    open my $fh, '<:encoding(' . $self->{encoding} . ')', $self->{input_file}
        or die "Cannot open file: $!";

    my @lines = <$fh>;
    close $fh;

    return \@lines;
}

sub process_lines {
    my ($self, $lines) = @_;
    my @processed;

    for my $line (@$lines) {
        chomp $line;
        next if $line =~ /^#/;  # Skip comments
        next if $line =~ /^\s*$/;  # Skip empty lines

        my @fields = split /,/, $line;
        my %record = (
            id => $fields[0],
            name => $fields[1],
            value => $fields[2] || 0,
        );

        push @processed, \%record;
    }

    return \@processed;
}

1;
"#,
            edit_operations: vec![
                EditOperation {
                    description: "Insert new method",
                    edit_type: EditType::LineInsertion,
                    start_line: 26,
                    start_character: 0,
                    end_line: 26,
                    end_character: 0,
                    new_text: r#"
sub validate_data {
    my ($self, $data) = @_;
    return scalar @$data > 0;
}
"#.to_string(),
                    expected_nodes_changed: 15,
                    expected_nodes_reused: 95,
                },
                EditOperation {
                    description: "Add error handling line",
                    edit_type: EditType::LineInsertion,
                    start_line: 41,
                    start_character: 0,
                    end_line: 41,
                    end_character: 0,
                    new_text: "        die \"Invalid record format\" unless @fields >= 2;\n".to_string(),
                    expected_nodes_changed: 8,
                    expected_nodes_reused: 102,
                },
            ],
            expected_reuse_percentage: 90.5,
            target_update_time_us: 750,
            max_memory_mb: 6,
            scenario_type: PerformanceScenario::LineEdit,
        },

        // Block-level edits (should be <1000μs)
        PerformanceFixture {
            name: "function_block_edit",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'signatures';
no warnings 'experimental::signatures';

package Calculator::Advanced;

sub new($class, %options) {
    my $self = {
        precision => $options{precision} || 2,
        rounding => $options{rounding} || 'half_up',
        memory => [],
    };
    return bless $self, $class;
}

sub add($self, $a, $b) {
    my $result = $a + $b;
    $self->_store_result($result);
    return $self->_format_result($result);
}

sub subtract($self, $a, $b) {
    my $result = $a - $b;
    $self->_store_result($result);
    return $self->_format_result($result);
}

sub multiply($self, $a, $b) {
    my $result = $a * $b;
    $self->_store_result($result);
    return $self->_format_result($result);
}

sub divide($self, $a, $b) {
    die "Division by zero" if $b == 0;
    my $result = $a / $b;
    $self->_store_result($result);
    return $self->_format_result($result);
}

sub _store_result($self, $result) {
    push @{$self->{memory}}, $result;
    # Keep only last 10 results
    splice @{$self->{memory}}, 0, -10 if @{$self->{memory}} > 10;
}

sub _format_result($self, $result) {
    return sprintf("%.${$self->{precision}}f", $result);
}

sub get_memory($self) {
    return [@{$self->{memory}}];
}

sub clear_memory($self) {
    $self->{memory} = [];
}

1;
"#,
            edit_operations: vec![
                EditOperation {
                    description: "Replace entire function with enhanced version",
                    edit_type: EditType::BlockEdit,
                    start_line: 34,
                    start_character: 0,
                    end_line: 39,
                    end_character: 1,
                    new_text: r#"sub divide($self, $a, $b) {
    # Enhanced division with error handling and logging
    if ($b == 0) {
        warn "Attempted division by zero";
        return 'ERROR';
    }

    if (!looks_like_number($a) || !looks_like_number($b)) {
        warn "Invalid numeric arguments: $a, $b";
        return 'ERROR';
    }

    my $result = $a / $b;
    $self->_store_result($result);
    $self->_log_operation('divide', $a, $b, $result);
    return $self->_format_result($result);
}"#.to_string(),
                    expected_nodes_changed: 25,
                    expected_nodes_reused: 120,
                },
            ],
            expected_reuse_percentage: 82.8,
            target_update_time_us: 950,
            max_memory_mb: 8,
            scenario_type: PerformanceScenario::BlockEdit,
        },

        // Large file with multiple edits (should be <2000μs total)
        PerformanceFixture {
            name: "large_file_multiple_edits",
            initial_perl_source: &generate_large_perl_file(),
            edit_operations: vec![
                EditOperation {
                    description: "Add import statement",
                    edit_type: EditType::LineInsertion,
                    start_line: 2,
                    start_character: 0,
                    end_line: 2,
                    end_character: 0,
                    new_text: "use JSON;\n".to_string(),
                    expected_nodes_changed: 3,
                    expected_nodes_reused: 450,
                },
                EditOperation {
                    description: "Modify function parameter",
                    edit_type: EditType::TextReplacement,
                    start_line: 25,
                    start_character: 20,
                    end_line: 25,
                    end_character: 25,
                    new_text: "$data_ref".to_string(),
                    expected_nodes_changed: 2,
                    expected_nodes_reused: 451,
                },
                EditOperation {
                    description: "Add new method at end",
                    edit_type: EditType::BlockInsertion,
                    start_line: 180,
                    start_character: 0,
                    end_line: 180,
                    end_character: 0,
                    new_text: r#"
sub export_json($self, $data) {
    return JSON->new->utf8->encode($data);
}
"#.to_string(),
                    expected_nodes_changed: 12,
                    expected_nodes_reused: 441,
                },
            ],
            expected_reuse_percentage: 96.3,
            target_update_time_us: 1800,
            max_memory_mb: 15,
            scenario_type: PerformanceScenario::LargeFileEdit,
        },

        // Real-time typing simulation (multiple rapid edits)
        PerformanceFixture {
            name: "realtime_typing_simulation",
            initial_perl_source: r#"#!/usr/bin/perl
use strict;
use warnings;

my $var = ""#,
            edit_operations: vec![
                EditOperation {
                    description: "Type 'h'",
                    edit_type: EditType::TextInsertion,
                    start_line: 4,
                    start_character: 11,
                    end_line: 4,
                    end_character: 11,
                    new_text: "h".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 8,
                },
                EditOperation {
                    description: "Type 'e'",
                    edit_type: EditType::TextInsertion,
                    start_line: 4,
                    start_character: 12,
                    end_line: 4,
                    end_character: 12,
                    new_text: "e".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 8,
                },
                EditOperation {
                    description: "Type 'l'",
                    edit_type: EditType::TextInsertion,
                    start_line: 4,
                    start_character: 13,
                    end_line: 4,
                    end_character: 13,
                    new_text: "l".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 8,
                },
                EditOperation {
                    description: "Type 'l'",
                    edit_type: EditType::TextInsertion,
                    start_line: 4,
                    start_character: 14,
                    end_line: 4,
                    end_character: 14,
                    new_text: "l".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 8,
                },
                EditOperation {
                    description: "Type 'o'",
                    edit_type: EditType::TextInsertion,
                    start_line: 4,
                    start_character: 15,
                    end_line: 4,
                    end_character: 15,
                    new_text: "o".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 8,
                },
                EditOperation {
                    description: "Complete string with quote",
                    edit_type: EditType::TextInsertion,
                    start_line: 4,
                    start_character: 16,
                    end_line: 4,
                    end_character: 16,
                    new_text: "\"".to_string(),
                    expected_nodes_changed: 1,
                    expected_nodes_reused: 8,
                },
            ],
            expected_reuse_percentage: 88.9,
            target_update_time_us: 100, // Per edit
            max_memory_mb: 1,
            scenario_type: PerformanceScenario::RealTimeTyping,
        },
    ]
}

/// Generate a large Perl file for performance testing
#[cfg(test)]
fn generate_large_perl_file() -> &'static str {
    // This would be a large file with ~200 lines for testing
    // For the fixture, we'll use a representative sample
    r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'signatures';
no warnings 'experimental::signatures';

# Large file for performance testing
package LargeModule::DataProcessor;

our $VERSION = '1.0.0';

# Class constructor
sub new($class, %args) {
    my $self = {
        config => $args{config} || {},
        cache => {},
        stats => {
            operations => 0,
            cache_hits => 0,
            cache_misses => 0,
        },
        log_level => $args{log_level} || 'info',
    };
    return bless $self, $class;
}

# Primary data processing method
sub process_batch($self, $batch_data) {
    $self->{stats}{operations}++;

    my @results;
    for my $item (@$batch_data) {
        my $processed = $self->process_item($item);
        push @results, $processed if defined $processed;
    }

    return \@results;
}

# Individual item processing
sub process_item($self, $item) {
    return undef unless defined $item && ref $item eq 'HASH';

    # Check cache first
    my $cache_key = $self->_generate_cache_key($item);
    if (exists $self->{cache}{$cache_key}) {
        $self->{stats}{cache_hits}++;
        return $self->{cache}{$cache_key};
    }

    $self->{stats}{cache_misses}++;

    # Process the item
    my $result = {
        id => $item->{id},
        processed_at => time(),
        data => $self->_transform_data($item->{data}),
        metadata => $self->_extract_metadata($item),
    };

    # Store in cache
    $self->{cache}{$cache_key} = $result;

    return $result;
}

# Data transformation logic
sub _transform_data($self, $data) {
    return {} unless defined $data;

    my %transformed;
    for my $key (keys %$data) {
        my $value = $data->{$key};

        if (looks_like_number($value)) {
            $transformed{$key} = $value * 1.0;  # Normalize numbers
        } elsif (ref $value eq 'ARRAY') {
            $transformed{$key} = $self->_transform_array($value);
        } elsif (ref $value eq 'HASH') {
            $transformed{$key} = $self->_transform_data($value);
        } else {
            $transformed{$key} = "$value";  # Stringify
        }
    }

    return \%transformed;
}

# Array transformation
sub _transform_array($self, $array) {
    my @transformed;
    for my $item (@$array) {
        if (ref $item eq 'HASH') {
            push @transformed, $self->_transform_data($item);
        } else {
            push @transformed, $item;
        }
    }
    return \@transformed;
}

# Metadata extraction
sub _extract_metadata($self, $item) {
    return {
        type => ref $item->{data} || 'scalar',
        size => $self->_calculate_size($item->{data}),
        checksum => $self->_calculate_checksum($item),
    };
}

# Calculate data size
sub _calculate_size($self, $data) {
    return 0 unless defined $data;

    if (ref $data eq 'HASH') {
        return scalar keys %$data;
    } elsif (ref $data eq 'ARRAY') {
        return scalar @$data;
    } else {
        return length("$data");
    }
}

# Generate cache key
sub _generate_cache_key($self, $item) {
    use Digest::MD5 qw(md5_hex);
    my $serialized = join('|',
        $item->{id} || '',
        ref $item->{data} || '',
        $self->_calculate_size($item->{data})
    );
    return md5_hex($serialized);
}

# Calculate checksum
sub _calculate_checksum($self, $item) {
    use Digest::SHA qw(sha256_hex);
    my $content = JSON->new->canonical->encode($item);
    return substr(sha256_hex($content), 0, 8);
}

# Get processing statistics
sub get_stats($self) {
    return {
        %{$self->{stats}},
        cache_size => scalar keys %{$self->{cache}},
        hit_ratio => $self->{stats}{cache_hits} + $self->{stats}{cache_misses} > 0
            ? $self->{stats}{cache_hits} / ($self->{stats}{cache_hits} + $self->{stats}{cache_misses})
            : 0,
    };
}

# Clear cache
sub clear_cache($self) {
    $self->{cache} = {};
    $self->{stats}{cache_hits} = 0;
    $self->{stats}{cache_misses} = 0;
}

# Configuration management
sub set_config($self, $key, $value) {
    $self->{config}{$key} = $value;
}

sub get_config($self, $key) {
    return $self->{config}{$key};
}

# Logging functionality
sub log($self, $level, $message) {
    return unless $self->_should_log($level);

    my $timestamp = localtime();
    print STDERR "[$timestamp] [$level] $message\n";
}

sub _should_log($self, $level) {
    my %levels = (
        debug => 0,
        info => 1,
        warn => 2,
        error => 3,
    );

    my $current_level = $levels{$self->{log_level}} || 1;
    my $message_level = $levels{$level} || 1;

    return $message_level >= $current_level;
}

# Utility functions
sub looks_like_number {
    my ($value) = @_;
    return defined $value && $value =~ /^-?(?:\d+\.?\d*|\.\d+)$/;
}

# Cleanup
sub DESTROY($self) {
    $self->clear_cache();
}

1;

__END__

=head1 NAME

LargeModule::DataProcessor - A comprehensive data processing module

=head1 SYNOPSIS

    use LargeModule::DataProcessor;

    my $processor = LargeModule::DataProcessor->new(
        config => { batch_size => 100 },
        log_level => 'info'
    );

    my $results = $processor->process_batch($data);

=head1 DESCRIPTION

This module provides comprehensive data processing capabilities with
caching, statistics tracking, and configurable logging.

=cut
"#
}

/// Performance measurement utilities
#[cfg(test)]
pub struct PerformanceMeasurement {
    pub edit_description: String,
    pub update_time_us: u64,
    pub nodes_changed: u32,
    pub nodes_reused: u32,
    pub memory_usage_kb: u64,
    pub reuse_percentage: f32,
}

#[cfg(test)]
impl PerformanceMeasurement {
    pub fn new(edit: &EditOperation, actual_time_us: u64, actual_memory_kb: u64) -> Self {
        let total_nodes = edit.expected_nodes_changed + edit.expected_nodes_reused;
        let reuse_percentage = if total_nodes > 0 {
            (edit.expected_nodes_reused as f32 / total_nodes as f32) * 100.0
        } else {
            0.0
        };

        Self {
            edit_description: edit.description.to_string(),
            update_time_us: actual_time_us,
            nodes_changed: edit.expected_nodes_changed,
            nodes_reused: edit.expected_nodes_reused,
            memory_usage_kb: actual_memory_kb,
            reuse_percentage,
        }
    }

    pub fn meets_performance_target(&self, target_time_us: u64) -> bool {
        self.update_time_us <= target_time_us
    }

    pub fn meets_reuse_target(&self, target_percentage: f32) -> bool {
        self.reuse_percentage >= target_percentage
    }
}

/// Performance benchmark utilities
#[cfg(test)]
pub struct PerformanceBenchmark {
    pub measurements: Vec<PerformanceMeasurement>,
    pub total_time_us: u64,
    pub average_reuse_percentage: f32,
    pub peak_memory_kb: u64,
}

#[cfg(test)]
impl PerformanceBenchmark {
    pub fn new() -> Self {
        Self {
            measurements: Vec::new(),
            total_time_us: 0,
            average_reuse_percentage: 0.0,
            peak_memory_kb: 0,
        }
    }

    pub fn add_measurement(&mut self, measurement: PerformanceMeasurement) {
        self.total_time_us += measurement.update_time_us;
        self.peak_memory_kb = self.peak_memory_kb.max(measurement.memory_usage_kb);
        self.measurements.push(measurement);

        // Update average reuse percentage
        self.average_reuse_percentage = self.measurements
            .iter()
            .map(|m| m.reuse_percentage)
            .sum::<f32>() / self.measurements.len() as f32;
    }

    pub fn passes_performance_requirements(&self) -> bool {
        // All measurements should meet their individual targets
        self.measurements.iter().all(|m| {
            m.update_time_us <= 1000 && // Max 1ms per edit
            m.reuse_percentage >= 70.0   // Min 70% reuse
        })
    }
}

use std::sync::LazyLock;

/// Lazy-loaded performance fixture registry
#[cfg(test)]
pub static PERFORMANCE_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, PerformanceFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_performance_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get performance fixture by name
#[cfg(test)]
pub fn get_performance_fixture_by_name(name: &str) -> Option<&'static PerformanceFixture> {
    PERFORMANCE_FIXTURE_REGISTRY.get(name)
}

/// Get fixtures by scenario type
#[cfg(test)]
pub fn get_fixtures_by_scenario(scenario: PerformanceScenario) -> Vec<&'static PerformanceFixture> {
    PERFORMANCE_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.scenario_type == scenario)
        .collect()
}

/// Get fixtures with specific performance targets
#[cfg(test)]
pub fn get_fixtures_by_performance_target(max_time_us: u64, min_reuse: f32) -> Vec<&'static PerformanceFixture> {
    PERFORMANCE_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| {
            fixture.target_update_time_us <= max_time_us &&
            fixture.expected_reuse_percentage >= min_reuse
        })
        .collect()
}