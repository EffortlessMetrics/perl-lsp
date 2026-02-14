#!/usr/bin/env perl
# Test: Basic Low-Frequency NodeKinds
# Impact: Ensures parser handles rarely-used but important Perl constructs
# NodeKinds: Ellipsis, Eval, For, Do, PhaseBlock, No, Format, Substitution, Transliteration,
#           Typeglob, Glob, If, While, LabeledStatement, IndirectCall, VariableWithAttributes

use strict;
use warnings;

# Ellipsis operator (yada yada)
sub todo_function {
    ...;  # Placeholder for unimplemented function
}

sub another_todo {
    print "Before yada\n";
    ...;  # Not reached
    print "After yada\n";
}

# Eval blocks
# Eval string
my $code = 'my $x = 10; return $x * 2;';
my $result = eval $code;
print "Eval string result: $result\n" if defined $result;
warn "Eval error: $@" if $@;

# Eval block (try-catch like)
eval {
    die "Test error";
};
if ($@) {
    print "Caught eval error: $@\n";
}

# For loops (C-style)
for (my $i = 0; $i < 5; $i++) {
    print "C-style for loop: $i\n";
}

# Do blocks
# Do as expression
my $do_result = do {
    my $x = 5;
    my $y = 3;
    $x + $y;
};
print "Do block result: $do_result\n";

# Phase blocks (BEGIN, END)
BEGIN {
    print "Running at compile time (BEGIN)\n";
}

END {
    print "Running at program termination (END)\n";
}

# No pragma
no strict;  # Disable strict checking
no warnings;  # Disable warnings

$undeclared_var = "This works without strict";  # Normally would be an error
print "Undeclared var: $undeclared_var\n";

use strict;  # Re-enable
use warnings;  # Re-enable

# Format statements
format STDOUT_TOP =
Page @<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$%
Title: Test Format Report
================================
.

format STDOUT =
@<<<<<<<<<<<<<<<<<<<<<<<<<<<<< @<<<<<<<<<<<<<
$name,                           $value
.

# Prepare format data
my $name = "Test";
my $value = 42;
$- = 0;  # Force page break
write;

# Substitution operator
my $text = "Hello World";
$text =~ s/World/Perl/;
print "After substitution: $text\n";

# Transliteration operator
my $tr_text = "abcde123";
$tr_text =~ tr/a-z/A-Z/;  # Uppercase
print "After tr: $tr_text\n";

# Typeglob operations
*OLD_GLOB = *STDOUT;  # Alias typeglob
print OLD_GLOB "Via typeglob alias\n";

# Typeglob assignment to code reference
*my_sub = sub { print "Via typeglob sub\n"; };
my_sub();

# Glob expressions
my @pl_files = glob("test_corpus/*.pl");
print "Found " . scalar @pl_files . " .pl files\n";

# If statements (comprehensive)
my $if_var = 10;

if ($if_var > 5) {
    print "Greater than 5\n";
} elsif ($if_var > 0) {
    print "Greater than 0\n";
} else {
    print "Less than or equal to 0\n";
}

# Single line if
print "Single line if\n" if $if_var > 5;

# While loops
my $while_count = 0;
while ($while_count < 3) {
    print "While loop: $while_count\n";
    $while_count++;
}

# Labeled statements
OUTER_LABEL: for my $i (1..3) {
    INNER_LABEL: for my $j (1..3) {
        if ($i == 2 && $j == 2) {
            last OUTER_LABEL;
        }
        print "Labeled: $i,$j\n";
    }
}

# Indirect object syntax/indirect calls
my $obj = bless {}, 'TestClass';
print "Using direct method call instead of indirect syntax\n";

# Indirect file handle
open my $indirect_fh, ">", "test.txt" or die $!;
print $indirect_fh "Direct print\n";

# Indirect object with print
print STDOUT "Via indirect object\n";

# Complex combinations
sub complex_function {
    my ($param) = @_;
    
    BEGIN {
        print "BEGIN in function scope\n";
    }
    
    eval {
        for (my $i = 0; $i < 3; $i++) {
            if ($i == 1) {
                next;
            }
            print "Complex: $i\n";
        }
    };
    
    warn "Eval error: $@" if $@;
    
    return do {
        $param * 2;
    };
}

my $complex_result = complex_function(5);
print "Complex result: $complex_result\n";

print "All low-frequency NodeKind tests completed\n";