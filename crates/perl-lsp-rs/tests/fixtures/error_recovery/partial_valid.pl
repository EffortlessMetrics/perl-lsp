#!/usr/bin/env perl
# LSP error recovery test fixture: partial valid code
# Tests for AC9: LSP graceful degradation
# Tests that LSP features work on valid portions despite errors

package PartialValid;
use strict;
use warnings;

# Valid package variable - should provide hover info
our $PACKAGE_VAR = "accessible";

# Valid subroutine - should provide definition navigation
sub valid_function {
    my ($arg1, $arg2) = @_;
    return $arg1 + $arg2;
}

# Syntax error in middle of file
my $broken = {
    key1 => 'value1',
    key2 => 'value2'
    key3 => 'value3'  # Missing comma - error
};

# Valid code after error - should still provide completion
my %database_config = (
    host => 'localhost',
    port => 5432,
    database => 'mydb',
    username => 'user',
    password => 'pass'
);

# Valid subroutine - should be in workspace symbols
sub connect_database {
    my %config = @_;
    # Connection logic here
    return "connected to $config{host}:$config{port}";
}

# Another syntax error
if ($condition  # Missing closing paren
    print "error\n";
}

# More valid code - should provide semantic tokens
my @valid_array = qw(
    element1
    element2
    element3
    element4
);

# Valid loop - should provide folding ranges
foreach my $element (@valid_array) {
    print "Processing: $element\n";

    # Valid nested code
    if ($element eq 'element2') {
        print "Found special element\n";
    }
}

# Incomplete subroutine - missing closing brace
sub incomplete_sub {
    my $var = "test";
    return $var;
# Missing }

# Code after incomplete sub - should still analyze
my $final_var = "still accessible";

# Valid subroutine for call hierarchy testing
sub call_other_functions {
    my $result1 = valid_function(1, 2);
    my $db = connect_database(%database_config);
    return "$result1, $db";
}

# Malformed regex
my $bad_regex = qr/[unclosed character class/;

# Valid code for testing references
sub get_package_var {
    return $PACKAGE_VAR;  # Should find reference to our variable
}

1;
