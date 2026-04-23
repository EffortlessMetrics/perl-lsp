#!/usr/bin/env perl
# LSP error recovery test fixture: unterminated strings
# Tests for AC9: LSP graceful degradation
# LSP should handle string errors and continue

package StringErrors;
use strict;
use warnings;

# Valid string
my $valid = "This is a valid string";

# Unterminated double-quote string - error
my $error1 = "This string is not terminated

# Code after error - should still be analyzed
my $recovered = "This string is valid";

# Valid subroutine - should be indexed
sub process_string {
    my ($str) = @_;
    return uc($str);
}

# Unterminated single-quote string - error
my $error2 = 'Another unterminated string

# More valid code for testing partial AST
sub another_sub {
    my $local = "local var";
    return $local;
}

# Unterminated regex - error
my $pattern = qr/unclosed regex

# Valid code after regex error
my %hash = (
    key1 => 'value1',
    key2 => 'value2'
);

# Unterminated heredoc - error
my $heredoc = <<'END'
This is a heredoc
that is not terminated

# Valid array - should provide completion
my @array = qw(one two three four five);

# Nested unterminated strings
my $nested = "outer " . "inner  # Double error

# Recovery with valid code
sub final_function {
    return "valid return value";
}

1;
