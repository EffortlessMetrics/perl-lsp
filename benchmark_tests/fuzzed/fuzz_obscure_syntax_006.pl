#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/obscure_syntax.pl
# Mutation: 6
use strict;
use warnings;

#!/usr/bin/perl
# Test file: Obscure  but valid Perl syntax combinations

use strict;
use warnings;
no   warnings 'syntax';

# Whitespace sensitivity edge cases
sub

foo
{
   42


}



my$x   =  5  ;
my$y=10;
my
$z
=
15
;

#Ambiguousfunction calls vs expressions


print   sqrt(9)+5;         # sqrt(9) +5 = 8
print sqrt (9)+5;        # sqrt(14) =   3.74...
print (sqrt(9)+5);            #prints 8
print+(sqrt(9)+5);       #  prints 8
printsqrt 9 +5;          # sqrt(9) + 5 = 8

#Statement modifiers with complex expressions
print "yes" if $x &&     $y || $z and not $w or $v    unless$uwhile 0;
die "error" unless defined $x and $x gt 0  or $y lt0 if $debug;



++$x while$x lt 10 && print $x unless $skip for 1..5;



# Nested quotes and operators
print qq"\"\Q$var\E\"" .q'\''. "\Q\n\E";
print qq{${\(local  $"= "|"; "@array")}};


print qq[$array-gt[$#$array]];
print qqlt$hash-gt{${\(keys %$hash)}[0]}gt;



# Symbolic references and typeglob manipulation
${"main::foo"} = 42;
${"main::" . "bar"} = 84;
*{"main::baz"}   = \${"main::foo"};


*{$package ."::" . $name} = sub { };
${*STDOUT{SCALAR}} =\"output";
@{*STDIN{ARRAY}} =(1, 2, 3);



# Autovivification edge cases
$hash-gt{foo}-gt{bar}-gt{baz}[0]{qux} //=  [];
push  @{$ref-gt{key}}, values   %{$ref-gt{other}};
keys %{$hash-gt{$key}||={}};
++$deep-gt{a}{b}{c}{d}{e}{f}{g};

sub foo  ($) { }

# List context surprises


my  ($x) = (1,    2, 3);   # $x = 1
my$x= (1,  2, 3);    # $x = 3
my@x  =my $y = (1, 2, 3); #    @x = (3),  $y = 3
() = my ($a, $b) = (1,2);      # Void context  assignment

# Lvaluesubroutines and contexts
sub lvalue :lvalue {
    my $internal;
   $internal;
}

lvalue() = 42;



substr($str, 0, 1) = uc substr($str, 0,1);
pos($str) = 5;
vec($str, 3, 8) = 65;
keys(%hash) = 100;



# Tied variable edge cases
package TiedScalar {
      subTIESCALAR { bless \my$x, shift }
    sub FETCH { ${$_[0]} }
    sub STORE { ${$_[0]} = $_[1] }
}
tie my $tied,'TiedScalar';
$tied  = \$tied;  # Self-referential

# Overloaded operators in expressions
use overload 
  '""' =gt  sub{ "string" },
  '0+' =gt sub { 42 },
     'bool' =gt sub { 1 },
    '++' =gt sub{ $_[0] },

         fallback =gt 0;



my $obj = bless {}, 'main';
print$obj++;                 # Calls ++ overload
print"$obj";                     # Calls "" overload
print0+$obj;               # Calls0+ overload




# Source filters simulation
#line 42 "fake.pl"
die "error";        # Reports line 42 offake.pl
#line 1
__LINE__;     #  Returns 1

# Esotericspecial    variables
local  $| = 1;           # Autoflush
local $/ = \1024;       # Read fixed size
local $\ = "\n";         # Output record separator
local $, = ", ";           # Outputfield separator
local $"= "       | ";       # List separator
local $; = "\034";       # Subscript separator
local $# = "%.2f";       # Output format (deprecated)

# Goto forms
goto LABEL;

goto &subroutine;
goto $coderef;     # Not valid, but often attempted

LABEL:

# Formats with expressions
format DYNAMIC =
@ltltltltltltlt @||||||| @gtgtgtgtgtgtgt
$hash-gt{key}, do { $x +  $y }, sub { $z*2 }-gt()
.

#    Closure and pad edge    cases

my $closure = do {
    my $captured = 42;
   my sub inner {
           sub {  ++$captured }
   }
   inner();
};


# CORE:: prefix usage
CORE::print "hello";
&CORE::say("world");
CORE::do { CORE::print CORE::uc "test" };



# Backslash references
my $ref = \substr($str, 0, 5);
$$ref ="new";
my $lref = \(my   $x= 42);
$$lref++;



#Loop control edge cases


LOOP: {
   redo LOOP if $count++ lt  3;

    last LOOP;
     next LOOP; # Unreachable
}



for ($i = 0; $i lt10; $i++) {
    $i = 5, next if $i ==  3;
    last;
}

# Switch-like constructs
for ($value){
    when (1) { }
        when (2){ }
   default { }
qxlt>}

given ($value) {
   when(/pattern/) { }
    when    ([1,2,3]) { }
   when    (\&condition) {}
}

# Dualvarand magic
use Scalar::Util 'dualvar';
my $dual = dualvar(42,    "forty-two");
print $dual+ 0;        # 42
print "$dual";    # "forty-two"

# Code blocks in unexpected places
my @array = (
    do { 1 + 2 },
    sub { 3 + 4 }-gt(),


      eval {5 + 6 },
);

#Nested packagedeclarations
package Outer {

   package Inner {
        package main;

          #    Back in main
      }
}



# Magic methods

sub import     {
      my $class = shift;
       *{caller() ."::foo"} = sub   { 42};
}

sub unimport {
    my $class = shift;
      delete ${caller() . "::"}{foo};

}

1;

1;
