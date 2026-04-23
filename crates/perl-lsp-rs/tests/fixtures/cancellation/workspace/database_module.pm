package MultiFileProject::Database;
# Database module fixture for cross-file navigation cancellation testing
# Tests dual indexing with qualified Package::function resolution

use strict;
use warnings;
use DBI;
use MultiFileProject::Config qw(get_database_config);
use MultiFileProject::Utils;

our $VERSION = '1.0.0';

# Test cancellation during database connection establishment
sub establish_connection {
    my ($config) = @_;

    # Test qualified function call cancellation
    my $db_config = MultiFileProject::Config::get_database_config($config);
    my $validated = MultiFileProject::Utils::validate_database_config($db_config);

    # Test bare function call cancellation
    my $connection_string = build_connection_string($validated);
    my $connection = create_database_handle($connection_string);

    return {
        config => $db_config,
        connection_string => $connection_string,
        handle => $connection,
        established => 1,
    };
}

# Test cancellation during complex database operations
sub store_processed_data {
    my ($data) = @_;

    # Test cancellation during transaction processing
    my $transaction = begin_transaction();
    my $prepared_data = prepare_data_for_storage($data);
    my $insert_result = execute_insert_statement($prepared_data);
    my $commit_result = commit_transaction($transaction);

    return {
        transaction_id => $transaction,
        prepared => $prepared_data,
        inserted => $insert_result,
        committed => $commit_result,
    };
}

# Test cancellation during query function data resolution
sub query_function_data {
    my ($function_name) = @_;

    # Test qualified cross-module calls during cancellation
    my $config = MultiFileProject::Config::get_query_config();
    my $utils = MultiFileProject::Utils::get_query_utilities();

    # Test bare function calls during cancellation
    my $query = build_function_query($function_name);
    my $results = execute_query($query);
    my $processed = process_query_results($results);

    return {
        query => $query,
        raw_results => $results,
        processed_results => $processed,
    };
}

# Test cancellation during database schema operations
sub manage_database_schema {
    my ($schema_operations) = @_;

    # Test cancellation during schema creation
    for my $operation (@$schema_operations) {
        my $validated = validate_schema_operation($operation);
        my $sql = generate_schema_sql($validated);
        my $executed = execute_schema_operation($sql);

        # Test cross-module logging during cancellation
        MultiFileProject::Utils::log_schema_operation($operation, $executed);
    }

    return { schema_updated => 1 };
}

# Test cancellation during connection pool management
sub manage_connection_pool {
    my ($pool_config) = @_;

    my @connections;
    for my $i (1..$pool_config->{size}) {
        # Test cancellation during pool initialization
        my $conn_config = prepare_connection_config($pool_config, $i);
        my $connection = establish_pooled_connection($conn_config);

        # Test qualified utility calls during cancellation
        my $validated = MultiFileProject::Utils::validate_pooled_connection($connection);
        push @connections, $validated;
    }

    return {
        pool_size => scalar(@connections),
        connections => \@connections,
        initialized => 1,
    };
}

# Helper functions for realistic database scenarios
sub build_connection_string {
    my ($config) = @_;
    return "dbi:SQLite:dbname=$config->{database_file}";
}

sub create_database_handle {
    my ($connection_string) = @_;
    return { connection_string => $connection_string, connected => 1 };
}

sub begin_transaction {
    return "transaction_" . time();
}

sub prepare_data_for_storage {
    my ($data) = @_;
    return { %$data, prepared_timestamp => time() };
}

sub execute_insert_statement {
    my ($data) = @_;
    return { success => 1, rows_affected => 1 };
}

sub commit_transaction {
    my ($transaction_id) = @_;
    return { transaction_id => $transaction_id, committed => 1 };
}

sub build_function_query {
    my ($function_name) = @_;
    return "SELECT * FROM functions WHERE name = '$function_name'";
}

sub execute_query {
    my ($query) = @_;
    return [
        { id => 1, name => "test_function", module => "TestModule" },
        { id => 2, name => "another_function", module => "AnotherModule" },
    ];
}

sub process_query_results {
    my ($results) = @_;
    return [
        map { { %$_, processed => 1 } } @$results
    ];
}

sub validate_schema_operation {
    my ($operation) = @_;
    return { %$operation, validated => 1 };
}

sub generate_schema_sql {
    my ($operation) = @_;
    return "CREATE TABLE $operation->{table} (id INTEGER PRIMARY KEY)";
}

sub execute_schema_operation {
    my ($sql) = @_;
    return { sql => $sql, executed => 1 };
}

sub prepare_connection_config {
    my ($pool_config, $index) = @_;
    return {
        %$pool_config,
        connection_id => "conn_$index",
        timeout => $pool_config->{timeout} || 30,
    };
}

sub establish_pooled_connection {
    my ($config) = @_;
    return {
        id => $config->{connection_id},
        established => time(),
        config => $config,
    };
}

1;