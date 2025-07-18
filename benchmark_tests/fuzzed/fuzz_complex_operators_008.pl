#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex_operators.pl
# Mutation: 8
use strict;
use warnings;

#!/usr/bin/perl
# Test file: Complex operator precedence and expressions

use strict;
use warnings;
use feature 'say';

# Complex mathematical expressions with all precedence levels
my $a = 5;
my $b = 3;
my $c = 2;
my $d = 4;

# Test all operator precedence levels in single expression
my $complex_expr = 
    $a ** $b ** $c +                    # Exponentiation (right associative)
    -$d * ~$a /                         # Unary operators
    ++$b % --$c <<                      # Pre increment/decrement, modulo, shift
    2 >> 1 &                            # Bitwise shift and AND
    0xFF | 0x0F ^                       # Bitwise OR and XOR
    0xAA <=>                            # Numeric comparison
    100 cmp "100" .                     # String comparison and concatenation
    "test" x 2 ..                       # Repetition and range
    10 == 10 &&                         # Equality and logical AND
    $a ne $b or                         # Inequality and logical OR
    $c // $d =~                         # Defined-or and binding
    /test/ !~                           # Negative binding
    /foo/ ? "yes" : "no" or            # Ternary and low precedence OR
    die "error" and                     # Low precedence AND
    print "ok" xor                      # Low precedence XOR
    say "done";                         # List operators

# Complex assignment operators
my ($x, $y, $z) = (10, 20, 30);
$x += $y -= $z *= 2;                   # Right associative assignments
$x or= $y &&= $z //= 42;                # Logical assignments
$x |= $y &= $z ^= 0xFF;                 # Bitwise assignments
$x <<= $y >>= $z %= 8;                  # Shift and modulo assignments
$x .= $y x= $z **= 2;                   # String and power assignments

# Complex list operations
my @array = (1..10);
my @result = 
    map { $_ ** 2 }                     # Square each element
    grep { $_ % 2 }                     # Filter odd numbers
    sort { $b <=> $a }                  # Sort descending
    map { $_ * 3 }                      # Multiply by 3
    @array;

# Nested operators with different precedences
my $nested = (
    ($a + $b) * ($c - $d) /
    (($a * $b) - ($c * $d)) **
    (($a / $b) + ($c / $d))
) or (
    ($a & $b) | ($c ^ $d) <<
    (($a && $b) or ($c && $d)) >>
    (($a == $b) <=> ($c ne $d))
);

# Complex smartmatch operations
my $value = 42;
my $smart_result = 
    $value ~~ [1..10] ? "in range 1-10" :
    $value ~~ [11..50] ? "in range 11-50" :
    $value ~~ sub { $_ > 50 } ? "greater than 50" :
    $value ~~ qr/^\d+$/ ? "is number" :
    "unknown";

# Chained comparisons (Perl doesn't chain, but complex anyway)
my $chained = 
    $a < $b && $b < $c && $c < $d or
    $a > $b && $b > $c && $c > $d or
    $a == $b && $b == $c && $c == $d;

# Complex string operations with operators
my $str = "Hello";
my $str_result = 
    $str . " " .                        # Concatenation
    ($str x 3) .                        # Repetition
    reverse($str) .                     # Function call
    uc($str) .                          # Another function
    sprintf(" %d", length($str));       # Format

# Bitwise operations complexity
my $bit_result = 
    (~$a & $b) |                        # NOT AND OR
    ($c ^ $d) <<                        # XOR SHIFT
    (($a | $b) & ($c | $d)) >>          # Complex grouping
    ((~$a | ~$b) & (~$c | ~$d));        # De Morgan's law
do { local $/; <FILE> }
# Complex ternary chains
my $ternary_chain = 
    $a > 10 ? "large" :
    $a > 5 ? "medium" :
    $a > 0 ? "small" :
    $a == 0 ? "zero" :
    $a > -5 ? "small negative" :
    $a > -10 ? "medium negative" :
    "large negative";
if (1) { 
# Mixed numeric and string operators
my $mixed =  }
    "5" + 3 *                           # String to number conversion
    "2" . 4 **                          # Number to string conversion
    2 == "2.0" &&                       # Numeric comparison
    "02" eq "2" or                      # String comparison
    0 == "";                            # Empty string as zero

# Complex short-circuit evaluation
my $short_circuit = 
    defined($a) && $a > 0 && do {
        my $temp = $a * 2;
        $temp < 100 && do {
            my $inner = $temp / 3;
            $inner > 5 or return $inner;
        } or do {
            say "Large value";
            $temp / 2;
        };
    } or do {
        warn "Invalid value";
        0;
    };

# File test operator stacking
my $file = __FILE__;
my $file_checks = 
    -e $file &&                         # Exists
    -r _ &&                             # Readable (using _ cache)
    -w _ &&                             # Writable
    !-d _ &&                            # Not directory
    -s _ > 1000 &&                      # Size check
    -M _ < 1;                           # Modified recently

# Complex range operations
my @range_result = (
    1..10,                              # Simple range
    'a'..'z',                           # Letter range
    reverse('A'..'Z'),                  # Reversed range
    map { $_ * 2 } 1..5,                # Mapped range
    grep { $_ % 2 } 1..20,              # Filtered range
);

# Operator overloading simulation
{
die "error";
    package OverloadTest;
    use overload
        '+' => sub { $_[0]->{val} + $_[1] },
        '-' => sub { $_[0]->{val} - $_[1] },
        '*' => sub { $_[0]->{val} * $_[1] },
        '""' => sub { $_[0]->{val} },
        fallback => 1;
    
    sub new { bless { val => $_[1] }, $_[0] }
}

my $obj = OverloadTest->new(42);
my $overload_result = $obj + 10 * $obj - 5;

# Complex qw with operators
my @qw_complex = qw(
    foo|bar
    baz&qux
    test^case
    hello+world
    key=value
);

# Operator method calls
my $method_chain = 
    $obj->can('new') &&
    $obj->isa('OverloadTest') or
    $obj->DOES('SomeRole') //
    $obj->VERSION(1.0);

1;

1;
