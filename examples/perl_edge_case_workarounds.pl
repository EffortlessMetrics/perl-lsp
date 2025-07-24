#!/usr/bin/env perl
# Examples of edge case workarounds for tree-sitter-perl parser

use strict;
use warnings;
use feature 'say';

# =============================================================================
# 1. REGEX WITH ARBITRARY DELIMITERS
# =============================================================================

# ❌ Not supported: m with non-slash delimiters
# $text =~ m!pattern!;
# $text =~ m{pa{ttern};
# $text =~ s|old|new|g;

# ✅ Workaround: Use standard slash delimiters
my $text = "Hello pattern world";
$text =~ /pattern/;        # Standard match
$text =~ s/old/new/g;      # Standard substitution

# For complex patterns with many slashes, use \Q...\E or quotemeta
my $url = "https://example.com/path/to/file";
my $path = "/path/to/file";
$url =~ /\Q$path\E/;       # Escape special chars

# =============================================================================
# 2. INDIRECT OBJECT SYNTAX
# =============================================================================

# ❌ Not supported: Indirect object/method calls
# method $object @args;
# new Class::Name;
# print $fh "Hello";

# ✅ Workaround: Use arrow notation or parentheses
package MyClass {
    sub new { bless {}, shift }
    sub method { say "Called with: @_" }
}

my $object = MyClass->new();           # Arrow constructor
$object->method('arg1', 'arg2');       # Arrow method call

# For filehandles, use parentheses
open(my $fh, '>', 'output.txt') or die $!;
print($fh, "Hello\n");                 # Parentheses for clarity
close($fh);

# =============================================================================
# 3. FORMAT DECLARATIONS
# =============================================================================

# ❌ Not supported: Format blocks
# format STDOUT =
# @<<<<<<   @||||||   @>>>>>>
# $name,    $price,   $quantity
# .

# ✅ Workaround: Use printf/sprintf
my @items = (
    { name => 'Apple',  price => 1.50, quantity => 10 },
    { name => 'Banana', price => 0.75, quantity => 25 },
);

# Header
printf "%-10s %8s %8s\n", 'Name', 'Price', 'Qty';
printf "%s\n", '-' x 30;

# Data rows
for my $item (@items) {
    printf "%-10s %8.2f %8d\n", 
        $item->{name}, 
        $item->{price}, 
        $item->{quantity};
}

# =============================================================================
# 4. TYPEGLOB ASSIGNMENTS
# =============================================================================

# ❌ Not supported: Direct typeglob manipulation
# *foo = *bar;
# *{$name} = \&function;

# ✅ Workaround: Use references
sub original_function { return "Original" }
sub new_function { return "New" }

# Instead of typeglob assignment, use references
my $func_ref = \&original_function;
$func_ref = \&new_function;            # Reassign reference
say $func_ref->();                     # Call through reference

# For symbol table manipulation, use the %:: hash
my $name = 'dynamic_sub';
$main::{$name} = sub { return "Dynamic!" };
say dynamic_sub();                     # Works!

# =============================================================================
# 5. ADDITIONAL BEST PRACTICES
# =============================================================================

# Use qr// for regex precompilation (fully supported)
my $pattern = qr/foo.*bar/i;
$text =~ $pattern;

# Use heredocs for multi-line strings (fully supported)
my $multi = <<'END';
This is a multi-line
string that works perfectly
with the parser.
END

# Modern try/catch (fully supported)
use feature 'try';
try {
    die "Error!";
} catch ($e) {
    say "Caught: $e";
}

say "\nAll workarounds demonstrated successfully!";