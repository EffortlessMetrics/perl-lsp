#!/usr/bin/env perl

# Test complex string interpolation edge cases

# Basic interpolation (should work)
"Hello $name";
"Array: @items";
"Hash element: $hash{key}";

# Complex interpolation cases
"Complex: @{[ $x + $y ]}";
"Deref: @{$arrayref}";
"Hash deref: %{$hashref}";
"Code eval: @{[ func($x) ]}";
"Nested: @{[ $arr->[$i] ]}";

# Multiple interpolations
"Name: $first $last";
"List: @items, count: $#items";

# Escaped interpolation
"Literal \$dollar";
"Literal \@at";

# Special variables
"PID: $$";
"Error: $!";
"Match: $1, $2";

# Method calls in interpolation
"Result: $obj->method()";
"Chain: $obj->foo()->bar()";

# Array/hash slices in interpolation
"Slice: @array[0..2]";
"Hash slice: @hash{'a', 'b'}";

# Mixed quotes
qq{Hello $name};
qq[Array @items];
qq|Path: $ENV{PATH}|;