package MultiFileProject::Utils;
# Utils module fixture for cross-file navigation cancellation testing
# Tests bare function calls and qualified Package::function dual indexing

use strict;
use warnings;
use Exporter qw(import);
use List::Util qw(first max min sum uniq);
use File::Spec;
use JSON::PP;

our $VERSION = '1.0.0';
our @EXPORT_OK = qw(
    process_complex_data
    validate_database_config
    log_schema_operation
    validate_pooled_connection
);

# Test cancellation during complex data processing utilities
sub initialize_utilities {
    my ($config) = @_;

    # Test cancellation during utility initialization
    my $processors = setup_data_processors($config->{processors});
    my $validators = setup_data_validators($config->{validators});
    my $formatters = setup_data_formatters($config->{formatters});

    return {
        processors => $processors,
        validators => $validators,
        formatters => $formatters,
        initialized => 1,
    };
}

# Test cancellation during complex data processing
sub process_complex_data {
    my ($data) = @_;

    # Test cancellation during data transformation pipeline
    my $normalized = normalize_input_data($data);
    my $validated = validate_data_structure($normalized);
    my $enriched = enrich_data_content($validated);
    my $formatted = format_output_data($enriched);

    return {
        original => $data,
        normalized => $normalized,
        validated => $validated,
        enriched => $enriched,
        formatted => $formatted,
    };
}

# Test cancellation during database configuration validation
sub validate_database_config {
    my ($config) = @_;

    # Test cancellation during configuration validation
    my $required_fields = check_required_fields($config);
    my $connection_params = validate_connection_parameters($config);
    my $security_settings = validate_security_settings($config);

    return {
        config => $config,
        required_fields => $required_fields,
        connection_params => $connection_params,
        security => $security_settings,
        valid => 1,
    };
}

# Test cancellation during pooled connection validation
sub validate_pooled_connection {
    my ($connection) = @_;

    # Test cancellation during connection health checks
    my $health_check = perform_connection_health_check($connection);
    my $performance_test = run_connection_performance_test($connection);
    my $security_audit = audit_connection_security($connection);

    return {
        connection => $connection,
        health => $health_check,
        performance => $performance_test,
        security => $security_audit,
        validated => 1,
    };
}

# Test cancellation during schema operation logging
sub log_schema_operation {
    my ($operation, $result) = @_;

    # Test cancellation during log processing
    my $timestamp = generate_log_timestamp();
    my $log_entry = format_log_entry($operation, $result, $timestamp);
    my $written = write_log_entry($log_entry);

    return {
        operation => $operation,
        result => $result,
        timestamp => $timestamp,
        log_entry => $log_entry,
        written => $written,
    };
}

# Test cancellation during utility configuration retrieval
sub get_query_utilities {
    my $utilities = {
        query_builder => setup_query_builder(),
        result_processor => setup_result_processor(),
        cache_manager => setup_cache_manager(),
    };

    return $utilities;
}

# Test cancellation during utility data processing
sub process_utility_data {
    my ($data) = @_;

    # Test cancellation during utility processing pipeline
    my @processed_items = map {
        my $item = $_;
        my $validated = validate_utility_item($item);
        my $transformed = transform_utility_item($validated);
        my $cached = cache_utility_result($transformed);
        $cached;
    } @$data;

    return \@processed_items;
}

# Helper functions for realistic utility scenarios
sub setup_data_processors {
    my ($config) = @_;
    return {
        text_processor => { enabled => 1, config => $config->{text} },
        number_processor => { enabled => 1, config => $config->{numbers} },
        date_processor => { enabled => 1, config => $config->{dates} },
    };
}

sub setup_data_validators {
    my ($config) = @_;
    return {
        schema_validator => { rules => $config->{schema_rules} },
        type_validator => { types => $config->{allowed_types} },
        range_validator => { ranges => $config->{value_ranges} },
    };
}

sub setup_data_formatters {
    my ($config) = @_;
    return {
        json_formatter => { pretty => $config->{pretty_json} },
        csv_formatter => { delimiter => $config->{csv_delimiter} },
        xml_formatter => { indent => $config->{xml_indent} },
    };
}

sub normalize_input_data {
    my ($data) = @_;
    return { %$data, normalized => 1, timestamp => time() };
}

sub validate_data_structure {
    my ($data) = @_;
    return { %$data, structure_valid => 1 };
}

sub enrich_data_content {
    my ($data) = @_;
    return {
        %$data,
        enriched => 1,
        metadata => {
            enrichment_timestamp => time(),
            enrichment_version => "1.0",
        }
    };
}

sub format_output_data {
    my ($data) = @_;
    return {
        %$data,
        formatted => 1,
        output_format => "json",
    };
}

sub check_required_fields {
    my ($config) = @_;
    my @required = qw(host port database username);
    my @missing = grep { !exists $config->{$_} } @required;
    return {
        required => \@required,
        missing => \@missing,
        valid => scalar(@missing) == 0,
    };
}

sub validate_connection_parameters {
    my ($config) = @_;
    return {
        host_valid => defined($config->{host}) && length($config->{host}) > 0,
        port_valid => defined($config->{port}) && $config->{port} =~ /^\d+$/,
        timeout_valid => !defined($config->{timeout}) || $config->{timeout} > 0,
    };
}

sub validate_security_settings {
    my ($config) = @_;
    return {
        ssl_enabled => $config->{ssl} // 0,
        auth_method => $config->{auth_method} // "password",
        encryption => $config->{encryption} // "none",
    };
}

sub perform_connection_health_check {
    my ($connection) = @_;
    return {
        status => "healthy",
        response_time_ms => 15,
        last_checked => time(),
    };
}

sub run_connection_performance_test {
    my ($connection) = @_;
    return {
        avg_query_time_ms => 23,
        max_concurrent_queries => 10,
        throughput_qps => 150,
    };
}

sub audit_connection_security {
    my ($connection) = @_;
    return {
        encrypted => 1,
        certificate_valid => 1,
        auth_verified => 1,
    };
}

sub generate_log_timestamp {
    my ($sec, $min, $hour, $mday, $mon, $year) = localtime(time());
    return sprintf("%04d-%02d-%02d %02d:%02d:%02d",
                   $year + 1900, $mon + 1, $mday, $hour, $min, $sec);
}

sub format_log_entry {
    my ($operation, $result, $timestamp) = @_;
    return {
        timestamp => $timestamp,
        operation => $operation,
        result => $result,
        level => "INFO",
    };
}

sub write_log_entry {
    my ($entry) = @_;
    # Simulate log writing
    return { written => 1, entry_id => time() };
}

sub setup_query_builder {
    return { type => "sql_builder", version => "1.0" };
}

sub setup_result_processor {
    return { type => "result_processor", version => "1.0" };
}

sub setup_cache_manager {
    return { type => "cache_manager", version => "1.0" };
}

sub validate_utility_item {
    my ($item) = @_;
    return { %$item, validated => 1 };
}

sub transform_utility_item {
    my ($item) = @_;
    return { %$item, transformed => 1 };
}

sub cache_utility_result {
    my ($item) = @_;
    return { %$item, cached => 1, cache_key => "key_" . time() };
}

1;