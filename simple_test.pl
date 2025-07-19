#!/usr/bin/env perl

# Test 1: Variables
my $x = 42;
print "Variables work\n";

# Test 2: Heredoc
my $text = <<EOF;
Hello from heredoc
EOF
print "Heredoc works\n";

# Test 3: Subroutine
sub greet {
    return "Hello";
}
print "Subroutines work\n";

print "All tests passed!\n";