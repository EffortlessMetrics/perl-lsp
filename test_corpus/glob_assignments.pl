#!/usr/bin/env perl
# Test glob assignments (Issue #448)
# Typeglob assignments for symbol table manipulation and aliasing

# AC1: Simple typeglob assignment
*foo = *bar;

# AC2: Typeglob with reference assignment
*PI = \3.14159;
*func = \&other_func;

# AC3: Dynamic typeglob
*{$name} = \&function;

# AC5: Typeglob dereferencing
my $val = ${*foo};
my @arr = @{*bar};

# Additional examples
*My::Package::func = *Other::Package::func;
local *FH;
my $ref = \*STDOUT;

# Ensure multiplication still works (AC6 - backward compatibility)
my $x = 2 * 3;

__END__
Parser assertions:
1. *foo = *bar parsed as typeglob assignment (not multiplication)
2. *PI = \3.14159 parsed with reference operator
3. Dynamic typeglob syntax accepted
4. Typeglob dereferencing handled
5. Qualified typeglob names supported
6. Multiplication operator still works correctly
