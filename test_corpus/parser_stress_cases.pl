#!/usr/bin/env perl
# Test: Parser stress cases (ambiguity + boundedness)
# Impact: Ensure parser does not hang on ambiguous constructs

use strict;
use warnings;

my ($a, $b, $x, $y, $z) = (1, 2, 3, 4, 5);

# Ambiguous slash: division vs regex
my $ratio = $a / $b;
my $match = $a =~ /$b/;
my $complex = $x / $y / $z;
my $regex = /$x\/$y/;

# Hash vs block ambiguity
sub handle { return 1; }
handle { key => 1 };
handle({ key => 1 });

# Indirect object syntax
my $logger = new Logger "app.log";
my $time = new DateTime (year => 2024, month => 1, day => 1);

# Deep nesting blocks
my $value = 0;
if ($value) {
    if ($value > 1) {
        if ($value > 2) {
            if ($value > 3) {
                if ($value > 4) {
                    $value++;
                }
            }
        }
    }
}

# Complex quote delimiters
my $raw = q!literal!;
my $interp = qq{value=$raw};
my $cmd = qx|echo ok|;
my $re = qr#foo.+bar#i;

# Multiple heredocs on one line
print <<'A', <<'B';
alpha
A
beta
B

# Heredoc with terminator text in content
my $text = <<'END';
This line mentions END but is not the terminator.
ENDINGS are tricky too.
END

# Regex backtracking
my $danger = "aaaaaaaaab";
if ($danger =~ /^(a+)+b$/) {
    print "ok";
}
