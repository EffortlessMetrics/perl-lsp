#!/usr/bin/env perl
# Test: require, do FILE, use/no pragma patterns, and module loading variations
# NodeKinds exercised: Use, No, FunctionCall, If, Block, PhaseBlock, Package, String
# Coverage gap: conditional loading, complex use args, do FILE, require variations

use strict;
use warnings;

# --- Basic use/no ---
use Carp;
use Carp qw(croak confess);
use Carp ();                       # import nothing
use File::Basename;
use File::Spec::Functions qw(catfile catdir rel2abs);

# use with version
use 5.010;                         # require Perl version
use 5.020_001;                     # underscore version
# use v5.36;                       # v-string version

# no pragma
no strict 'refs';
no warnings 'uninitialized';
no warnings qw(once redefine);

# Re-enable
use strict 'refs';
use warnings 'uninitialized';

# --- use with import arguments ---
use constant PI => 3.14159;
use constant {
    E    => 2.71828,
    TAU  => 6.28318,
    ZERO => 0,
};

use Exporter 'import';
# use base qw(Exporter);
# use parent 'Exporter';
# use parent -norequire, 'Exporter';  # skip require

# --- require variations ---
# require with module name (bareword)
require Carp;

# require with string (file path)
require "Carp.pm";

# require with version
require 5.010;

# Conditional require
my $has_json;
eval {
    require JSON;
    JSON->import();
    $has_json = 1;
};

# require in if statement
if (eval { require YAML; 1 }) {
    # YAML is available
}

# require with error handling
my $module = "Data::Dumper";
eval "require $module" or warn "Cannot load $module: $@";

# --- do FILE ---
# do executes a file and returns the last expression
# my $result = do "config.pl";

# do with error checking (three-part check)
# my $config = do "config.pl";
# if ($@) {
#     die "Error parsing config: $@";
# } elsif (!defined $config) {
#     die "Cannot read config: $!";
# } elsif (!$config) {
#     die "Config returned false";
# }

# do as expression
# my $settings = do {
#     local $/;
#     open my $fh, "<", "settings.pl" or die $!;
#     my $content = <$fh>;
#     close $fh;
#     eval $content;
# };

# --- Conditional module loading patterns ---

# Pattern 1: eval-require-import
my %loaded_modules;
sub load_module {
    my ($module, @imports) = @_;
    return 1 if $loaded_modules{$module};

    eval "require $module" or return 0;
    if (@imports) {
        $module->import(@imports);
    }
    $loaded_modules{$module} = 1;
    return 1;
}

# Pattern 2: try multiple backends
my $json_backend;
for my $mod (qw(Cpanel::JSON::XS JSON::XS JSON::PP)) {
    if (eval "require $mod; 1") {
        $json_backend = $mod;
        last;
    }
}

# Pattern 3: optional feature detection
my $CAN_FORK = eval { require POSIX; POSIX::_SC_OPEN_MAX(); 1 };
my $HAS_THREADS = eval { require threads; 1 };

# --- use with complex import lists ---
# use POSIX qw(:sys_wait_h :signal_h);     # tag imports
# use Fcntl qw(:flock :seek :mode);        # multiple tags
# use Socket qw(:addrinfo IPPROTO_TCP);    # mixed tags and names
# use Scalar::Util qw(blessed reftype weaken);

# --- no with specific categories ---
no strict 'vars';     # just vars
use strict 'vars';    # re-enable

no warnings;          # all warnings
use warnings;         # re-enable all

no warnings 'experimental::signatures';  # specific experimental

# --- BEGIN/END interaction with use ---
BEGIN {
    # This runs at compile time, like use
    # push @INC, "/custom/lib";
}

# use lib is syntactic sugar for BEGIN { push @INC, ... }
# use lib '/custom/lib';
# use lib qw(/path1 /path2);

# no lib to remove paths
# no lib '/custom/lib';

# --- Package-level use patterns ---
package MyModule;

# use Exporter 'import';
# our @EXPORT_OK = qw(func1 func2);
# our %EXPORT_TAGS = (
#     all    => \@EXPORT_OK,
#     basic  => [qw(func1)],
# );

# Version declaration with use
our $VERSION = '1.23';

sub func1 { return "func1" }
sub func2 { return "func2" }

package main;

# --- use overload ---
package Overloaded;

sub new {
    my ($class, $value) = @_;
    return bless { value => $value }, $class;
}

use overload
    '+'  => \&add,
    '-'  => \&subtract,
    '""' => \&stringify,
    '<=>' => \&compare,
    'fallback' => 1;

sub add {
    my ($self, $other, $swap) = @_;
    my $val = ref($other) ? $other->{value} : $other;
    return Overloaded->new($self->{value} + $val);
}

sub subtract {
    my ($self, $other, $swap) = @_;
    my $val = ref($other) ? $other->{value} : $other;
    my $result = $swap ? $val - $self->{value} : $self->{value} - $val;
    return Overloaded->new($result);
}

sub stringify {
    my ($self) = @_;
    return $self->{value};
}

sub compare {
    my ($self, $other, $swap) = @_;
    my $val = ref($other) ? $other->{value} : $other;
    return $swap ? $val <=> $self->{value} : $self->{value} <=> $val;
}

package main;

my $a = Overloaded->new(10);
my $b = Overloaded->new(20);
my $c = $a + $b;
my $d = $b - $a;
my $cmp = $a <=> $b;

print "require/do/use patterns test complete\n";
