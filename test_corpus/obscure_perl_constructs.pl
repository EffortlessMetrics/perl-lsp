#!/usr/bin/perl
# Corpus strengthening fixture for issue #1381.
#
# Exercises obscure-but-valid Perl constructs that were previously absent from
# test_corpus/. Every construct below parses cleanly today; this fixture locks
# that in via the auto-discovery corpus parse gate (corpus_gap_tests.rs).
use strict;
use warnings;
use feature 'current_sub';
no warnings 'experimental';

# 1. Recursive anonymous subroutine via __SUB__ (Perl 5.16+).
my $factorial = sub {
    my ($n) = @_;
    return 1 if $n <= 1;
    return $n * __SUB__->($n - 1);
};
my $result = $factorial->(5);

# Bare __SUB__ reference (no immediate invocation).
my $self_ref = sub { return __SUB__ };

# 2. Overriding a builtin through the CORE::GLOBAL:: typeglob.
BEGIN {
    *CORE::GLOBAL::exit = sub { die "exit trapped: @_\n" };
}

# 3. Explicit CORE:: calls reach the original builtin past any override.
my $fh;
if (CORE::open($fh, '<', '/dev/null')) {
    CORE::close($fh);
}

# 4. Smartmatch operator (Perl 5.10+).
my @list = (1, 2, 3);
if (3 ~~ @list) {
    print "found\n";
}

# 5. Scalar flip-flop range operator.
my $emit = 0;
for my $line ("BEGIN", "body", "END", "after") {
    $emit = 1 if $line =~ /BEGIN/ .. $line =~ /END/;
}

# 6. vec() used as an lvalue and as an rvalue.
my $bitstring = '';
vec($bitstring, 0, 8) = 65;
my $byte = vec($bitstring, 0, 8);

# 7. Variable-variables: deeply nested sigils.
my $name   = 'value';
my $ref    = \$name;
my $deref  = ${$ref};
my %hash   = (key => 'data');
my $key    = 'key';
my $nested = ${$hash{$key}};

1;
