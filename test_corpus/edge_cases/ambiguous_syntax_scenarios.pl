#!/usr/bin/env perl
# Test: Ambiguous syntax scenarios
# Impact: Test parser's ability to disambiguate complex syntax

use strict;
use warnings;

# Test 1: Slash disambiguation (division vs regex)
my ($a, $b, $c, $d) = (10, 2, 3, 4);

# Clear division
my $div1 = $a / $b;
my $div2 = $a / $b / $c;
my $div3 = ($a + $b) / ($c - $d);

# Regex patterns
my $regex1 = $a =~ /$b/;
my $regex2 = /$a\/$b/;
my $regex3 = m/$a\/$b/;

# Ambiguous cases that require careful parsing
my $ambiguous1 = $a / $b / $c;  # Could be ($a / $b) / $c or $a / ($b / $c)
my $ambiguous2 = $a =~ /$b/ / $c;  # Regex followed by division
my $ambiguous3 = $a / $b =~ /$c/;  # Division then regex match

# Test 2: Hash vs block ambiguity in various contexts
sub handle { return $_[0] }

# As function arguments
handle { key => 1 };  # Could be hashref or block
handle({ key => 1 });  # Clearly a hashref

# In conditional context
if ({ key => 1 }) { print "hashref\n"; }  # Hashref in condition
if (sub { return 1 }) { print "subref\n"; }  # Subref in condition

# With leading operators
+{ key => 1 };  # Hashref (unary plus)
-{ key => 1 };  # Hashref (unary minus)
*{ key => 1 };  # Typeglob

# In list context
my @list = ({ key => 1 }, { key => 2 });  # Array of hashrefs
my @list2 = ({ key => 1 }, sub { return 2 });  # Mixed

# Test 3: Indirect object syntax vs method calls
# Classic indirect object
my $file = new FileHandle "test.txt", "r";
my $time = new DateTime (year => 2024, month => 1, day => 1);
my $obj = new MyClass 'arg1', 'arg2';

# Method call syntax
my $file2 = FileHandle->new("test.txt", "r");
my $time2 = DateTime->new(year => 2024, month => 1, day => 1);
my $obj2 = MyClass->new('arg1', 'arg2');

# Ambiguous cases
new MyClass "arg1", "arg2";  # Indirect object
MyClass->new("arg1", "arg2");  # Method call

# With complex arguments
new MyClass 
    arg1 => "value1",
    arg2 => {
        nested => "value"
    };

# Test 4: Function vs method call disambiguation
package TestPackage;
sub new { bless {}, shift }
sub method { return "method" }
sub function { return "function" }

package main;

# Function calls
my $result1 = TestPackage::function();
my $result2 = function();

# Method calls
my $obj = TestPackage->new();
my $result3 = $obj->method();
my $result4 = TestPackage->method();

# Ambiguous with bareword
my $result5 = TestPackage::method();  # Function call
my $result6 = TestPackage->method();   # Class method

# Test 5: Ambiguous parentheses and precedence
my $x = 1, $y = 2, $z = 3;

# Function vs list vs hash
my $func_result = func($x, $y, $z);
my $list_result = ($x, $y, $z);
my $hash_result = ($x => $y, $z => 1);

# Ambiguous with complex expressions
my $complex1 = (1 + 2) * 3;  # Parentheses for grouping
my $complex2 = (1, 2, 3);    # List
my $complex3 = (1 => 2, 3 => 4);  # Hash

# Test 6: Ambiguous dereferencing
my $ref = [1, 2, 3];
my $href = { a => 1, b => 2 };

# Different dereference syntaxes
my $val1 = $ref->[0];
my $val2 = $$ref[0];
my $val3 = $href->{a};
my $val4 = $$href{a};

# Ambiguous with complex expressions
my $val5 = $ref->[$x + $y];
my $val6 = $$ref[$x + $y];
my $val7 = $href->{$x . $y};
my $val8 = $$href{$x . $y};

# Test 7: Ambiguous quote-like operators
my $str1 = q/test/;    # Single quotes
my $str2 = qq/test/;   # Double quotes
my $str3 = qw/test/;   # Word list
my $str4 = qr/test/;   # Regex
my $str5 = qx/test/;   # Command

# With different delimiters
my $str6 = q{test};
my $str7 = q[test];
my $str8 = q(test);
my $str9 = q|test|;

# Ambiguous with nested delimiters
my $str10 = q{test{nested}test};
my $str11 = q[test[nested]test];
my $str12 = q(test(nested)test);

# Test 8: Ambiguous regex modifiers
my $regex1 = /pattern/i;
my $regex2 = /pattern/msx;
my $regex3 = /pattern/gc;

# With variables
my $modifier = 'i';
my $regex4 = /pattern/$modifier;  # Variable interpolation vs modifier

# Test 9: Ambiguous statement modifiers
my $flag = 1;

# Classic statement modifiers
print "test" if $flag;
print "test" unless $flag;
print "test" while $flag;
print "test" until $flag;

# Ambiguous with complex expressions
print "test" if $flag && $x > 0;
print "test" unless $flag || $x < 0;
print "test" while $flag && $x < 10;
print "test" until $flag || $x > 10;

# Test 10: Ambiguous bareword handling
use constant CONST => 'value';
my $const = CONST;

# As subroutine call
sub bareword_sub { return "called" }
my $result = bareword_sub;

# As filehandle
open FILE, "test.txt" or die $!;
print FILE "test\n";
close FILE;

# As indirect object
my $fh = bareword_sub "arg1", "arg2";

# Test 11: Ambiguous prototype handling
sub proto1 ($) { return $_[0] }
sub proto2 (@) { return @_ }
sub proto2 (&) { return $_[0]->() }

# Different calling styles
my $p1 = proto1($x);      # With prototype
my $p2 = proto1 $x;       # Without parentheses
my $p3 = &proto1($x);     # Explicit reference

# Test 12: Ambiguous attribute syntax
my $attr1 : shared = 1;
my $attr2 : shared = 2;

# Subroutine attributes
sub attr_sub : method {
    return "method";
}

# With complex attributes
sub complex_attr : method : cached($timeout = 30) {
    return "cached method";
}

print "Ambiguous syntax scenarios test completed\n";