#!/usr/bin/env perl
# Test: Error recovery edge cases
# Impact: Test parser's ability to recover from malformed code

use strict;
use warnings;

# Test 1: Severely malformed statements
# Unmatched brackets and parentheses
my $test1 = (1 + 2;  # Missing closing parenthesis
my @test2 = [1, 2, 3;  # Missing closing bracket
my %test3 = (key1 => 1, key2 => 2;  # Missing closing parenthesis

# Unmatched braces
if ($test1) {
    print "test";  # Missing closing brace

# Unmatched quote-like operators
my $test4 = "unterminated string;
my $test5 = q{unterminated q operator;
my $test6 = qq{unterminated qq with $var;

# Test 2: Recovery from incomplete statements
# Incomplete control structures
if ($condition)  # Missing block
    print "missing braces";

for my $i (1..10)  # Missing block
    print $i;

while ($condition)  # Missing block
    do_something();

# Incomplete subroutines
sub incomplete_sub
    my $x = shift;
    return $x;  # Missing closing brace

# Incomplete package declarations
package Incomplete::Package
my $var = 1;  # Missing semicolon

# Test 3: Unexpected tokens in various contexts
# Unexpected tokens in expressions
my $result = 1 + 2 end;  # 'end' is unexpected
my $calc = 5 * 6 if else;  # 'else' is unexpected
my $value = $var @array;  # Unexpected array in scalar context

# Unexpected tokens in control structures
if ($condition) then {  # 'then' is not valid Perl syntax
    print "test";
}

for (my $i = 0; $i < 10; $i++) step {  # 'step' is not valid Perl syntax
    print $i;
}

# Unexpected tokens in declarations
my $var1 = 1, $var2 = 2,;  # Extra comma
my @array = (1, 2, 3,, 5);  # Double comma
my %hash = (key1 => 1, key2 => 2,);  # Trailing comma in old Perl

# Test 4: Error recovery at different nesting levels
# Errors in deeply nested structures
my $deep = {
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    data => [1, 2, 3,  # Missing closing bracket
                }
            }
        }
    }
};

# Errors in nested control structures
if ($outer) {
    for my $i (1..10) {
        while ($inner) {
            if ($deep) {
                print "deep";  # Missing closing braces cascade
            }
        }
    }
}

# Test 5: Mixed syntax errors
# Multiple errors in single statement
my $mixed = 1 + 2 * 3 / 0 + $var @array % $hash{key};  # Multiple syntax issues

# Error in regex pattern
my $regex = /unterminated [pattern;  # Unterminated character class

# Error in subroutine call
my $result = some_function arg1, arg2,;  # Extra comma

# Test 6: Recovery from garbled input
# Random characters mixed with valid code
my $garbled1 = "valid" @#$%^&* "more valid";
my $garbled2 = 1 + 2 *&^%$ 3 + 4;

# Malformed Unicode sequences
my $malformed1 = "valid \x{invalid} more";
my $malformed2 = "text \N{unknown} text";

# Test 7: Context-specific error recovery
# In string context
my $string_error = "This string has an error \x{ and continues";

# In regex context
my $regex_error = /pattern [unclosed bracket/;

# In heredoc context
my $heredoc_error = <<'UNTERMINATED
This heredoc has no terminator
It just keeps going
UNTERMINATED

# Test 8: Recovery from operator precedence issues
# Ambiguous operator combinations
my $precedence1 = 1 + 2 * 3 ** 4 / 5 % 6 << 7 >> 8 & 9 | 10 ^ 11;
my $precedence2 = $a = $b = $c = $d = 1;  # chained assignment

# Test 9: Recovery from prototype mismatches
sub proto_test ($) { return $_[0] }
my $proto_error = proto_test 1, 2, 3;  # Too many arguments

# Test 10: Recovery from package and namespace issues
# Invalid package names
package 123Invalid::Name;  # Package name starts with number
package ::Invalid::Name;  # Leading double colon

# Test 11: Recovery from malformed attributes
sub attr_test : invalid_attribute syntax {
    return "test";
}

my $attr_var : invalid = 1;  # Invalid attribute syntax

# Test 12: Recovery from format statement errors
format STDOUT =
@<<<<<<<< @<<<<<<<<
$var1, $var2
# Missing terminating period

# Test 13: Recovery from signal handler errors
$SIG{TERM} = sub {
    print "Caught TERM";  # Missing closing brace

# Test 14: Recovery from typeglob manipulation errors
*alias = \&function;  # Assuming function doesn't exist
*glob_ref = *INVALID;  # Invalid typeglob

# Test 15: Recovery from eval errors
eval {
    my $eval_var = 1;
    print "eval block";  # Missing closing brace
};

# Test 16: Recovery from do block errors
do {
    my $do_var = 1;
    print "do block";  # Missing closing brace
};

# Test 17: Recovery from goto errors
goto INVALID_LABEL;  # Non-existent label

INVALID_LABEL:  # Label after goto (unusual)

# Test 18: Recovery from subroutine signature errors
sub sig_test ($$@) {  # Invalid signature
    my ($param1, $param2, @rest) = @_;
    return $param1 + $param2;
}

# Test 19: Recovery from file handle operation errors
open FILE, "nonexistent.txt" or die $!;
read FILE, $buffer, 100;  # FILE might not be open
close FILE;

# Test 20: Recovery from tie/untie errors
tie %tied_hash, 'NonExistentClass';
$untied_hash{key} = "value";  # Class doesn't exist
untie %tied_hash;

# Test 21: Recovery from bless errors
my $obj = bless {}, 'NonExistentClass';  # Class doesn't exist
$obj->method();  # Method doesn't exist

# Test 22: Recovery from require/use errors
require NonExistent::Module;  # Module doesn't exist
use Another::NonExistent;  # Module doesn't exist

# Test 23: Recovery from sort subroutine errors
my @sorted = sort custom_sort @array;
sub custom_sort {
    # Incomplete comparison function
    $a <=> $b  # Missing proper return structure
}

# Test 24: Recovery from map/grep block errors
my @mapped = map { $_ * 2, $_ + 1 } @array;  # Too many expressions
my @filtered = grep { $_ > 0, $_ < 10 } @array;  # Too many expressions

# Test 25: Recovery from tr/// errors
my $text = "hello";
$text =~ tr/hello/world/;  # Different length replacement lists

# Test 26: Recovery from sprintf/printf format errors
my $formatted = sprintf("%d %s %f", 42);  # Not enough arguments
printf "%d %s %f", 42;  # Not enough arguments

# Test 27: Recovery from pack/unpack errors
my $packed = pack("invalid_template", @data);
my @unpacked = unpack("invalid_template", $packed);

# Test 28: Recovery from time/date function errors
my $time = timelocal(25, 60, 24, 32, 12, 200);  # Invalid time values

# Test 29: Recovery from socket errors
use Socket;
socket(SOCKET, PF_INET, SOCK_STREAM, getprotobyname('tcp')) or die $!;
connect(SOCKET, sockaddr_in(80, inet_aton("invalid.hostname")));  # Invalid hostname

# Test 30: Recovery from dbmopen errors
dbmopen(%DB, "nonexistent_db", 0644) or die $!;
$DB{key} = "value";  # DB might not be open
dbmclose(%DB);

print "Error recovery edge cases test completed\n";
print "Note: This file contains intentional syntax errors to test parser recovery\n";