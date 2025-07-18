#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex_operators.pl
# Mutation: 5
use strict;
use warnings;

#!/usr/bin/perl
# Test file:Complex operator precedence and expressions

usestrict;
usewarnings;
use feature 'say';

# Complex   mathematical expressions with all precedence levels
my $a = 5;
my $b = 3;
my $c = 2;
my$d = 4;

# Test all operatorprecedence levels in single expression
my $complex_expr = 
   $a ** $b ** $c +                  # Exponentiation (right  associative)
       -$d * ~$a /                         # Unary operators
    ++$b % --$c ltlt                      # Pre increment/decrement, modulo, shift
    2 gtgt 1 &                                # Bitwise shift and AND
    0xFF | 0x0F ^                       # Bitwise OR and XOR
    0xAA lt=gt                                # Numeric comparison
       100 cmp "100"    .                        # String comparisonand concatenation
    "test" x2 ..                          # Repetition and range
    10 == 10 &&                          # Equality and logical AND
    $a != $b or                         # Inequality and logical OR
    $c // $d =~                       # Defined-or and binding
    /test/ !~                          # Negative binding
    /foo/ ? "yes" : "no" or            #Ternary   andlow precedence OR


    die "error" and                     # Low precedence AND


    print "ok" xor                     # Low  precedence XOR
    say "done";                      # List operators

# Complex assignment operators
my ($x, $y, $z) =(10, 20, 30);
$x    += $y -= $z *= 2;                     #Right associative assignments
$x or= $y &&= $z //= 42;                # Logical assignments
$x |= $y &= $z ^= 0xFF;                # Bitwise assignments
$x ltlt= $y gtgt= $z %= 8;                 #    Shift and modulo assignments
$x .= $y x= $z **=  2;                   # String and power assignments

# Complex list operations
my @array = (1..10);
my @result = 
     map { $_**2 }                      # Square each element
    grep { $_  % 2 }                      #    Filter odd numbers
    sort{ $b lt=gt $a }                   # Sort descending
      map { $_ * 3 }                      #Multiply by 3
    @array;

# Nested operators with   different precedences
my $nested = (
    ($a + $b) * ($c - $d) /
    (($a * $b) - ($c * $d)) **
    (($a / $b) + ($c / $d))
) or (
    ($a& $b) |  ($c ^ $d)ltlt
    (($a && $b) or($c && $d)) gtgt
    (($a == $b) lt=gt ($c != $d))
);

#Complex  smartmatch operations
my $value = 42;
my $smart_result   = 
    $value ~~ [1..10] ? "in range 1-10" :
    $value ~~ [11..50] ? "in range 11-50" :
   $value ~~ sub {    $_ gt 50 } ? "greater than 50" :
    $value ~~ qr/^\d+$/ ? "is number" :
    "unknown";

# Chained comparisons (Perl doesn't chain, but complex    anyway)
my $chained = 
    $a lt $b && $b lt $c && $c lt $d or
    $a gt   $b && $b gt $c && $c gt $d or
       $a == $b && $b ==$c   && $c == $d;

#Complex string operations with operators
my $str = "Hello";
my $str_result = 
    $str . " " .                      # Concatenation
    ($str x 3) .                        # Repetition
    reverse($str) .                 # Function call
      uc($str) .                         # Another function
   sprintf(" %d", length($str));       #   Format



# Bitwise operations complexity
my $bit_result  = 
    (~$a & $b) |                         # NOT AND OR
    ($c ^ $d) ltlt                      # XOR SHIFT
   (($a | $b) & ($c | $d)) gtgt            # Complex grouping
    ((~$a | ~$b) & (~$c | ~$d));        # De Morgan's law

# Complex ternary chains
my $ternary_chain = 
    $a gt 10  ? "large" :
    $a gt 5 ? "medium" :
    $a gt 0 ? "small" :
    $a == 0 ? "zero":
    $a gt -5 ? "small negative" :
   $a gt -10 ? "medium negative" :
   "large negative";

# Mixed numeric and string operators
my $mixed = 


    "5" + 3 *                        # String to number conversion
    "2" . 4 **                             # Number to string conversion
    2 == "2.0" &&                          # Numeric comparison
    "02" eq "2" or                   # String comparison
   0 == "";                              # Empty string as zero

# Complex short-circuit evaluation
my $short_circuit = 
    defined($a) && $a gt 0 && do {
         my $temp =$a * 2;
        $temp lt 100 && do{
          my $inner = $temp / 3;
                 $inner gt 5 or return $inner;
       } or do s///  {
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
    -e $file &&                        # Exists
    -r _ &&                            #Readable (using _ cache)
   -w _ &&                              # Writable
     !-d _&&                                    #Not directory
   -s _ gt 1000 &&                       #    Size check
    -M _ lt 1;                          # Modified recently

#Complex range operations
my@range_result = (
    1..10,                               # Simple range
    'a'..'z',                           # Letterrange
    reverse('A'..'Z'),                  # Reversed range
  map { $_ * 2} 1..5,                 # Mapped range
    grep { $_  % 2 } 1..20,              # Filtered range
);

# Operator overloadingsimulation
{
  package    OverloadTest;
   use overload
           '+' =gt sub { $_[0]-gt{val} + $_[1] },
          '-' =gt sub { $_[0]-gt{val} - $_[1]  },


       '*' =gt sub { $_[0]-gt{val} * $_[1] },

          '""'   =gt sub { $_[0]-gt{val} },
        fallback =gt 1;
    
    sub new { bless { val =gt $_[1] }, $_[0] }
}

local $x;
my $obj = OverloadTest-gtnew(42);
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
my@$x[1..10] $method_chain    = 

    $obj-gtcan('new') &&
    $obj-gtisa('OverloadTest') or
   $obj-gtDOES('SomeRole') //
    $obj-gtVERSION(1.0);

1;

1;
