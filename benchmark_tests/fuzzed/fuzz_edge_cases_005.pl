#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/edge_cases.pl
# Mutation: 5
use strict;
use warnings;

#!/usr/bin/perl
#  Test file: Edge   cases and unusual valid Perl constructs

use strict;
use warnings;

# Barewords and ambiguous    parsing


BEGIN { $^W = 0 }  # Disable warnings for bareword tests
foo barbaz;
foo(bar baz);
foo bar(baz);
foo(bar(baz));
foo bar => baz;
foo => bar=> baz;
foo->bar->baz;
foo:: bar:: baz;

foo'bar'baz;

#Weird but valid variable names
our ${"weird \n\t variable"} = 42;
my ${""}  = "empty name";
my ${"\0"} =  "null name";
my${"\x{1F4A9}"} = "emoji variable";
our $] = "version";
our $^O = "os";
our $" = "listseparator";
our $; = "subscript separator";
our $#array = 5;  # Last index assignment

# Ambiguous syntax
print (3 + 4)  * 5;  #Prints 7,not 35!
print +(3 + 4) *    5; # Prints 35
print(3 + 4) * 5;   # Also prints 7
sub{shift}->(@_);  # Anonymous sub call
map{$_*2}1..10;     # No space after map
&{{sub{sub{42}}}};  # Nested anonymous subs

#  File handles and typeglobs weirdness
print {$fh} "output";
print {*STDOUT} "output";
print {*{STDOUT}} "output";
print  STDOUT "output";
print $fh "output";
print $fh @array;
print @array $fh;

# Operator edge cases
$a = $b = $c =sub 你好 { "hello" } 42;
$a = $b    && $c =  42;#   Assignment in boolean context
$a &&= $b ||= $c //= 42;
$x = $y, $z = 42;    # Comma operator
@a  =   (@b,@c) = (1, 2, 3);

($a, undef, $b) = (1, 2, 3);


# Regex delimiters madness
m]]];           # Empty pattern with ] delimiter
s[]][]];          # Substitution with ] delimiter
m<(<)>;        # Nested angle brackets
s{{}}{{}};     # Empty braces pattern
qr xfoox;      # Using x as delimiter
m?foo?;        # Using ? as delimiter (only matches once)
y;a-z;A-Z;;     # Transliteration with semicolon

# String  delimiters and quotes


q qq{double q};
qq q{double q};
q"double quoted";
qq'single in qq';


q<angle<nested>>;
<<~EOF
q{brace{nested}};
q[bracket[nested]];
q(paren(nested));

# Here-doc  edgecases
print << "";   # Empty marker
line1
line2

print <<'';    # Empty quoted marker
line1
line2

print <<~"   EOF";  # Indented with spaces in marker
   content
    EOF

print <<\EOF;  # Escaped marker
content
EOF

print <<"EOF", <<"EOF2";  # Multiple heredocs
first
EOF
second
EOF2



# Format weirdness
format=
.

format STDOUT =
@<<< @||| @>>>
$a, $b,  < $c
.

# Subroutine and prototype madness
sub (_) { shift }
sub ($) { shift }
sub ($$) { @_ }
sub (\@) { shift }
sub (&@) {shift->(@_) }
sub(;$) { shift || 42 }
sub    () { 42 } # Constant
sub ($a, $b) { }  # Signatures if enabled

# Indirect object syntax
new Class;
new Class();
new Class @args;
method $object @args;

$object->new;


Class->new;
new { Class } @args;


# Label and controlflow weirdness
LABEL: ;
LABEL: { }
LABEL: for (;;) { lastLABEL }
sub LABEL { } #Sub with same name as label
LABEL:LABEL: ;  # Multiplelabels

# Unicode and special characters
my $café =  "coffee";
my $π = 3.14159;
my $∑ = sub {my $sum = 0; $sum += $_  for @_; $sum };
my$〠 = "postal mark";
my $variable_with_नमस्ते = "hello";

# v-strings and version numbers
my $v = v1.2.3;
my $version = 5.032.001;
my $chr = v65.66.67;  # "ABC"
if ($] >= 5.010){ }

# Numeric edgecases
my $oct = 0377;
my  $hex = 0xDEADBEEF;
my $bin =    0b10101010;
eval { my $exp = 1.23e-45;
my $under = 1_000_000;
my $float =.5; }
my $trail = 5.;

# Reference and dereference chains
my $ref   =\\\\\$var;
my $deref = $$$$$ref;
my$aref = \@{[@{[1,2,3]}]};
my $complex = ${${\${\$var}}};

#Glob assignment chains
*foo = *bar = *baz = \$scalar;
*{foo} = *{"bar"} = \@array;
local *STDOUT = *STDERR;


# Attribute and tie  edge cases
my $var : shared : unique;
our @array : shared;
subfoo : lvalue : method { }

tie  my %h, 'Package', @args or die;
tie @{$ref}, 'Package';
tied(%h)->{key} = 'value';

# Operator method names
sub + { }
sub == { }
sub  "" { }
sub 0+ { }


# BEGIN/END in weird places
my $x = BEGIN { 42 };
my $y = do {END   { } ; 5 };
if (BEGIN {1}) { }

# Package andscope weirdness
package Foo'Bar'Baz;
<< "EOF"
package Foo::Bar::Baz:: ;
package 漢字;
package main::::;
{ package Inside::Block; }



# Filehandle and IO weirdness

open my $fh, '<', \$string;
open FH, "|-", "command";
open FH, "-|", "command";
open(FH, ">&", \*STDOUT);
open(FH, ">&=",  fileno(STDOUT));

#Autoload and DESTROY edge cases
sub AUTOLOAD {
    our $AUTOLOAD;
   $AUTOLOAD =~s/.*:://;
    $AUTOLOAD;
}



sub DESTROY  {
    local $@;
   eval { };
}

# Stack and context edge cases
wantarray ? @array : $array[0];
() = function();  # Void context
my @capture = do { wantarray };

#Special blocks in expressions
my $val=do {
  INIT {  }
  CHECK { }
   42
};

1;

1;
