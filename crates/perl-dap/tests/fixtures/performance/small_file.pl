#!/usr/bin/env perl
# Small performance test file (~100 lines)
# Expected: <50ms breakpoint validation
# Expected: <100ms step/continue operations

use strict;
use warnings;

# Global configuration
my $DEBUG = 1;
my $VERSION = '1.0.0';
my %CONFIG = (
    timeout => 30,
    retry_count => 3,
    log_level => 'info',
);

# Helper function for logging
sub log_message {
    my ($level, $message) = @_;
    print "[$level] $message\n" if $DEBUG;
    return 1;
}

# Process data with validation
sub process_data {
    my ($data) = @_;

    unless (defined $data) {
        log_message('error', 'No data provided');
        return undef;
    }

    if (ref($data) eq 'ARRAY') {
        return process_array($data);
    } elsif (ref($data) eq 'HASH') {
        return process_hash($data);
    } else {
        return process_scalar($data);
    }
}

# Process array data
sub process_array {
    my ($array) = @_;
    my @results;

    foreach my $item (@$array) {
        push @results, process_scalar($item);
    }

    return \@results;
}

# Process hash data
sub process_hash {
    my ($hash) = @_;
    my %results;

    foreach my $key (keys %$hash) {
        $results{$key} = process_scalar($hash->{$key});
    }

    return \%results;
}

# Process scalar data
sub process_scalar {
    my ($value) = @_;

    return uc($value) if defined $value;
    return '';
}

# Retry logic with exponential backoff
sub retry_operation {
    my ($operation, $max_retries) = @_;
    $max_retries //= $CONFIG{retry_count};

    my $attempt = 0;
    while ($attempt < $max_retries) {
        eval {
            return $operation->();
        };

        if ($@) {
            $attempt++;
            log_message('warn', "Attempt $attempt failed: $@");
            sleep(2 ** $attempt);
        } else {
            return 1;
        }
    }

    die "Operation failed after $max_retries attempts";
}

# Main execution
sub main {
    log_message('info', 'Starting application');

    my $data = ['foo', 'bar', 'baz'];
    my $result = process_data($data);

    log_message('info', 'Processing complete');
    return $result;
}

# Entry point
my $result = main();
print "Result: " . join(', ', @$result) . "\n";

exit 0;
