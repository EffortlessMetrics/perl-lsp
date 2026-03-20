#!/usr/bin/perl
# examples/perl/modern.pl
#
# Demonstrates: modern Perl syntax -- try/catch, subroutine signatures,
#               class/method (v5.38+), state variables, postfix dereference,
#               and feature bundles.
#
# LSP features exercised:
#   - diagnostics : catch undefined variable usage inside signatures
#   - hover       : show signature types when hovering over sub names
#   - completion  : parameter names appear in completions after opening '('
#   - go-to-def   : jump into class definitions from instantiation sites

use strict;
use warnings;
use feature 'say';

# ---------------------------------------------------------------------------
# 1. Subroutine signatures (5.20+)
# ---------------------------------------------------------------------------

use feature 'signatures';
no warnings 'experimental::signatures';

sub greet($name, $greeting = 'Hello') {
    return "$greeting, $name!";
}

sub add($x, $y) { $x + $y }

sub sum_all(@nums) {
    my $total = 0;
    $total += $_ for @nums;
    return $total;
}

sub divide($dividend, $divisor = 1) {
    die "division by zero\n" if $divisor == 0;
    return $dividend / $divisor;
}

say greet('World');
say greet('Perl', 'Greetings');
say "sum: " . sum_all(1, 2, 3, 4, 5);

# ---------------------------------------------------------------------------
# 2. try / catch (5.34+)
# ---------------------------------------------------------------------------

use feature 'try';
no warnings 'experimental::try';

sub safe_divide($a, $b) {
    my $result;
    try {
        $result = divide($a, $b);
    }
    catch ($e) {
        warn "caught: $e";
        $result = undef;
    }
    return $result;
}

say "10/2 = " . (safe_divide(10, 2) // 'undef');
say "10/0 = " . (safe_divide(10, 0) // 'undef');

# Nested try/catch with re-throw
sub parse_int($str) {
    my $result;
    try {
        die "not a number\n" unless $str =~ /^\d+$/;
        $result = int($str);
    }
    catch ($e) {
        if ($e =~ /not a number/) {
            die "parse_int: '$str' is not a valid integer\n";
        }
        die $e;   # re-throw unexpected errors
    }
    return $result;
}

try {
    say "parsed: " . parse_int('42');
    say "this won't print: " . parse_int('hello');
}
catch ($e) {
    say "outer catch: $e";
}

# ---------------------------------------------------------------------------
# 3. state variables (5.10+)
# ---------------------------------------------------------------------------

use feature 'state';

sub counter($label = 'default') {
    state %counts;
    $counts{$label}++;
    return $counts{$label};
}

say counter('visits');    # 1
say counter('visits');    # 2
say counter('clicks');    # 1
say counter('visits');    # 3

sub memoized_fib($n) {
    state %cache = (0 => 0, 1 => 1);
    return $cache{$n} if exists $cache{$n};
    $cache{$n} = memoized_fib($n - 1) + memoized_fib($n - 2);
    return $cache{$n};
}

say "fib(15) = " . memoized_fib(15);

# ---------------------------------------------------------------------------
# 4. Postfix dereference (5.20+)
# ---------------------------------------------------------------------------

use feature 'postderef';
no warnings 'experimental::postderef';

my $aref = [10, 20, 30, 40];
my $href = { a => 1, b => 2, c => 3 };
my $sref = \'hello';
my $cref = sub { "called!" };

# Postfix instead of ${$sref}, @{$aref}, %{$href}
say "scalar: "  . $sref->$*;
say "array:  "  . join(', ', $aref->@*);
say "hash:   "  . join(', ', map { "$_=$href->{$_}" } sort keys $href->%*);
say "code:   "  . $cref->&*;

# Slice via postfix
my @slice  = $aref->@[1, 3];     # elements at index 1 and 3
my %hslice = $href->%{qw(a c)};  # keys a and c
say "array slice: @slice";
say "hash slice: " . join(', ', map { "$_=$hslice{$_}" } sort keys %hslice);

# Nested data structure with postfix deref
my $data = {
    users => [
        { name => 'Alice', scores => [95, 87, 92] },
        { name => 'Bob',   scores => [78, 84, 91] },
    ],
};

for my $user ($data->{users}->@*) {
    my $avg = sum_all($user->{scores}->@*) / scalar($user->{scores}->@*);
    printf "%s: avg=%.1f\n", $user->{name}, $avg;
}

# ---------------------------------------------------------------------------
# 5. class / method (v5.38 corinna -- experimental)
# ---------------------------------------------------------------------------

use feature 'class';
no warnings 'experimental::class';

class Point {
    field $x :param;
    field $y :param;

    method x { $x }
    method y { $y }

    method to_string {
        "($x, $y)"
    }

    method distance_to($other) {
        my $dx = $x - $other->x;
        my $dy = $y - $other->y;
        return sqrt($dx**2 + $dy**2);
    }

    method translate($dx, $dy) {
        # Returns a new Point (immutable style)
        return Point->new(x => $x + $dx, y => $y + $dy);
    }
}

my $p1 = Point->new(x => 0, y => 0);
my $p2 = Point->new(x => 3, y => 4);
say $p1->to_string;
say $p2->to_string;
say "distance: " . $p1->distance_to($p2);   # 5

my $p3 = $p1->translate(1, 2);
say "translated: " . $p3->to_string;

say "Modern Perl feature demonstration complete.";
