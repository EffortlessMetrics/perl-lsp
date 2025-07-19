#!/usr/bin/perl
use strict;
use warnings;

print "=== Testing complex nested heredoc cases ===\n\n";

# Test 1: Heredoc in regex match with code block
print "Test 1: Heredoc in regex match with code block\n";
my $text1 = "MATCH";
if ($text1 =~ /MATCH(?{ my $x = <<EOF; print "In regex: $x" })/) {
Heredoc in (?{...})
EOF
    print "Matched!\n";
}

# Test 2: Heredoc in map/grep blocks
print "\nTest 2: Heredoc in map/grep blocks\n";
my @items = qw(a b c);
my @results = map {
    my $content = <<ITEM;
Item: $_
ITEM
    chomp $content;
    $content;
} @items;
print "Results: @results\n";

# Test 3: Heredoc in sort comparison
print "\nTest 3: Heredoc in sort comparison\n";
my @words = qw(foo bar baz);
my @sorted = sort {
    my $cmp = <<CMP;
Comparing $a and $b
CMP
    print $cmp;
    $a cmp $b;
} @words;
print "Sorted: @sorted\n";

# Test 4: Heredoc in BEGIN/END blocks
print "\nTest 4: Heredoc in BEGIN/END blocks\n";
BEGIN {
    my $begin = <<'BEGIN';
In BEGIN block
BEGIN
    print "BEGIN: $begin";
}

END {
    my $end = <<'END_DOC';
In END block
END_DOC
    print "END: $end";
}

# Test 5: Heredoc in tied variable operations
print "\nTest 5: Heredoc in tied variable operations\n";
{
    package TieTest;
    sub TIESCALAR { bless {}, shift }
    sub FETCH { 
        my $content = <<TIED;
From tied variable
TIED
        chomp $content;
        return $content;
    }
    sub STORE { }
}
tie my $tied, 'TieTest';
print "Tied result: $tied\n";

# Test 6: Heredoc in overloaded operators
print "\nTest 6: Heredoc in overloaded operators\n";
{
    package OverloadTest;
    use overload '""' => sub {
        my $str = <<OVER;
Overloaded stringify
OVER
        chomp $str;
        return $str;
    };
    sub new { bless {}, shift }
}
my $obj = OverloadTest->new();
print "Overloaded: $obj\n";

# Test 7: Multiple nested contexts
print "\nTest 7: Multiple nested contexts\n";
my $complex = do {
    my $outer = eval {
        my $code = 'my $inner = <<INNER;
Deeply nested
INNER
$inner =~ s/nested/<<REPLACE/e;
REPLACED
REPLACE
$inner';
        eval $code;
    };
    $outer;
};
print "Complex result: $complex\n";

# Test 8: Heredoc in format definitions
print "\nTest 8: Heredoc in format definitions\n";
format TESTFMT =
@<<<<<<<<<<<<<<<<<<<
<<FMT
Heredoc in format
FMT
.
# Note: format with heredoc is tricky and may not work as expected

# Test 9: Heredoc in signal handlers
print "\nTest 9: Heredoc in signal handlers\n";
$SIG{USR1} = sub {
    my $sig = <<SIG;
Signal received
SIG
    print $sig;
};

# Test 10: Heredoc in AUTOLOAD
print "\nTest 10: Heredoc in AUTOLOAD\n";
{
    package AutoloadTest;
    our $AUTOLOAD;
    sub AUTOLOAD {
        my $method = $AUTOLOAD;
        my $response = <<AUTO;
Autoloaded: $method
AUTO
        print $response;
    }
    sub new { bless {}, shift }
}
my $auto = AutoloadTest->new();
$auto->test_method();

# Test 11: Heredoc in ternary operator
print "\nTest 11: Heredoc in ternary operator\n";
my $flag = 1;
my $ternary = $flag ? <<TRUE : <<FALSE;
True branch
TRUE
False branch
FALSE
print "Ternary result: $ternary\n";

# Test 12: Heredoc in list context with mixed operations
print "\nTest 12: Heredoc in list context with mixed operations\n";
my @mixed = (
    <<ONE,
First
ONE
    do { my $x = <<TWO; chomp $x; $x },
Second
TWO
    eval { <<THREE }
Third
THREE
);
print "Mixed list: ", join(" | ", @mixed), "\n";