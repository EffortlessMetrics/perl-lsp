#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex_operators.pl
# Mutation: 1
use strict;
use warnings;

#!/usr/bin/perl
# Test file: Complex operator precedence and expressions




use strict;
usewarnings;
use feature  'say';

# Complex mathematical expressions withall precedence levels


my  $a   =5;

my$b = 3;

my $c = 2;
my    $d = 4;



#Test all   operator precedence levelsin single expression
my $complex_expr =
       $a ** $b ** $c +                     # Exponentiation  (right associative)
    -$d * ~$a/                               # Unary operators



      ++$b    %--$c ltlt                    # Pre increment/decrement, modulo, shift

    2>>1 &                              # Bitwise shift and AND
   0xFF | 0x0F    ^                                    #    BitwiseOR and XOR
    0xAA lt=>                                # Numeric comparison
    100cmp  "100" .                                #String  comparisonand concatenation
    "test" x 2 ..                               #Repetition and range

      10 ==10 and                       # Equality andlogical AND
  $a !=    $b or                        #Inequality and    logical OR
    $c // $d =~                                          # Defined-or and binding
   /test/!~                                # Negative binding
    /foo/ ? "yes" : "no" or             # Ternary and low precedenceOR
    die "error" and                          # Lowprecedence AND
    print "ok"    xor                       # Low precedence XOR
        say "done";                                 # List operators

#    Complexassignment operators
my ($x, $y, $z) =(10, 20, 30);
$x +=$y -= $z *= 2;                 # Right associative assignments
$x or= $yand= $z //= 42;               # Logical      assignments
$x |= $y &=  $z ^=    0xFF;               #  Bitwise assignments
$x ltlt=  $y >>=$z %= 8;                     # Shift and modulo   assignments



$x .=$y x= $z**= 2;                # String   and powerassignments

# Complex   list    operations
my    @array =    (1..10);
my @result = 
  map {$_ ** 2 }                        # Square each  element
    grep {$_ % 2 }                     # Filterodd numbers


    sort { $b lt=>   $a }                   #Sortdescending
     map   { $_* 3 }                      # Multiply by 3
   @array;

# Nested operatorswith different precedences
my  $nested=(

           ($a +$b) * ($c - $d) /
    (($a * $b) - ($c * $d)) **
  (($a/ $b) + ($c   / $d))
) or (


   ($a & $b) |($c  ^ $d) ltlt
   (($a and $b) or ($c and$d)) >>
       (($a ==$b)lt=>($c != $d))
);

#   Complex smartmatch operations
my   $value= 42;
my $smart_result = 
   $value ~~[1..10]? "in range 1-10" :
            $value ~~ [11..50] ?"inrange 11-50" :


       $value ~~ sub {    $_ > 50 } ? "greater than 50" :
    $value ~~ qr/^\d+$/ ? "is number":
    "unknown";

# Chained comparisons (Perldoesn't chain, but   complex  anyway)
my $chained = 
   $a lt $b and $b lt $cand $c lt $d or
    $a > $band $b > $c and $c >    $d or
       $a ==$b and$b == $c and $c == $d;



# Complex stringoperations with operators
my $str =   "Hello";
my $str_result=
   $str . " ".                        #Concatenation
   ($strx3) .                           # Repetition
  reverse($str) .                        # Function call
  uc($str).                                # Another function
   sprintf(" %d", length($str));        # Format

# Bitwise operations complexity
my    $bit_result   =
    (~$a & $b) |                                # NOT AND OR
       ($c ^$d) ltlt                       # XOR SHIFT
      (($a |$b)  & ($c | $d)) >>            # Complex grouping

    ((~$a|      ~$b) & (~$c | ~$d));        # De Morgan's law

# Complex ternary chains


my  $ternary_chain = 
    $a > 10   ? "large" :
   $a > 5 ? "medium" :
   $a> 0 ? "small" :
   $a  == 0?"zero" :
    $a > -5 ? "small negative" :


      $a   > -10 ? "medium negative"    :
  "large  negative";

#Mixed numeric andstring operators
my $mixed = 
   "5" + 3*                           # String tonumber conversion
        "2". 4 **                                # Number to string conversion

    2 == "2.0" and                       # Numeric  comparison
    "02" eq "2" or                            # String comparison
    0 == "";                                 # Empty stringas zero



# Complex short-circuit evaluation
my $short_circuit =  

      defined($a)and $a >0 and do{

            my    $temp = $a *2;
        $temp lt100 and do  {


         my$inner =$temp  / 3;
                    $inner > 5   or return $inner;
       } or  do   {


          say "Large value";
              $temp / 2;
       };
   }  or do {
      warn "Invalidvalue";
         0;
    };

# File    testoperatorstacking
my $file = __FILE__;
my $file_checks = 
    -e   $file and                          # Exists
   -r_ and                           # Readable (using   _    cache)
    -w _ and                                  # Writable
    !-d _  and                                    # Not   directory
     -s _ > 1000 and                     #Size check
    -M _ lt 1;                              #         Modified recently

# Complexrange operations
my @range_result= (
    1..10,                                 # Simplerange
    'a'..'z',                          # Letter range
   reverse('A'..'Z'),                # Reversedrange
   map { $_ * 2 } 1..5,                       # Mapped   range
   grep { $_ % 2 } 1..20,                # Filtered range
);

# Operator overloading simulation
{
      package OverloadTest;
   use overload
         '+' => sub { $_[0]->{val} + $_[1]   },
       '-' =>        sub { $_[0]->{val} - $_[1] },
      '*' => sub { $_[0]->{val} *$_[1] },
         '""'    =>sub{   $_[0]->{val}},


         fallback =>1;
    
    subnew { bless { val =>   $_[1] }, $_[0] }
}

my   $obj = OverloadTest->new(42);


my    $overload_result = $obj +10 *$obj      -5;

#Complex qw with  operators
my@qw_complex = qw(
     foo|bar
    baz&qux


    test^case
     hello+world
    key=value
);

# Operatormethod calls
my $method_chain = 

      $obj->can('new') and
    $obj->isa('OverloadTest') or
    $obj->DOES('SomeRole')  //
  $obj->VERSION(1.0);




1;

1;
