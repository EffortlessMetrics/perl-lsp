#!/bin/bash

# Test script for LSP features
echo "Testing Perl Language Server v0.6.0 features..."

# Create a test directory
mkdir -p lsp_test
cd lsp_test

# Create a test Perl file with various features
cat > test_features.pl << 'EOF'
#!/usr/bin/env perl
use strict;
use warnings;
use Test::More;

# Function for call hierarchy testing
sub process_data {
    my ($data) = @_;
    validate_input($data);
    transform_data($data);
    return calculate_result($data);
}

sub validate_input {
    my ($input) = @_;
    die "Invalid input" unless defined $input;
}

sub transform_data {
    my ($data) = @_;
    $data->{transformed} = 1;
}

sub calculate_result {
    my ($data) = @_;
    return $data->{value} * 2;
}

# Test functions
sub test_basic_math {
    is(2 + 2, 4, "Basic addition works");
}

sub test_string_operations {
    my $str = "Hello";
    is(length($str), 5, "String length is correct");
}

# Call the functions to demonstrate call hierarchy
my $result = process_data({ value => 10 });

# Run tests
test_basic_math();
test_string_operations();

done_testing();
EOF

# Create package file for workspace symbols
cat > MyPackage.pm << 'EOF'
package MyPackage;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub method_one {
    my ($self, $param) = @_;
    return $param * 2;
}

sub method_two {
    my ($self) = @_;
    return $self->{value};
}

1;
EOF

# Create test file
cat > test_suite.t << 'EOF'
#!/usr/bin/env perl
use strict;
use warnings;
use Test::More;

# Test basic functionality
ok(1, "True is true");
is(2 + 2, 4, "Math works");

# Test string operations
my $str = "test";
like($str, qr/test/, "Regex matching works");

# Test array operations
my @arr = (1, 2, 3);
is(scalar(@arr), 3, "Array has correct size");

done_testing();
EOF

echo "Test files created."
echo ""
echo "To test the LSP features:"
echo "1. Open VSCode in this directory: code ."
echo "2. Install the Perl Language Server extension"
echo "3. Open test_features.pl"
echo ""
echo "Features to test:"
echo "- Syntax highlighting (semantic tokens)"
echo "- Hover over functions to see signatures"
echo "- Right-click on 'process_data' and select 'Show Call Hierarchy'"
echo "- Check for inlay hints showing parameter names"
echo "- Open Testing panel to see discovered tests"
echo "- Ctrl+Shift+P -> 'Perl: Run Test' to execute tests"
echo "- Ctrl+Click on function names to navigate"
echo "- Type 'process' and see autocomplete"
echo "- Workspace symbols: Ctrl+T and search for 'method'"
echo ""
echo "Configuration:"
echo "- Open settings (Ctrl+,) and search for 'perl'"
echo "- Adjust inlay hints, test runner settings, etc."

cd ..