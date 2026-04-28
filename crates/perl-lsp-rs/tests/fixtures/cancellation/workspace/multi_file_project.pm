package MultiFileProject::Core;
# Multi-file project fixture for cross-file navigation cancellation testing
# Tests dual indexing pattern with Package::function and bare function calls

use strict;
use warnings;
use MultiFileProject::Database;
use MultiFileProject::Utils;
use MultiFileProject::Config;

# Test cancellation during cross-file Package::function resolution
sub initialize_project {
    my ($config_file) = @_;

    # Test qualified function call cancellation
    my $config = MultiFileProject::Config::load_configuration($config_file);
    my $db = MultiFileProject::Database::establish_connection($config->{database});
    my $utils = MultiFileProject::Utils::initialize_utilities($config->{utils});

    # Test bare function call cancellation with dual indexing
    my $validated_config = validate_configuration($config);
    my $initialized_db = initialize_database($db);
    my $setup_utils = setup_utilities($utils);

    return {
        config => $validated_config,
        database => $initialized_db,
        utils => $setup_utils,
    };
}

# Test cancellation during workspace symbol resolution
sub process_project_data {
    my ($data) = @_;

    # Test cancellation during qualified method calls
    my $processed_data = MultiFileProject::Utils::process_complex_data($data);
    my $stored_result = MultiFileProject::Database::store_processed_data($processed_data);

    # Test cancellation during bare function calls
    my $validated = validate_data($data);
    my $transformed = transform_data($validated);
    my $finalized = finalize_processing($transformed);

    return {
        processed => $processed_data,
        stored => $stored_result,
        validated => $validated,
        transformed => $transformed,
        finalized => $finalized,
    };
}

# Test cancellation during import resolution and optimization
sub manage_project_imports {
    my ($module_path) = @_;

    # Used imports for cancellation testing
    use File::Spec;
    use JSON::PP qw(decode_json encode_json);
    use List::Util qw(first max min sum);

    # Unused imports for import optimization cancellation testing
    use Data::Dumper;  # Unused - should be detected during cancellation
    use Time::HiRes;   # Unused - should be detected during cancellation
    use Carp qw(croak confess);  # Unused - should be detected during cancellation

    # Test cancellation during import usage analysis
    my $spec = File::Spec->catfile($module_path, "config.json");
    my $json_data = decode_json(read_file($spec));
    my $max_value = max(@{$json_data->{values}});

    return {
        spec => $spec,
        data => $json_data,
        max => $max_value,
    };
}

# Test cancellation during subroutine reference resolution
sub analyze_function_references {
    my ($target_function) = @_;

    # Direct function calls for reference analysis cancellation
    my $result1 = process_target_data($target_function);
    my $result2 = MultiFileProject::Database::query_function_data($target_function);

    # Function references for cancellation testing
    my $func_ref = \&process_target_data;
    my $qualified_ref = \&MultiFileProject::Utils::process_utility_data;

    # Test cancellation during coderef invocation
    my $invoked1 = $func_ref->($target_function);
    my $invoked2 = $qualified_ref->($target_function);

    return {
        direct1 => $result1,
        direct2 => $result2,
        ref1 => $invoked1,
        ref2 => $invoked2,
    };
}

# Helper functions for dual indexing pattern testing
sub validate_configuration {
    my ($config) = @_;
    return { %$config, validated => 1 };
}

sub initialize_database {
    my ($db_handle) = @_;
    return { handle => $db_handle, initialized => 1 };
}

sub setup_utilities {
    my ($utils) = @_;
    return { utils => $utils, setup => 1 };
}

sub validate_data {
    my ($data) = @_;
    return { %$data, validation_passed => 1 };
}

sub transform_data {
    my ($data) = @_;
    return { %$data, transformed => 1 };
}

sub finalize_processing {
    my ($data) = @_;
    return { %$data, finalized => 1 };
}

sub process_target_data {
    my ($target) = @_;
    return "processed_$target";
}

sub read_file {
    my ($file) = @_;
    return '{"values": [1, 2, 3, 4, 5]}';
}

1;