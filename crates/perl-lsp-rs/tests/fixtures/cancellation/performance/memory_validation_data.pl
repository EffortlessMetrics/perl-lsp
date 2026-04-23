#!/usr/bin/perl
# Memory validation test fixture for LSP cancellation performance testing
# Generates realistic Perl code for memory overhead measurement scenarios

package MemoryValidation;
use strict;
use warnings;
use feature qw(say state);

# Small code sample for baseline memory measurement
sub small_memory_baseline {
    my ($input) = @_;

    my $result = process_simple($input);
    return $result;
}

sub process_simple {
    my ($data) = @_;
    return "processed_$data";
}

# Medium complexity code for memory scaling testing
sub medium_memory_scenario {
    my ($data_set) = @_;

    # Create moderate data structures for memory testing
    my @processed_items = map {
        my $item = $_;
        my $metadata = {
            id => $item->{id},
            timestamp => time(),
            processed => 1,
        };

        {
            original => $item,
            metadata => $metadata,
            processed_data => process_item_data($item),
        }
    } @$data_set;

    return \@processed_items;
}

sub process_item_data {
    my ($item) = @_;
    return {
        size => length($item->{content} // ""),
        hash => calculate_simple_hash($item),
    };
}

sub calculate_simple_hash {
    my ($item) = @_;
    return sprintf("%08x", abs(int(rand(0xFFFFFFFF))));
}

# Large memory scenario for stress testing cancellation overhead
sub large_memory_scenario {
    my ($dataset_size) = @_;

    # Generate larger data structures to test memory overhead
    my @large_dataset;
    for my $i (1..$dataset_size) {
        my $record = {
            id => $i,
            content => generate_content_block($i),
            metadata => generate_metadata($i),
            processing_history => generate_processing_history($i),
        };

        push @large_dataset, $record;
    }

    # Process the large dataset with potential for cancellation
    my $results = process_large_dataset(\@large_dataset);

    return {
        dataset_size => $dataset_size,
        records_processed => scalar(@large_dataset),
        results => $results,
    };
}

sub generate_content_block {
    my ($seed) = @_;

    my $content = "Content block $seed: ";
    $content .= "x" x (100 + ($seed % 50));  # Variable length content

    return $content;
}

sub generate_metadata {
    my ($seed) = @_;

    return {
        created_at => time() - ($seed * 60),
        priority => ($seed % 5) + 1,
        tags => [map { "tag_$_" } (1..($seed % 3 + 1))],
        attributes => {
            category => "category_" . ($seed % 10),
            weight => ($seed % 100) / 10.0,
            flags => {
                active => $seed % 2 == 0,
                validated => $seed % 3 == 0,
                indexed => $seed % 4 == 0,
            }
        }
    };
}

sub generate_processing_history {
    my ($seed) = @_;

    my @history;
    my $history_length = ($seed % 5) + 1;

    for my $i (1..$history_length) {
        push @history, {
            step => $i,
            operation => "operation_$i",
            timestamp => time() - (($history_length - $i + 1) * 30),
            status => $i == $history_length ? "completed" : "processed",
        };
    }

    return \@history;
}

sub process_large_dataset {
    my ($dataset) = @_;

    my $results = {
        total_items => scalar(@$dataset),
        processing_stats => {
            start_time => time(),
            memory_checkpoints => [],
        },
        processed_items => [],
    };

    # Process items with memory checkpoints for testing
    for my $i (0..$#$dataset) {
        my $item = $dataset->[$i];

        # Add memory checkpoint every 100 items
        if ($i % 100 == 0) {
            push @{$results->{processing_stats}->{memory_checkpoints}}, {
                item_index => $i,
                timestamp => time(),
                estimated_memory_kb => estimate_memory_usage($i),
            };
        }

        my $processed = {
            original_id => $item->{id},
            content_length => length($item->{content}),
            metadata_keys => scalar(keys %{$item->{metadata}}),
            history_length => scalar(@{$item->{processing_history}}),
            processed_at => time(),
        };

        push @{$results->{processed_items}}, $processed;
    }

    $results->{processing_stats}->{end_time} = time();
    $results->{processing_stats}->{duration} =
        $results->{processing_stats}->{end_time} - $results->{processing_stats}->{start_time};

    return $results;
}

sub estimate_memory_usage {
    my ($item_count) = @_;

    # Rough estimation for testing purposes
    my $base_memory = 50;  # KB base
    my $per_item_memory = 0.5;  # KB per item

    return int($base_memory + ($item_count * $per_item_memory));
}

# Memory-intensive regex operations for testing cancellation overhead
sub memory_intensive_regex_processing {
    my ($text_samples) = @_;

    my @results;
    for my $sample (@$text_samples) {
        # Complex regex operations that use memory
        my $processed = $sample;

        # Multiple substitution passes
        $processed =~ s/(\w+)_(\w+)_(\w+)/TRANSFORMED_$1_$2_$3/g;
        $processed =~ s/pattern_(\d+)_(\w+)/PATTERN_${1}_${2}_MATCHED/g;
        $processed =~ s/([A-Z]+)_([0-9]+)/CAPS_${1}_NUM_${2}/g;

        # Extract information with captures
        my @matches;
        while ($processed =~ /TRANSFORMED_(\w+)_(\w+)_(\w+)/g) {
            push @matches, {
                part1 => $1,
                part2 => $2,
                part3 => $3,
                position => $-[0],
            };
        }

        push @results, {
            original_length => length($sample),
            processed_length => length($processed),
            match_count => scalar(@matches),
            matches => \@matches,
        };
    }

    return \@results;
}

# State-based processing for testing cancellation with stateful operations
sub stateful_memory_processing {
    state $global_state = {
        processing_count => 0,
        total_memory_allocated => 0,
        active_contexts => [],
    };

    my ($processing_request) = @_;

    $global_state->{processing_count}++;

    # Allocate processing context
    my $context = {
        id => $global_state->{processing_count},
        request => $processing_request,
        allocated_memory => estimate_context_memory($processing_request),
        created_at => time(),
    };

    push @{$global_state->{active_contexts}}, $context;
    $global_state->{total_memory_allocated} += $context->{allocated_memory};

    # Process the request
    my $result = {
        context_id => $context->{id},
        processed_data => process_with_context($processing_request, $context),
        memory_stats => {
            context_memory => $context->{allocated_memory},
            total_allocated => $global_state->{total_memory_allocated},
            active_contexts => scalar(@{$global_state->{active_contexts}}),
        },
    };

    return $result;
}

sub estimate_context_memory {
    my ($request) = @_;

    my $base_memory = 10;  # KB
    my $complexity_factor = length($request->{content} // "") / 1000;

    return int($base_memory + $complexity_factor);
}

sub process_with_context {
    my ($request, $context) = @_;

    return {
        context_id => $context->{id},
        request_processed => 1,
        processing_time => time() - $context->{created_at},
    };
}

# Cleanup function for testing memory cleanup during cancellation
sub cleanup_memory_state {
    my ($context_id) = @_;

    state $global_state = {
        processing_count => 0,
        total_memory_allocated => 0,
        active_contexts => [],
    };

    # Find and remove context
    my @remaining_contexts;
    my $freed_memory = 0;

    for my $context (@{$global_state->{active_contexts}}) {
        if ($context->{id} == $context_id) {
            $freed_memory = $context->{allocated_memory};
        } else {
            push @remaining_contexts, $context;
        }
    }

    $global_state->{active_contexts} = \@remaining_contexts;
    $global_state->{total_memory_allocated} -= $freed_memory;

    return {
        freed_memory_kb => $freed_memory,
        remaining_contexts => scalar(@remaining_contexts),
        total_memory_allocated => $global_state->{total_memory_allocated},
    };
}

1;