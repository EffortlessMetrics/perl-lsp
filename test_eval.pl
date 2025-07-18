#!/usr/bin/perl

# Eval block
eval {
    print "In eval block\n";
    die "Error!" if $error;
};

# Eval string
eval "print 'Hello from eval string\n'";

# Eval with assignment
my $result = eval { $x + $y };

# Eval error checking
eval {
    dangerous_operation();
};
if ($@) {
    print "Error: $@\n";
}