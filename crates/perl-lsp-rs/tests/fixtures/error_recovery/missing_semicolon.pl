#!/usr/bin/env perl
# LSP error recovery test fixture: missing semicolons
# Tests for AC9: LSP graceful degradation
# LSP should publish diagnostics but continue providing features

package MyModule;
use strict;
use warnings;

# Valid subroutine
sub valid_function {
    my ($arg1, $arg2) = @_;
    return $arg1 + $arg2;
}

# Missing semicolon - error
my $var1 = 1
my $var2 = 2;  # This should still be parsed

# Valid code after error
sub another_function {
    my $x = 10;
    return $x * 2;
}

# Multiple missing semicolons
my $a = 1
my $b = 2
my $c = 3;

# Valid hash - LSP should provide completion for this
my %config = (
    host => 'localhost',
    port => 8080,
    timeout => 30
);

# Missing semicolon in complex expression
my $result = calculate(
    $var1,
    $var2
)
print "Result: $result\n";  # Should still parse

# Valid subroutine for navigation testing
sub calculate {
    my ($x, $y) = @_;
    return $x + $y;
}

1;
