#!/usr/bin/env perl
# Test: Enhanced Indirect Call Production Scenarios
# Impact: Comprehensive testing of legacy indirect object syntax and method calls
# NodeKinds: IndirectCall, IndirectObject, MethodCall
# 
# This file tests the parser's ability to handle:
# 1. Indirect object syntax with various built-in functions
# 2. Legacy method call patterns
# 3. Indirect filehandle operations
# 4. Complex indirect object expressions
# 5. Ambiguous syntax resolution
# 6. Performance considerations with indirect calls
# 7. Real-world legacy code patterns
# 8. Modern vs indirect call compatibility

use strict;
use warnings;

print "=== Enhanced Indirect Call Production Tests ===\n\n";

# Test 1: Basic indirect object syntax
print "=== Basic Indirect Object Syntax ===\n";

# Indirect object with print
my $message = "Hello from indirect object";
print STDOUT $message, "\n";
print STDERR "Error message to STDERR\n";

# Indirect object with printf
printf STDOUT "Formatted output: %s %d\n", "test", 42;

# Indirect object with sprintf
my $formatted = sprintf "String: %s, Number: %d", "value", 123;
print "Sprintf result: $formatted\n";

# Indirect object with system
system "echo", "System call with indirect syntax";

# Indirect object with exec (commented out to avoid terminating script)
# exec "echo", "This would exec and terminate";

print "\n";

# Test 2: Indirect filehandle operations
print "=== Indirect Filehandle Operations ===\n";

# Create a test file for indirect operations
my $test_file = 'indirect_test.txt';

# Indirect open
open my $fh, ">", $test_file or die "Cannot open $test_file: $!";
print $fh "Line 1 written with indirect print\n";
print $fh "Line 2 written with indirect print\n";
close $fh;

# Indirect read
open $fh, "<", $test_file or die "Cannot open $test_file for reading: $!";
my $line1 = <$fh>;
my $line2 = <$fh>;
close $fh;

print "Read from file: $line1";
print "Read from file: $line2";

# Indirect seek and tell
open $fh, "+<", $test_file or die "Cannot open $test_file for update: $!";
seek $fh, 0, 0;  # Seek to beginning
print $fh "Modified line\n";
tell $fh;  # Get current position
close $fh;

# Indirect file test operations
if (-f $test_file) {
    print "File exists: $test_file\n";
}
if (-r $test_file) {
    print "File is readable: $test_file\n";
}
if (-w $test_file) {
    print "File is writable: $test_file\n";
}

print "\n";

# Test 3: Legacy method call patterns
print "=== Legacy Method Call Patterns ===\n";

# Traditional indirect object method calls
package IndirectTestClass;
sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub method1 {
    my ($self, $arg) = @_;
    return "method1 called with: $arg";
}

sub method2 {
    my ($self, @args) = @_;
    return "method2 called with: " . join(', ', @args);
}

sub get_value {
    my ($self) = @_;
    return $self->{value} // 'undefined';
}

sub set_value {
    my ($self, $value) = @_;
    $self->{value} = $value;
    return $self;
}

package main;

# Create objects using indirect syntax
my $obj1 = new IndirectTestClass value => 42;
my $obj2 = new IndirectTestClass(name => 'test');

# Call methods using indirect syntax
my $result1 = method1 $obj1, "argument1";
my $result2 = method2 $obj2, "arg1", "arg2", "arg3";

print "Indirect method call 1: $result1\n";
print "Indirect method call 2: $result2\n";

# Chain indirect calls
my $chained_result = method1 (set_value $obj1, 100), "after_set";
print "Chained indirect calls: $chained_result\n";

print "\n";

# Test 4: Complex indirect object expressions
print "=== Complex Indirect Object Expressions ===\n";

# Indirect object with complex expressions
my $complex_obj = new IndirectTestClass(
    value => 10 + 20,
    name => 'computed_' . (5 * 3),
    data => [1, 2, 3, 4, 5]
);

# Indirect method with complex arguments
my $complex_result = method2 $complex_obj, 
    join('-', 'a', 'b', 'c'),
    $complex_obj->get_value() * 2,
    { nested => 'hash' };

print "Complex indirect call result: $complex_result\n";

# Indirect object with function calls as arguments
sub compute_value {
    my ($multiplier) = @_;
    return $multiplier * 10;
}

my $computed_obj = new IndirectTestClass(
    value => compute_value(5),
    name => uc('dynamic_name')
);

print "Computed object value: " . $computed_obj->get_value() . "\n";

# Indirect method with conditional arguments
my $conditional_arg = rand() > 0.5 ? 'high' : 'low';
my $conditional_result = method1 $computed_obj, $conditional_arg;
print "Conditional indirect call: $conditional_result\n";

print "\n";

# Test 5: Ambiguous syntax resolution
print "=== Ambiguous Syntax Resolution ===\n";

# This could be interpreted as either:
# 1. Indirect object: print $fh "message"
# 2. Function call: print($fh, "message")

open my $ambiguous_fh, ">", $test_file or die $!;
print $ambiguous_fh "This uses indirect object syntax\n";
close $ambiguous_fh;

# Function with same name as built-in
sub print {
    my (@args) = @_;
    return "Custom print called with: " . join(', ', @args);
}

