package SymbolicDerefAssign;

# Pattern: ${$name} = 1
# Variable created/accessed via symbolic dereference.
# The variable exists dynamically — suppress undefined-symbol diagnostic.
my $name = "dynamic_var";
${$name} = 1;
my $val = ${$name};
