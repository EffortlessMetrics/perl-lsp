#!/usr/bin/env perl
# Invalid for-loop combination test fixtures
# Tests for AC3: For-loop tuple validation
# These should trigger descriptive errors instead of unreachable!()

# Invalid: Mixing C-style init with foreach syntax
for (my $i = 0; my $item (@array)) {
    print "$i: $item\n";
}

# Invalid: C-style with 'in' keyword (not Perl syntax)
for (my $i = 0 in @array) {
    print "$i\n";
}

# Invalid: Four-part for loop
for (my $i = 0; $i < 10; $i++; extra_part) {
    print "$i\n";
}

# Invalid: Two-part for loop (not enough parts)
for (my $i = 0; $i < 10) {
    print "$i\n";
}

# Invalid: Using 'in' instead of proper foreach syntax
for (my $item in @array) {
    print "$item\n";
}

# Invalid: Mixing initialization styles
for ($i = 0; my $j = 0; $i++) {
    print "$i $j\n";
}

# Invalid: Using arrow operator in for header
for ($i = 0; $i < 10; $i->) {
    print "$i\n";
}
