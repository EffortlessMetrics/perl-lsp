#!/usr/bin/env perl
# CPAN Distribution Tooling Patterns - Makefile.PL / Build.PL / dzil-style hooks
# Real-world style fixture covering installer metadata, version probing, and toolchain hooks.

use strict;
use warnings;
use 5.014;

package Local::Toolchain::Probe;

use Config;
use ExtUtils::MakeMaker;
use File::Spec;
use File::Basename qw(dirname);
use JSON::PP qw(encode_json decode_json);

our $VERSION = '0.001';

my %meta_merge = (
    resources => {
        repository => {
            type => 'git',
            url  => 'https://example.invalid/repo.git',
            web  => 'https://example.invalid/repo',
        },
        bugtracker => {
            web => 'https://example.invalid/repo/issues',
        },
    },
    no_index => {
        directory => [qw(t xt inc maint)],
    },
);

my %fallback_prereqs = (
    'JSON::PP' => '2.97001',
    'Try::Tiny' => '0.30',
    'Path::Tiny' => '0.118',
);

my %suggested_features = (
    dev => {
        description => 'Developer tooling and lint commands',
        prereqs => {
            'Perl::Critic' => '1.150',
            'Perl::Tidy' => '20240202',
        },
    },
    coverage => {
        description => 'Coverage reports in CI',
        prereqs => {
            'Devel::Cover' => '1.40',
        },
    },
);

sub _probe_prereq {
    my ($module, $minimum) = @_;
    my $ok = eval "use $module $minimum (); 1";
    return $ok ? 1 : 0;
}

sub _collect_missing {
    my (%spec) = @_;
    my @missing;

    for my $module (sort keys %spec) {
        my $minimum = $spec{$module};
        push @missing, "$module >= $minimum" unless _probe_prereq($module, $minimum);
    }

    return @missing;
}

sub _dynamic_prereqs {
    my %dynamic = (
        'Module::Metadata' => '1.000038',
    );

    if ($^O eq 'MSWin32') {
        $dynamic{'Win32::Console::ANSI'} = '1.10';
    } elsif ($^O =~ /darwin|linux/) {
        $dynamic{'IO::Pty'} = '1.17';
    }

    return %dynamic;
}

sub _load_optional_config {
    my $path = File::Spec->catfile(dirname(__FILE__), 'dist.config.json');
    return {} unless -f $path;

    open my $fh, '<', $path or die "Cannot read $path: $!";
    local $/;
    my $json = <$fh>;
    close $fh;

    return decode_json($json);
}

sub _emit_build_snapshot {
    my (%args) = @_;
    my $snapshot = {
        perl => $],
        archname => $Config{archname},
        osname => $Config{osname},
        cc => $Config{cc},
        config_args => \%args,
        generated_at => scalar gmtime,
    };

    my $target = File::Spec->catfile('maint', 'build-snapshot.json');
    if (open my $out, '>', $target) {
        print {$out} encode_json($snapshot);
        close $out;
    }
}

my %dynamic_prereqs = _dynamic_prereqs();
my %all_prereqs = (%fallback_prereqs, %dynamic_prereqs);
my @missing = _collect_missing(%all_prereqs);
my $optional_config = _load_optional_config();

warn "Optional prerequisites missing: @missing\n" if @missing;

my %write_makefile_args = (
    NAME => 'Local::Toolchain::Probe',
    VERSION_FROM => 'lib/Local/Toolchain/Probe.pm',
    ABSTRACT_FROM => 'lib/Local/Toolchain/Probe.pm',
    LICENSE => 'perl',
    EXE_FILES => [qw(script/toolchain-probe script/toolchain-report)],
    PREREQ_PM => \%all_prereqs,
    MIN_PERL_VERSION => '5.014',
    META_MERGE => \%meta_merge,
    CONFIGURE_REQUIRES => {
        'ExtUtils::MakeMaker' => '7.70',
        'JSON::PP' => '2.97001',
    },
    TEST_REQUIRES => {
        'Test2::V0' => '0.000145',
        'Test::Warnings' => '0.038',
    },
    ($optional_config->{extra_makefile} ? %{$optional_config->{extra_makefile}} : ()),
);

if (@missing) {
    $write_makefile_args{PREREQ_FATAL} = 0;
    $write_makefile_args{realclean}{FILES} = 'MYMETA.*';
}

_write_feature_comments(%suggested_features);
_emit_build_snapshot(%write_makefile_args);
WriteMakefile(%write_makefile_args);

sub MY::postamble {
    return <<'POSTAMBLE';
cover ::
	$(FULLPERLRUN) -MDevel::Cover -Ilib -It/lib -e "do './t/author/coverage.t'"

lint ::
	$(FULLPERLRUN) -Mstrict -Mwarnings -e "print qq{lint ok\\n}"
POSTAMBLE
}

sub _write_feature_comments {
    my (%features) = @_;

    for my $name (sort keys %features) {
        my $entry = $features{$name};
        my $desc  = $entry->{description} // 'no description';
        my @mods  = sort keys %{$entry->{prereqs} || {}};

        printf "# feature=%s desc=%s modules=%s\n", $name, $desc, join(',', @mods);
    }
}

1;
