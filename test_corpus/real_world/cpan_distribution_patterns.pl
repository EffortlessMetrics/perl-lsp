#!/usr/bin/env perl
# Real-world CPAN distribution patterns inspired by App::*, Dist::Zilla, and Moo-based modules.

use strict;
use warnings;
use v5.20;
use feature 'signatures';
no warnings 'experimental::signatures';

{
    package My::App::Config;

    use Exporter 'import';
    our @EXPORT_OK = qw(load_config merge_env expand_path);

    use File::Spec;
    use Cwd qw(abs_path);
    use Scalar::Util qw(looks_like_number);

    use constant DEFAULT_TIMEOUT => 30;
    use constant DEFAULT_RETRIES => 3;

    sub load_config ($class, $path = 'myapp.ini') {
        state %CACHE;
        return $CACHE{$path} if exists $CACHE{$path};

        my %config = (
            app_name => 'my-app',
            timeout  => DEFAULT_TIMEOUT,
            retries  => DEFAULT_RETRIES,
            plugins  => [qw(Core Metrics Notify)],
        );

        if (-e $path) {
            open my $fh, '<', $path or die "open($path): $!";
            my $section = 'default';

            while (my $line = <$fh>) {
                chomp $line;
                next if $line =~ /^\s*(?:#|;|$)/;

                if ($line =~ /^\s*\[(.+?)\]\s*$/) {
                    $section = $1;
                    next;
                }

                if ($line =~ /^\s*([A-Za-z_][\w\-]*)\s*=\s*(.*?)\s*$/) {
                    my ($key, $value) = ($1, $2);

                    if ($key =~ /^(timeout|retries)$/ && looks_like_number($value)) {
                        $config{$key} = 0 + $value;
                    }
                    elsif ($key eq 'plugins') {
                        my @plugins = grep { length } map { s/^\s+|\s+$//gr } split /,/, $value;
                        $config{$key} = \@plugins;
                    }
                    else {
                        $config{"${section}.${key}"} = $value;
                    }
                }
            }

            close $fh;
        }

        return $CACHE{$path} = \%config;
    }

    sub merge_env ($config) {
        my %env_map = (
            timeout => 'MY_APP_TIMEOUT',
            retries => 'MY_APP_RETRIES',
            log     => 'MY_APP_LOG_LEVEL',
        );

        for my $key (keys %env_map) {
            my $env_key = $env_map{$key};
            next unless exists $ENV{$env_key};
            $config->{$key} = $ENV{$env_key};
        }

        return $config;
    }

    sub expand_path ($base, $child) {
        my $joined = File::Spec->catfile($base, $child);
        my $abs = abs_path($joined // $child);
        return $abs // $joined;
    }
}

{
    package My::App::Role::Logging;

    use Moo::Role;
    requires 'log_target';

    around log_message => sub ($orig, $self, $level, $message) {
        my $prefix = sprintf '[%s][%s]', scalar(localtime), uc($level // 'info');
        return $self->$orig($level, "$prefix $message");
    };
}

{
    package My::App::Command::Run;

    use Moo;
    use Time::HiRes qw(time sleep);

    with 'My::App::Role::Logging';

    has config => (is => 'ro', required => 1);
    has tasks  => (is => 'rw', default => sub { [] });
    has stats  => (is => 'rw', default => sub { { ok => 0, failed => 0, elapsed => 0 } });

    sub log_target ($self) { return *STDERR; }

    sub log_message ($self, $level, $message) {
        my $fh = $self->log_target;
        print {$fh} "$message\n";
        return 1;
    }

    sub run ($self) {
        my $start = time;
        local $SIG{__WARN__} = sub ($warning) {
            $self->log_message('warn', "runtime warning: $warning");
        };

        TASK:
        for my $task (@{ $self->tasks }) {
            my ($name, $code) = @{$task}{qw(name code)};
            next TASK unless $name && ref($code) eq 'CODE';

            my $ok = eval {
                $self->log_message('info', "starting $name");
                $code->($self->config);
                1;
            };

            if ($ok) {
                $self->stats->{ok}++;
                next TASK;
            }

            $self->stats->{failed}++;
            my $error = $@ || 'unknown error';
            $self->log_message('error', "task $name failed: $error");
            last TASK if $self->config->{fail_fast};
        }

        $self->stats->{elapsed} = time - $start;
        return $self->stats;
    }
}

1;

__DATA__
[defaults]
timeout=15
retries=2
plugins=Core,Metrics,Notify,Pager
