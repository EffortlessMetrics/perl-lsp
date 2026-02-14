#!/usr/bin/env perl
# Test: Comprehensive Try/Catch/Finally Blocks
# Impact: Ensures parser handles modern exception handling
# NodeKinds: Try

use strict;
use warnings;
use feature 'try';
no warnings 'experimental::try';

# Basic try/catch
try {
    die "Basic error";
}
catch ($e) {
    print "Caught: $e\n";
}

# Try/catch/finally
try {
    my $result = 10 / 0;  # This will cause a warning, not an exception
    print "Result: $result\n";
}
catch ($e) {
    print "Caught error: $e\n";
}
finally {
    print "Cleanup code\n";
}

# Nested try/catch
try {
    try {
        die "Inner exception";
    }
    catch ($inner) {
        print "Inner catch: $inner\n";
        die "Rethrowing with context";
    }
}
catch ($outer) {
    print "Outer catch: $outer\n";
}

# Try with multiple catch blocks (Perl 5.34+)
try {
    die "Some error";
}
catch ($e) {
    if ($e =~ /error/) {
        print "Caught error pattern: $e\n";
    } else {
        print "Default catch: $e\n";
    }
}

# Try with variable declaration in catch
try {
    die "Test error";
}
catch (my $error) {
    print "Error variable: $error\n";
}

# Try in subroutine context
sub safe_operation {
    my ($x, $y) = @_;
    try {
        return $x / $y;
    }
    catch ($e) {
        warn "Division failed: $e";
        return undef;
    }
}

# Try with complex expressions
try {
    my $complex = {
        data => [1, 2, 3],
        compute => sub { 
            die "Compute error" if $_[0] == 0;
            return 100 / $_[0];
        }
    };
    return $complex->{compute}->(0);
}
catch ($e) {
    print "Complex error: $e\n";
}

# Try with finally only
try {
    print "Doing work\n";
}
finally {
    print "Always cleanup\n";
}

# Try with return in catch
sub try_with_return {
    try {
        die "Error in function";
    }
    catch ($e) {
        print "Function caught: $e\n";
        return "error handled";
    }
    finally {
        print "Function cleanup\n";
    }
}

# Try with next/last in loops
foreach my $item (1..5) {
    try {
        die if $item == 3;
        print "Processing $item\n";
    }
    catch ($e) {
        print "Skipped $item due to error\n";
        next;
    }
}

# Try with eval fallback
try {
    die "Try block error";
}
catch ($e) {
    print "Try caught: $e\n";
}

# Fallback to eval if try not available
eval {
    die "Eval block error";
};
if ($@) {
    print "Eval caught: $@\n";
}

# Try with object-oriented exceptions
package MyError;
use overload '""' => sub { "MyError: $_[0]->{message}" };

sub new {
    my ($class, $message) = @_;
    return bless { message => $message }, $class;
}

package main;

try {
    die MyError->new("Custom error message");
}
catch ($e) {
    print "Custom error: $e\n";
}

# Try with warnings as exceptions
{
    use warnings 'FATAL' => 'all';
    
    try {
        my $undefined = $not_defined;  # This will be fatal
    }
    catch ($e) {
        print "Warning as exception: $e\n";
    }
}

print "All try/catch tests completed\n";