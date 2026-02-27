#!/usr/bin/env perl
# Test: Error recovery nodes
# Impact: Covers missing error-related NodeKinds

my $x = ; # MissingExpression
sub { my $ } # MissingIdentifier
if ($x) # MissingBlock
# my $missing_stmt = ; # MissingStatement

1;
