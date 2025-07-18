#!/usr/bin/perl

# If statement
if ($x > 5) {
    print "x is greater than 5\n";
}

# If-else  
if ($y == 0) {
    print "y is zero\n";
} else {
    print "y is not zero\n";
}

# If-elsif-else
if ($z < 0) {
    print "negative\n";
} elsif ($z == 0) {
    print "zero\n";
} else {
    print "positive\n";
}

# Unless
unless ($flag) {
    print "flag is false\n";
}

# While loop
while ($i < 10) {
    print $i;
    $i++;
}

# For loop
for ($j = 0; $j < 5; $j++) {
    print $j;
}

# Foreach loop
foreach my $item (@array) {
    print $item;
}

# Until loop
until ($done) {
    process();
    $done = check_done();
}