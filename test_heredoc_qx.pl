#!/usr/bin/perl
use strict;
use warnings;

print "=== Testing heredocs in qx// and backtick contexts ===\n\n";

# Test 1: Heredoc in qx//
print "Test 1: Heredoc in qx//\n";
my $cmd1 = qx(echo "<<EOF
Hello from qx
EOF");
print "Result: $cmd1\n";

# Test 2: Heredoc in backticks
print "\nTest 2: Heredoc in backticks\n";
my $cmd2 = `echo "<<EOF
Hello from backticks
EOF"`;
print "Result: $cmd2\n";

# Test 3: Using heredoc to build command
print "\nTest 3: Using heredoc to build command\n";
my $script = <<'SCRIPT';
echo "Running script"
echo "Line 2"
SCRIPT
my $result3 = qx(sh -c '$script');
print "Result: $result3\n";

# Test 4: Heredoc with interpolation in qx
print "\nTest 4: Heredoc with interpolation in qx\n";
my $var = "interpolated";
my $cmd4 = <<"CMD";
echo "$var in command"
CMD
chomp $cmd4;
my $result4 = qx($cmd4);
print "Result: $result4\n";

# Test 5: Multiple heredocs for complex commands
print "\nTest 5: Multiple heredocs for complex commands\n";
my $input = <<'INPUT';
test input
INPUT
my $script5 = <<'SCRIPT';
cat | tr a-z A-Z
SCRIPT
chomp $input;
chomp $script5;
my $result5 = qx(echo '$input' | sh -c '$script5');
print "Result: $result5\n";

# Test 6: Nested qx with heredocs
print "\nTest 6: Nested qx with heredocs\n";
my $outer = qx(perl -e 'my \$inner = <<END;
Inner content
END
print "From inner: \$inner"');
print "Result: $outer\n";

# Test 7: qx with different delimiters
print "\nTest 7: qx with different delimiters\n";
my $cmd7 = <<'DELIM';
echo "Different delimiter test"
DELIM
chomp $cmd7;
my $result7 = qx{$cmd7};
print "Result: $result7\n";

# Test 8: Heredoc in system() vs qx
print "\nTest 8: Heredoc in system() vs qx\n";
my $cmd8 = <<'SYS';
echo "From heredoc in system"
SYS
chomp $cmd8;
print "Using system: ";
system($cmd8);
print "Using qx: ";
my $qx8 = qx($cmd8);
print $qx8;

# Test 9: Complex command composition
print "\nTest 9: Complex command composition\n";
my $data = <<'DATA';
line1
line2
line3
DATA
my $filter = <<'FILTER';
grep line
FILTER
chomp $data;
chomp $filter;
my $result9 = qx(echo '$data' | $filter);
print "Result: $result9\n";

# Test 10: Heredoc with special characters in qx
print "\nTest 10: Heredoc with special characters in qx\n";
my $special = <<'SPECIAL';
echo "Special: \$HOME and \`date\`"
SPECIAL
chomp $special;
my $result10 = qx($special);
print "Result: $result10\n";