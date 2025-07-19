#!/usr/bin/perl
use strict;
use warnings;

print "=== Testing heredocs in eval contexts ===\n\n";

# Test 1: Simple heredoc in eval string
print "Test 1: Simple heredoc in eval string\n";
my $code1 = 'my $x = <<EOF;
Hello from eval
EOF
print "In eval: $x\n";
';
eval $code1;
print "Error: $@\n" if $@;

# Test 2: Heredoc in eval block
print "\nTest 2: Heredoc in eval block\n";
eval {
    my $msg = <<'END';
This is a heredoc
inside an eval block
END
    print "Block eval: $msg";
};
print "Error: $@\n" if $@;

# Test 3: Multiple heredocs in eval string
print "\nTest 3: Multiple heredocs in eval string\n";
my $code3 = 'my ($a, $b) = (<<FOO, <<BAR);
First heredoc
FOO
Second heredoc
BAR
print "First: $a";
print "Second: $b";
';
eval $code3;
print "Error: $@\n" if $@;

# Test 4: Nested eval with heredocs
print "\nTest 4: Nested eval with heredocs\n";
eval {
    my $inner_code = 'my $nested = <<NESTED;
Inner heredoc
NESTED
print "Inner: $nested";
';
    eval $inner_code;
    print "Inner error: $@\n" if $@;
};
print "Outer error: $@\n" if $@;

# Test 5: Heredoc with interpolation in eval
print "\nTest 5: Heredoc with interpolation in eval\n";
my $var = "interpolated";
my $code5 = 'my $interp = <<"INTERP";
This is $var
INTERP
print "Result: $interp";
';
eval $code5;
print "Error: $@\n" if $@;

# Test 6: Eval generating code with heredocs
print "\nTest 6: Eval generating code with heredocs\n";
my $generator = <<'CODEGEN';
sub make_heredoc {
    my $content = <<'CONTENT';
Generated content
CONTENT
    return $content;
}
print "Generated: ", make_heredoc();
CODEGEN
eval $generator;
print "Error: $@\n" if $@;

# Test 7: Heredoc delimiter collision in eval
print "\nTest 7: Heredoc delimiter collision in eval\n";
my $collision = 'my $a = <<EOF;
First EOF
EOF
my $b = <<EOF;
Second EOF
EOF
print "A: $a";
print "B: $b";
';
eval $collision;
print "Error: $@\n" if $@;

# Test 8: Complex eval with mixed quotes and heredocs
print "\nTest 8: Complex eval with mixed quotes and heredocs\n";
eval q{
    my $complex = <<'COMPLEX';
This has "quotes" and 'apostrophes'
And $interpolation attempts
COMPLEX
    print "Complex: $complex";
};
print "Error: $@\n" if $@;