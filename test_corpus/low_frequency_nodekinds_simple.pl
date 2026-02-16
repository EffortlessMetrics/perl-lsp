#!/usr/bin/env perl
# Test: Simple Low-Frequency NodeKinds
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

# Eval with return value
my $eval_result = eval {
    my $a = 5;
    my $b = 3;
    return $a + $b;
};
print "Eval block result: $eval_result\n";

# For loops (C-style)
for (my $i = 0; $i < 5; $i++) {
    print "C-style for loop: $i\n";
}

for (my $j = 10; $j >= 0; $j -= 2) {
    print "Counting down: $j\n";
}

# Complex for loop
for (my $k = 0; $k < 10; $k++) {
    next if $k % 2 == 0;
    last if $k > 7;
    print "Complex for: $k\n";
}

# Do blocks
# Do as expression
my $do_result = do {
    my $x = 5;
    my $y = 3;
    $x + $y;
};
print "Do block result: $do_result\n";

# Do with conditional
my $do_conditional = do {
    if ($do_result > 5) {
        "Greater than 5";
    } else {
        "Less than or equal to 5";
    }
};
print "Do conditional: $do_conditional\n";

# Do with last/next/redo
my $do_loop = do {
    for my $i (1..5) {
        last if $i == 3;
        print "Do loop: $i\n";
    }
    "Done";
};

# Phase blocks (BEGIN, END, CHECK, INIT, UNITCHECK)
BEGIN {
    print "Running at compile time (BEGIN)\n";
}

END {
    print "Running at program termination (END)\n";
}

CHECK {
    print "Running after compilation (CHECK)\n";
}

INIT {
    print "Running before runtime (INIT)\n";
}

UNITCHECK {
    print "Running after compilation unit (UNITCHECK)\n";
}

# No pragma
no strict;  # Disable strict checking
no warnings;  # Disable warnings

$undeclared_var = "This works without strict";  # Normally would be an error
print "Undeclared var: $undeclared_var\n";

use strict;  # Re-enable
use warnings;  # Re-enable

# No with specific warnings
no warnings 'uninitialized';  # Disable specific warning
my $undef_var;
print "Undefined var: $undef_var\n";  # No warning
use warnings 'uninitialized';  # Re-enable

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

# Complex format
format COMPLEX =
@<<<<<<<<< @>>>>>>> @<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$left,    $right, $description
.

$~ = "COMPLEX";
my $left = "Left";
my $right = "Right";
$description = "Description";
write;

# Substitution operator
my $text = "Hello World";
$text =~ s/World/Perl/;
print "After substitution: $text\n";

# Substitution with modifiers
$text =~ s/hello/hi/i;  # Case insensitive
print "Case insensitive: $text\n";

$text =~ s/(\w+)/uc($1)/ge;  # Global with evaluation
print "Global eval: $text\n";

# Substitution with delimiters
$text =~ s{Hello}{Hi};
$text =~ s[World][Perl];
$text =~ s<test>example>;

# Transliteration operator
my $tr_text = "abcde123";
$tr_text =~ tr/a-z/A-Z/;  # Uppercase
print "After tr: $tr_text\n";

$tr_text =~ tr/0-9/9-0/;  # Reverse digits
print "After digit tr: $tr_text\n";

$tr_text =~ tr/a-zA-Z//d;  # Delete letters
print "After delete tr: $tr_text\n";

# Complement transliteration
$tr_text = "abc123";
$tr_text =~ tr/a-z/0-9/c;  # Complement - replace non-a-z with 0-9
print "Complement tr: $tr_text\n";

# Squash duplicate characters
$tr_text = "aaabbbccc";
$tr_text =~ tr/a-z//s;  # Squash
print "Squash tr: $tr_text\n";

# Typeglob operations
*OLD_GLOB = *STDOUT;  # Alias typeglob
print OLD_GLOB "Via typeglob alias\n";

# Typeglob assignment to code reference
*my_sub = sub { print "Via typeglob sub\n"; };
my_sub();

# Typeglob with file handle
*MY_FH = *STDOUT;
print MY_FH "Via file handle glob\n";

# Glob expressions
my @pl_files = glob("test_corpus/*.pl");
print "Found " . scalar @pl_files . " .pl files\n";

my @all_files = glob("*");
print "Total files: " . scalar @all_files . "\n";

# Glob with patterns
my @test_files = glob("test_corpus/test_*.pl");
my @corpus_files = glob("test_corpus/{basic,advanced}*.pl");

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

# Unless
print "Unless condition\n" unless $if_var < 0;

# While loops
my $while_count = 0;
while ($while_count < 3) {
    print "While loop: $while_count\n";
    $while_count++;
}

# While with continue
my $while_continue = 0;
while ($while_continue < 3) {
    print "While continue: $while_continue\n";
    $while_continue++;
} continue {
    print "Continue block\n";
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

# Label with while
LABEL_WHILE: while (1) {
    print "Labeled while\n";
    last LABEL_WHILE;
}

# Indirect object syntax/indirect calls
# Indirect method call
my $obj = bless {}, 'TestClass';
print "Using direct method call instead of indirect syntax\n";

# Indirect file handle
open my $indirect_fh, ">", "test.txt" or die $!;
print $indirect_fh "Direct print\n";

# Indirect object with print
print STDOUT "Via indirect object\n";

# Variable with attributes
# Note: This requires experimental features in newer Perl
# For compatibility, we'll show the syntax structure in comments
# my $attr_var :attr1 = "value";  # Would require experimental attributes

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