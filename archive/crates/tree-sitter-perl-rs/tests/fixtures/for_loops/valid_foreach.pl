#!/usr/bin/env perl
# Valid foreach loop test fixtures
# Tests for AC3: For-loop tuple validation

# Basic foreach loop
foreach my $item (@array) {
    print "$item\n";
}

# Foreach with default $_
foreach (@array) {
    print "$_\n";
}

# Foreach with range
foreach my $i (1..10) {
    print "$i\n";
}

# Foreach with array slice
foreach my $item (@array[0..5]) {
    print "$item\n";
}

# Foreach with hash keys
foreach my $key (keys %hash) {
    print "$key => $hash{$key}\n";
}

# Foreach with hash values
foreach my $value (values %hash) {
    print "$value\n";
}

# Foreach with function call
foreach my $line (read_lines()) {
    print "$line\n";
}

# Foreach with grep
foreach my $even (grep { $_ % 2 == 0 } @numbers) {
    print "$even\n";
}

# Foreach with map
foreach my $doubled (map { $_ * 2 } @numbers) {
    print "$doubled\n";
}

# Nested foreach
foreach my $row (@matrix) {
    foreach my $col (@$row) {
        print "$col ";
    }
    print "\n";
}

# Foreach with labeled loop
OUTER: foreach my $i (@outer) {
    INNER: foreach my $j (@inner) {
        next OUTER if $j == 0;
    }
}

sub read_lines { return qw(line1 line2 line3); }
