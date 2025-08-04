#!/usr/bin/perl
use strict;
use warnings;

# Test file for VSCode extension features

package TestPackage;

# This should show "X references" code lens
sub test_basic {
    print "Running basic test\n";
    ok(1, "basic test");
}

# This should show "▶ Run Test" and "X references" code lenses
sub helper_function {
    return 42;
}

# Another test function
sub test_advanced {
    my $result = helper_function();
    is($result, 42, "helper returns 42");
}

# Regular subroutine
sub process_data {
    my ($data) = @_;
    
    # This will test syntax highlighting and completion
    my $hash = {
        name => 'test',
        value => 123,
    };
    
    return $hash->{value} * 2;
}

# Main execution
sub main {
    test_basic();
    test_advanced();
    
    my $result = process_data({ value => 10 });
    print "Result: $result\n";
}

main() if !caller;

1;