#!/usr/bin/env perl
# CPAN release automation patterns inspired by Dist::Zilla/App::Cmd workflows
# Purpose: exercise realistic production Perl involving Moo, roles, signatures,
# typed parameters, IPC/system integration, and rich regex/text processing.

use strict;
use warnings;
use v5.24;
use feature qw(signatures postderef);
no warnings qw(experimental::signatures experimental::postderef);

{
    package MyCPAN::Release::Config;

    use Moo;
    use Types::Standard qw(Str ArrayRef Bool Int);

    has dist_name   => (is => 'ro', isa => Str, required => 1);
    has release_tag => (is => 'ro', isa => Str, required => 1);
    has changelog   => (is => 'ro', isa => Str, default  => sub { 'Changes' });
    has smoke_jobs  => (is => 'ro', isa => ArrayRef[Str], default => sub { [qw(unit integration author)] });
    has dry_run     => (is => 'ro', isa => Bool, default => sub { 0 });
    has retries     => (is => 'ro', isa => Int, default  => sub { 2 });

    sub archive_name ($self) {
        return sprintf '%s-%s.tar.gz', $self->dist_name, $self->release_tag;
    }
}

{
    package MyCPAN::Release::Runner;

    use Moo;
    use Time::HiRes qw(sleep);
    use IPC::Open3 qw(open3);
    use Symbol qw(gensym);

    has config => (is => 'ro', required => 1);

    sub run ($self) {
        my @steps = (
            sub { $self->_assert_clean_git_tree },
            sub { $self->_verify_changelog },
            sub { $self->_run_smoke_jobs },
            sub { $self->_build_and_upload },
        );

        STEP:
        for my $step (@steps) {
            my $ok = eval { $step->(); 1 };
            next STEP if $ok;
            die "release pipeline failed: $@";
        }

        return {
            status  => 'ok',
            archive => $self->config->archive_name,
            jobs    => [ $self->config->smoke_jobs->@* ],
        };
    }

    sub _assert_clean_git_tree ($self) {
        my ($code, $stdout) = $self->_run_command(qw(git status --porcelain));
        die "git status failed" if $code != 0;
        die "working tree dirty: $stdout" if $stdout =~ /\S/;
        return 1;
    }

    sub _verify_changelog ($self) {
        open my $fh, '<', $self->config->changelog
          or die "unable to open changelog: $!";

        my $found_release_line = 0;
        while (my $line = <$fh>) {
            if ($line =~ /^\Q$self->{config}->{release_tag}\E\s+-\s+\d{4}-\d{2}-\d{2}$/) {
                $found_release_line = 1;
                last;
            }
        }

        close $fh;
        die "missing release entry in Changes" unless $found_release_line;
        return 1;
    }

    sub _run_smoke_jobs ($self) {
        JOB:
        for my $job ($self->config->smoke_jobs->@*) {
            my $attempt = 0;
            RETRY:
            while ($attempt <= $self->config->retries) {
                my ($code) = $self->_run_command('prove', '-lv', "t/$job.t");
                return 1 if $code == 0;

                $attempt++;
                if ($attempt <= $self->config->retries) {
                    sleep 0.05 * $attempt;
                    next RETRY;
                }

                die "smoke job failed: $job";
            }

            next JOB;
        }

        return 1;
    }

    sub _build_and_upload ($self) {
        my @cmd = $self->config->dry_run
          ? qw(dzil build --no-tgz)
          : qw(dzil release --trial);

        my ($code, $stdout, $stderr) = $self->_run_command(@cmd);
        die "release command failed: $stderr" if $code != 0;

        if ($stdout =~ /(Uploading|Released)\s+\Q@{[$self->config->archive_name]}\E/x) {
            return 1;
        }

        die "expected archive confirmation not found";
    }

    sub _run_command ($self, @cmd) {
        my $stderr = gensym;
        my $pid = open3(my $in, my $out, $stderr, @cmd);
        close $in;

        my $stdout = do { local $/; <$out> // '' };
        my $errout = do { local $/; <$stderr> // '' };

        waitpid $pid, 0;
        my $exit = $? >> 8;

        return wantarray ? ($exit, $stdout, $errout) : $exit;
    }
}

package main;

use Getopt::Long qw(GetOptionsFromArray);

my $dry_run = 0;
my $tag     = '0.10.0';
my $dist    = 'My-CPAN-Dist';

GetOptionsFromArray(
    \@ARGV,
    'dry-run!' => \$dry_run,
    'tag=s'    => \$tag,
    'dist=s'   => \$dist,
);

my $config = MyCPAN::Release::Config->new(
    dist_name   => $dist,
    release_tag => $tag,
    dry_run     => $dry_run,
);

my $runner = MyCPAN::Release::Runner->new(config => $config);
my $result = eval { $runner->run };

if (my $err = $@) {
    warn "release failed: $err";
    exit 1;
}

print "release status=$result->{status} archive=$result->{archive}\n";

__DATA__
# Example changelog format accepted by _verify_changelog
0.10.0 - 2026-01-31
- Added async index cache warmup
- Improved parser diagnostics for quote-like operators
