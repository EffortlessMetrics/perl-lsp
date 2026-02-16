#!/usr/bin/env perl
# Test: VariableListDeclaration NodeKind
# Impact: Ensures parser handles multiple variable declarations in single statements
# NodeKinds: VariableListDeclaration
# 
# This file tests the parser's ability to handle:
# 1. Multiple scalar declarations with list assignment
# 2. Mixed type declarations (scalars, arrays, hashes)
# 3. Nested list declarations
# 4. Complex assignment patterns
# 5. Error cases and edge conditions

use strict;
use warnings;

# Basic VariableListDeclaration - multiple scalars
# This tests the core VariableListDeclaration NodeKind
my ($x, $y, $z) = (1, 2, 3);
print "Basic list assignment: x=$x, y=$y, z=$z\n";

# VariableListDeclaration with array assignment
my @array_data = (10, 20, 30, 40);
my ($first, $second, $rest) = @array_data;
print "Array destructuring: first=$first, second=$second, rest=$rest\n";

# VariableListDeclaration with hash assignment
my %hash_data = (name => 'John', age => 30, city => 'NYC');
my ($name, $age, $city) = @hash_data{'name', 'age', 'city'};
print "Hash destructuring: name=$name, age=$age, city=$city\n";

# Mixed type VariableListDeclaration
my ($scalar1, @array1, %hash1) = ('scalar', (1, 2, 3), (key => 'value'));
print "Mixed types: scalar1=$scalar1, array1=@array1, hash1=" . join(',', %hash1) . "\n";

# VariableListDeclaration with function call return values
sub get_coordinates {
    return (10.5, 20.7, 30.2);
}
my ($lat, $lon, $alt) = get_coordinates();
print "Function return destructuring: lat=$lat, lon=$lon, alt=$alt\n";

# VariableListDeclaration with complex expressions
my ($sum, $product, $difference) = (5 + 3, 4 * 6, 10 - 2);
print "Expression assignment: sum=$sum, product=$product, difference=$difference\n";

# VariableListDeclaration with string operations
my ($concat, $repeat, $substr) = ('Hello' . ' World', 'A' x 5, substr('Perl', 1, 2));
print "String operations: concat=$concat, repeat=$repeat, substr=$substr\n";

# VariableListDeclaration with default values using defined-or
my ($default_a, $default_b, $default_c) = (undef, 0, 'default');
$default_a //= 'fallback_a';
$default_b //= 'fallback_b';
$default_c //= 'fallback_c';
print "Default values: a=$default_a, b=$default_b, c=$default_c\n";

# Nested VariableListDeclaration
my ($outer1, $outer2, ($inner1, $inner2)) = (1, 2, (3, 4));
print "Nested declaration: outer1=$outer1, outer2=$outer2, inner1=$inner1, inner2=$inner2\n";

# VariableListDeclaration with references
my ($scalar_ref, $array_ref, $hash_ref) = (\$scalar1, \@array1, \%hash1);
print "Reference assignment: scalar_ref=$$scalar_ref, array_ref=" . join(',', @$array_ref) . "\n";

# VariableListDeclaration with regex capture
my $text = "John:30:Engineer";
my ($captured_name, $captured_age, $captured_job) = $text =~ /^(\w+):(\d+):(\w+)$/;
print "Regex capture: name=$captured_name, age=$captured_age, job=$captured_job\n";

# VariableListDeclaration with list operations
my @source = (1, 2, 3, 4, 5);
my ($head, @tail) = @source;
my (@body, $last) = @source;
print "List operations: head=$head, tail=@tail, body=@body, last=$last\n";

# VariableListDeclaration with splice operations
my ($splice1, $splice2, @splice_rest) = splice(@source, 1, 2);
print "Splice results: splice1=$splice1, splice2=$splice2, splice_rest=@splice_rest\n";

# VariableListDeclaration with map/grep
my @numbers = (1, 2, 3, 4, 5);
my ($doubled_first, $doubled_second) = map { $_ * 2 } @numbers[0, 1];
my ($even_first, $even_second) = grep { $_ % 2 == 0 } @numbers;
print "Map/Grep results: doubled=$doubled_first,$doubled_second, even=$even_first,$even_second\n";

# VariableListDeclaration with sort
my @unsorted = (3, 1, 4, 1, 5, 9, 2);
my ($smallest, $largest) = (sort @unsorted)[0, -1];
print "Sort results: smallest=$smallest, largest=$largest\n";

# VariableListDeclaration with shift/unshift/push/pop
my @stack = (1, 2, 3);
my ($popped, $shifted) = (pop(@stack), shift(@stack));
print "Stack operations: popped=$popped, shifted=$shifted, remaining=@stack\n";

# VariableListDeclaration with file operations
open my $fh, '<', 'test_corpus/basic_constructs.pl' or die "Cannot open file: $!";
my ($first_line, $second_line) = <$fh>;
close $fh;
print "File lines: first_line=" . (defined $first_line ? substr($first_line, 0, 20) : 'undef') . "\n";

# VariableListDeclaration with time functions
my ($sec, $min, $hour, $mday, $mon, $year) = localtime();
print "Time components: hour=$hour, min=$min, sec=$sec\n";

# VariableListDeclaration with array slicing
my @large_array = (1..20);
my ($slice1, $slice2, $slice3) = @large_array[1, 5, 10];
print "Array slicing: slice1=$slice1, slice2=$slice2, slice3=$slice3\n";

# VariableListDeclaration with hash slicing
my %large_hash = (a => 1, b => 2, c => 3, d => 4, e => 5);
my ($hash_slice1, $hash_slice2, $hash_slice3) = @large_hash{'a', 'c', 'e'};
print "Hash slicing: hash_slice1=$hash_slice1, hash_slice2=$hash_slice2, hash_slice3=$hash_slice3\n";

# VariableListDeclaration with each function
my %test_hash = (x => 10, y => 20, z => 30);
my ($each_key, $each_value) = each %test_hash;
print "Each function: key=$each_key, value=$each_value\n";

# VariableListDeclaration with glob operations
my @glob_files = glob('test_corpus/*.pl');
my ($first_file, $second_file) = @glob_files[0, 1];
print "Glob results: first_file=" . (defined $first_file ? $first_file : 'none') . "\n";

# VariableListDeclaration with error handling
eval {
    my ($error_test1, $error_test2) = (undef, undef);
    $error_test1 = "success";
    1;  # Return true on success
} or do {
    print "Error in VariableListDeclaration test: $@\n";
};

# Complex VariableListDeclaration combining multiple concepts
sub complex_data_structure {
    return (
        ['array', 'elements'],
        { hash => 'value', another => 'key' },
        \"scalar_reference",
        sub { return "subroutine_reference" }
    );
}

my ($array_ref_complex, $hash_ref_complex, $scalar_ref_complex, $sub_ref_complex) = complex_data_structure();
print "Complex structure: array=" . join(',', @$array_ref_complex) . 
      ", hash=" . $hash_ref_complex->{hash} . 
      ", scalar_ref=" . $$scalar_ref_complex . 
      ", sub_ref=" . $sub_ref_complex->() . "\n";

print "VariableListDeclaration tests completed successfully\n";