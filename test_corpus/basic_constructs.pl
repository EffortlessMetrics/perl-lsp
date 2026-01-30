#!/usr/bin/env perl
# Test: Basic constructs (for, ternary, ellipsis, etc.)
# Impact: Covers missing NodeKinds in corpus audit

use v5.38;

# For loop (NodeKind::For)
for (my $i = 0; $i < 10; $i++) {
    print $i;
}

# Ternary operator (NodeKind::Ternary)
my $x = 10;
my $y = $x > 5 ? "greater" : "smaller";

# Ellipsis (NodeKind::Ellipsis)
sub unimplemented {
    ...
}

# Variable with attributes (NodeKind::VariableWithAttributes)
my $shared_var :shared = 42;
my ($x :shared, $y :locked) = @_;

# Eval (NodeKind::Eval)
eval {
    die "error";
};

# Named parameters (NodeKind::NamedParameter)
sub greet_named(:$name) {
    print "Hello, $name";
}

use feature 'class';
class Person {
    field $name :param;
}

# Missing constructs (for error recovery testing)
# (Assuming these are produced when parser encounters errors)
# my $missing_stmt = ;
# sub { my $ } # Missing identifier
# if ($x) # Missing block

# Unknown rest
# (Assuming this is a catch-all for unparsed content)

# Heredoc depth limit (NodeKind::HeredocDepthLimit)
my $h1 = <<EOF1;
my $h2 = <<EOF2;
# ... (lots of them)
EOF2
EOF1

1;
