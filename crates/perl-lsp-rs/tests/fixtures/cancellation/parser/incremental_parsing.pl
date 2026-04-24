#!/usr/bin/perl
# Incremental parsing test fixture for LSP cancellation scenarios
# Tests cancellation during complex function parsing with nested constructs

package IncrementalParsing;
use strict;
use warnings;
use feature 'say';

# Enhanced builtin function scenarios for cancellation testing
sub complex_map_processing {
    my ($data_ref) = @_;

    # Test cancellation during map with {} block parsing
    my @processed = map {
        my $item = $_;
        my $result = process_complex_item($item);
        transform_result($result);
    } @$data_ref;

    return \@processed;
}

sub nested_grep_filter {
    my ($items, $criteria) = @_;

    # Test cancellation during nested grep operations
    my @filtered = grep {
        my $item = $_;
        my $matches = grep {
            my $criterion = $_;
            check_criterion($item, $criterion);
        } @$criteria;

        scalar(@$matches) > 0;
    } @$items;

    return \@filtered;
}

sub sort_with_complex_comparator {
    my ($items) = @_;

    # Test cancellation during sort with complex comparison
    my @sorted = sort {
        my $result_a = calculate_sort_key($a);
        my $result_b = calculate_sort_key($b);

        # Complex multi-field comparison for cancellation testing
        $result_a->{priority} <=> $result_b->{priority} ||
        $result_a->{timestamp} <=> $result_b->{timestamp} ||
        $result_a->{name} cmp $result_b->{name};
    } @$items;

    return \@sorted;
}

# Substitution operator test cases for comprehensive parsing
sub substitution_with_balanced_delimiters {
    my ($text) = @_;

    # Test cancellation during s{}{} parsing
    $text =~ s{pattern_(\w+)_end}{replacement_$1_done}g;

    # Test cancellation during s[][] parsing
    $text =~ s[old_(\d+)_value][new_$1_result]g;

    # Test cancellation during s<> parsing
    $text =~ s<begin_(\w+)_here><start_$1_there>g;

    return $text;
}

sub substitution_with_alternative_delimiters {
    my ($content) = @_;

    # Test cancellation during s/// parsing
    $content =~ s/search_(\w+)/replace_$1/g;

    # Test cancellation during s### parsing
    $content =~ s#path/(\w+)#new_path/$1#g;

    # Test cancellation during s||| parsing
    $content =~ s|old_(\w+)_pattern|new_$1_format|g;

    # Test single-quote substitution delimiters
    $content =~ s'literal_text'replacement_text'g;

    return $content;
}

# Unicode identifier and emoji support for cancellation testing
sub process_unicode_identifiers {
    my ($データ) = @_;

    my $結果 = {};
    my $🔧_tool = "processing";
    my $📊_data = $データ;

    # Test cancellation during unicode processing
    for my $項目 (keys %$📊_data) {
        my $処理済み = transform_unicode_item($項目);
        $結果->{$項目} = $処理済み;
    }

    return $結果;
}

# Cross-file reference scenarios for dual indexing cancellation
sub call_external_functions {
    my ($param) = @_;

    # Test cancellation during Package::function resolution
    my $db_result = Database::connect();
    my $util_data = Utils::process_data($param);

    # Test cancellation during bare function resolution
    my $processed = process_data($param);
    my $connected = connect();

    return {
        db => $db_result,
        utils => $util_data,
        processed => $processed,
        connected => $connected,
    };
}

# Complex nested structures for incremental parsing cancellation
sub deeply_nested_structure {
    my ($level) = @_;

    return {
        metadata => {
            level => $level,
            timestamp => time(),
            nested => {
                inner => {
                    data => {
                        items => [
                            map {
                                {
                                    id => $_,
                                    processed => process_item($_),
                                    children => [
                                        map {
                                            {
                                                child_id => $_,
                                                value => calculate_value($_),
                                            }
                                        } (1..5)
                                    ]
                                }
                            } (1..10)
                        ]
                    }
                }
            }
        }
    };
}

# Helper functions for realistic parsing scenarios
sub process_complex_item {
    my ($item) = @_;
    return { processed => $item, timestamp => time() };
}

sub transform_result {
    my ($result) = @_;
    return $result->{processed} . "_transformed";
}

sub check_criterion {
    my ($item, $criterion) = @_;
    return $item =~ /$criterion/;
}

sub calculate_sort_key {
    my ($item) = @_;
    return {
        priority => $item->{priority} // 0,
        timestamp => $item->{timestamp} // time(),
        name => $item->{name} // "default",
    };
}

sub transform_unicode_item {
    my ($item) = @_;
    return "processed_$item";
}

sub process_item {
    my ($item) = @_;
    return "processed_$item";
}

sub calculate_value {
    my ($input) = @_;
    return $input * 2;
}

1;