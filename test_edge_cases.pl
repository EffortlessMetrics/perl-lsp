#!/usr/bin/perl

# Test edge case fixes

# 1. ISA operator with qualified names (FIXED)
my $obj = bless {}, 'Test::Object';
if ($obj isa Test::Object) {
    print "ISA simple works\n";
}
if ($obj isa Test::Object::Subclass) {
    print "ISA qualified works\n";  
}

# 2. Complex array interpolation (PARTIALLY FIXED)
my @array = (1, 2, 3);
print "Simple: @array\n";
print "Complex literal: @{[1, 2, 3]}\n";
# This might still have issues:
# print "Complex method: @{[$obj->method()]}\n";

# 3. Many more builtins without parentheses (FIXED)
my $len = length "test string";
my $upper = uc "hello";
my $lower = lc "WORLD";
my $abs_val = abs -42;
my $int_val = int 3.14;
my $chr_val = chr 65;
my $ord_val = ord 'A';

open my $fh, '<', 'file.txt' or die "Can't open";
close $fh;

my @sorted = sort 3, 1, 4, 1, 5, 9;
my $joined = join ',', @sorted;
my @parts = split ',', $joined;

# 4. Bareword qualified names (PARTIAL WORKAROUND)
# Still needs quotes but we added a special case for standalone calls
Foo::Bar->new();  # This works as a statement
# my $obj = Foo::Bar->new();  # This still needs quotes

# 5. All grammar fixes from earlier
use strict;
use warnings;
use Data::Dumper qw(Dumper);

package Modern {
    sub new {
        my $class = shift;
        return bless {}, $class;
    }
}

print "All edge case tests completed\n";