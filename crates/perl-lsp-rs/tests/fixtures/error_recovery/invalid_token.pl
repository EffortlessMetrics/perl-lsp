#!/usr/bin/env perl
# LSP error recovery test fixture: invalid tokens
# Tests for AC9: LSP graceful degradation
# LSP should handle lexer errors and continue

package InvalidTokens;
use strict;
use warnings;

# Valid code
my $valid_var = 42;

# Invalid token - bareword where operator expected
my $x = 5 bareword 10;

# Code after error
sub valid_sub {
    return "still works";
}

# Invalid operator
my $result = $x @@ $y;

# More valid code
my %config = (
    setting1 => 'value1',
    setting2 => 'value2'
);

# Invalid sigil combination
my $$double_dollar = "error";

# Recovery
sub another_function {
    my $param = shift;
    return $param * 2;
}

# Invalid delimiter in regex
my $regex = qr/pattern\/invalid;

# Valid array definition
my @numbers = (1, 2, 3, 4, 5);

# Stray closing brace
my $var = 1;
}  # Error: unexpected closing brace

# More valid code for partial AST
sub get_numbers {
    return @numbers;
}

# Invalid hash access
my $val = $hash->{key1]};  # Mismatched brackets

# Valid code continues
my $sum = 0;
foreach my $num (@numbers) {
    $sum += $num;
}

# Invalid function call syntax
my $broken = function-name-with-hyphens();

# Final valid subroutine
sub calculate_sum {
    my @values = @_;
    my $total = 0;
    $total += $_ for @values;
    return $total;
}

1;