# Now this calls our custom print function
my $custom_result = print "arg1", "arg2";
print "Custom print result: $custom_result\n";

# Use CORE:: to call built-in print
CORE::print "Built-in print via CORE:: prefix\n";

# Restore built-in print
undef &print;

print "\n";

# Test 6: Performance considerations
print "=== Performance Considerations ===\n";

# Benchmark indirect vs direct calls
sub benchmark_indirect_calls {
    my ($iterations) = @_;
    $iterations ||= 10000;
    
    # Create test object
    my $bench_obj = new IndirectTestClass value => 1;
    
    # Benchmark indirect method calls
    my $start = time();
    for (1..$iterations) {
        method1 $bench_obj, "test";
    }
    my $indirect_time = time() - $start;
    
    # Benchmark direct method calls
    $start = time();
    for (1..$iterations) {
        $bench_obj->method1("test");
    }
    my $direct_time = time() - $start;
    
    print "Benchmark results ($iterations iterations):\n";
    print "  Indirect calls: $indirect_time seconds\n";
    print "  Direct calls: $direct_time seconds\n";
    print "  Performance ratio: " . sprintf('%.2f', $indirect_time / $direct_time) . "x\n";
}

benchmark_indirect_calls(50000);

print "\n";

# Test 7: Real-world legacy patterns
print "=== Real-World Legacy Patterns ===\n";

# Pattern 1: File operations with indirect syntax
sub legacy_file_processor {
    my ($filename, $operation) = @_;
    
    open my $input, "<", $filename or die "Cannot open $filename: $!";
    open my $output, ">", "$filename.processed" or die "Cannot create output: $!";
    
    while (my $line = <$input>) {
        chomp $line;
        # Process line
        my $processed = $operation->($line);
        print $output $processed, "\n";
    }
    
    close $input;
    close $output;
    
    return "$filename processed successfully";
}

# Create test file for legacy processing
open my $legacy_file, ">", $test_file or die $!;
print $legacy_file "Line 1\n";
print $legacy_file "Line 2\n";
print $legacy_file "Line 3\n";
close $legacy_file;

my $process_result = legacy_file_processor($test_file, sub {
    my ($line) = @_;
    return uc($line);
});

print "Legacy processing result: $process_result\n";

# Pattern 2: Database operations with indirect syntax
package MockDBI;
sub connect {
    my ($class, @args) = @_;
    return bless { connected => 1, args => \@args }, $class;
}

sub prepare {
    my ($self, $sql) = @_;
    return bless { sql => $sql, db => $self }, 'MockStatement';
}

sub execute {
    my ($self, @params) = @_;
    return bless { statement => $self, params => \@params }, 'MockResult';
}

package MockStatement;
sub execute {
    my ($self, @params) = @_;
    return bless { statement => $self, params => \@params }, 'MockResult';
}

package MockResult;
sub fetchrow_array {
    my ($self) = @_;
    return wantarray ? ('row1_col1', 'row1_col2') : 'row1_col1';
}

package main;

# Legacy database pattern
my $dbh = connect MockDBI "dbi:mysql:database=test", "user", "pass";
my $sth = prepare $dbh "SELECT * FROM users WHERE id = ?";
my $result = execute $sth 123;
my @row = fetchrow_array $result;

print "Legacy DB pattern result: " . join(', ', @row) . "\n";

# Pattern 3: Object construction with indirect syntax
sub legacy_object_factory {
    my ($type, %config) = @_;
    
    # Simulate different object types
    if ($type eq 'logger') {
        return new IndirectTestClass(
            type => 'logger',
            level => $config{level} || 'INFO',
            output => $config{output} || 'STDOUT'
        );
    } elsif ($type eq 'cache') {
        return new IndirectTestClass(
            type => 'cache',
            size => $config{size} || 100,
            ttl => $config{ttl} || 3600
        );
    } else {
        return new IndirectTestClass(type => $type, %config);
    }
}

my $logger = legacy_object_factory('logger', level => 'DEBUG', output => 'file.log');
my $cache = legacy_object_factory('cache', size => 1000, ttl => 7200);

print "Legacy logger type: " . $logger->{type} . "\n";
print "Legacy cache type: " . $cache->{type} . "\n";

print "\n";

# Test 8: Modern vs indirect call compatibility
print "=== Modern vs Indirect Compatibility ===\n";

# Modern approach
sub modern_create_object {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub modern_method {
    my ($self, $arg) = @_;
    return "Modern: $arg";
}

# Both approaches should work
my $modern_obj = modern_create_object('ModernClass', value => 42);
my $indirect_obj = new IndirectTestClass value => 24;

# Modern method call
my $modern_result = $modern_obj->modern_method('modern_call');

# Indirect method call (using our custom method)
my $indirect_result = method1 $indirect_obj, 'indirect_call';

print "Modern result: $modern_result\n";
print "Indirect result: $indirect_result\n";

# Mixed usage
my $mixed_result = method1 $modern_obj, 'mixed_usage';
print "Mixed usage result: $mixed_result\n";

# Cleanup test files
unlink $test_file if -f $test_file;
unlink "$test_file.processed" if -f "$test_file.processed";

print "\n=== Enhanced Indirect Call Production Tests Completed ===\n";
print "This file demonstrates comprehensive indirect call patterns\n";
print "found in legacy Perl codebases and their modern equivalents.\n";