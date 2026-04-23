#!/usr/bin/env perl
# Medium performance test file (~1000 lines)
# Expected: <50ms breakpoint validation
# Expected: <100ms step/continue operations

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/lib";

# Package: DataProcessor
package DataProcessor {
    use strict;
    use warnings;

    sub new {
        my ($class, %args) = @_;
        my $self = {
            config => $args{config} || {},
            stats => {
                processed => 0,
                errors => 0,
                warnings => 0,
            },
            cache => {},
        };
        return bless $self, $class;
    }

    sub process {
        my ($self, $data) = @_;
        $self->{stats}{processed}++;

        return $self->validate($data)
            && $self->transform($data)
            && $self->store($data);
    }

    sub validate {
        my ($self, $data) = @_;

        unless (defined $data) {
            $self->{stats}{errors}++;
            return 0;
        }

        if (ref($data) eq 'HASH') {
            return $self->validate_hash($data);
        } elsif (ref($data) eq 'ARRAY') {
            return $self->validate_array($data);
        }

        return 1;
    }

    sub validate_hash {
        my ($self, $hash) = @_;

        foreach my $key (keys %$hash) {
            unless ($self->validate_key($key)) {
                $self->{stats}{warnings}++;
                return 0;
            }
        }

        return 1;
    }

    sub validate_array {
        my ($self, $array) = @_;

        foreach my $item (@$array) {
            unless ($self->validate($item)) {
                return 0;
            }
        }

        return 1;
    }

    sub validate_key {
        my ($self, $key) = @_;
        return $key =~ /^[a-zA-Z_][a-zA-Z0-9_]*$/;
    }

    sub transform {
        my ($self, $data) = @_;

        if (ref($data) eq 'HASH') {
            return $self->transform_hash($data);
        } elsif (ref($data) eq 'ARRAY') {
            return $self->transform_array($data);
        }

        return $self->transform_scalar($data);
    }

    sub transform_hash {
        my ($self, $hash) = @_;
        my %result;

        foreach my $key (keys %$hash) {
            $result{$key} = $self->transform($hash->{$key});
        }

        return \%result;
    }

    sub transform_array {
        my ($self, $array) = @_;
        my @result;

        foreach my $item (@$array) {
            push @result, $self->transform($item);
        }

        return \@result;
    }

    sub transform_scalar {
        my ($self, $value) = @_;
        return uc($value) if defined $value;
        return '';
    }

    sub store {
        my ($self, $data) = @_;
        my $key = $self->generate_key($data);
        $self->{cache}{$key} = $data;
        return 1;
    }

    sub generate_key {
        my ($self, $data) = @_;
        return sprintf("%d", time());
    }

    sub get_stats {
        my ($self) = @_;
        return $self->{stats};
    }
}

# Package: Logger
package Logger {
    use strict;
    use warnings;

    my %LEVELS = (
        debug => 0,
        info => 1,
        warn => 2,
        error => 3,
    );

    sub new {
        my ($class, %args) = @_;
        my $self = {
            level => $args{level} || 'info',
            output => $args{output} || \*STDERR,
        };
        return bless $self, $class;
    }

    sub log {
        my ($self, $level, $message) = @_;

        return unless $self->should_log($level);

        my $fh = $self->{output};
        print $fh $self->format_message($level, $message);
    }

    sub should_log {
        my ($self, $level) = @_;
        return $LEVELS{$level} >= $LEVELS{$self->{level}};
    }

    sub format_message {
        my ($self, $level, $message) = @_;
        my $timestamp = localtime();
        return "[$timestamp] [$level] $message\n";
    }

    sub debug { shift->log('debug', @_) }
    sub info  { shift->log('info', @_) }
    sub warn  { shift->log('warn', @_) }
    sub error { shift->log('error', @_) }
}

# Package: ConfigReader
package ConfigReader {
    use strict;
    use warnings;

    sub new {
        my ($class, %args) = @_;
        my $self = {
            file => $args{file},
            data => {},
        };
        return bless $self, $class;
    }

    sub load {
        my ($self) = @_;

        return 1 unless $self->{file};

        open(my $fh, '<', $self->{file}) or return 0;

        while (my $line = <$fh>) {
            chomp $line;
            next if $line =~ /^\s*#/;
            next if $line =~ /^\s*$/;

            if ($line =~ /^(\w+)\s*=\s*(.+)$/) {
                my ($key, $value) = ($1, $2);
                $self->{data}{$key} = $value;
            }
        }

        close $fh;
        return 1;
    }

    sub get {
        my ($self, $key) = @_;
        return $self->{data}{$key};
    }

    sub set {
        my ($self, $key, $value) = @_;
        $self->{data}{$key} = $value;
    }
}

# Package: Application
package Application {
    use strict;
    use warnings;

    sub new {
        my ($class, %args) = @_;

        my $self = {
            processor => DataProcessor->new(config => $args{config}),
            logger => Logger->new(level => $args{log_level} || 'info'),
            config_reader => ConfigReader->new(file => $args{config_file}),
        };

        return bless $self, $class;
    }

    sub initialize {
        my ($self) = @_;

        $self->{logger}->info('Initializing application');

        if ($self->{config_reader}->load()) {
            $self->{logger}->info('Configuration loaded');
        } else {
            $self->{logger}->warn('Could not load configuration');
        }

        return 1;
    }

    sub run {
        my ($self, $data) = @_;

        $self->{logger}->info('Running application');

        my @results;
        foreach my $item (@$data) {
            if ($self->{processor}->process($item)) {
                push @results, $item;
            } else {
                $self->{logger}->error("Failed to process item");
            }
        }

        $self->report_stats();

        return \@results;
    }

    sub report_stats {
        my ($self) = @_;

        my $stats = $self->{processor}->get_stats();
        $self->{logger}->info("Processed: $stats->{processed}");
        $self->{logger}->info("Errors: $stats->{errors}");
        $self->{logger}->info("Warnings: $stats->{warnings}");
    }

    sub shutdown {
        my ($self) = @_;
        $self->{logger}->info('Shutting down application');
    }
}

# Main execution
package main;

sub main {
    my $app = Application->new(
        log_level => 'debug',
        config => {
            timeout => 30,
            retry_count => 3,
        },
    );

    $app->initialize();

    my $data = [
        { name => 'item1', value => 100 },
        { name => 'item2', value => 200 },
        { name => 'item3', value => 300 },
        { name => 'item4', value => 400 },
        { name => 'item5', value => 500 },
    ];

    my $results = $app->run($data);

    print "Processed " . scalar(@$results) . " items\n";

    $app->shutdown();

    return 0;
}

exit main();
