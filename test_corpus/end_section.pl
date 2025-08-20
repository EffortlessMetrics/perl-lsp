#!/usr/bin/env perl
# Test: __END__ section (different from __DATA__)
# Impact: Nothing after __END__ is accessible to the program

use strict;
use warnings;

# Main program
my $x = 42;
print "Value: $x\n";

sub calculate {
    return shift() * 2;
}

# Note: __END__ is different from __DATA__
# - __DATA__ creates the DATA filehandle
# - __END__ does not create any filehandle
# - __END__ is truly the end of compilation

__END__

This section is completely ignored by Perl
It cannot be read via any filehandle
Often used for:
- Documentation
- Cut code that might be restored
- Notes and TODOs
- Example usage

# Old code kept for reference
sub old_implementation {
    my ($param) = @_;
    # This was the old way
    return $param + 1;
}

=pod

Even POD sections here are not processed

=cut

my $this_is_not_code = "Perl ignores everything after __END__";

BEGIN {
    # Even BEGIN blocks here are ignored
    print "This will never run\n";
}

# Common use: embedding other languages' code for reference
class Example {
    public static void main(String[] args) {
        System.out.println("Java example");
    }
}

def python_example():
    """Python code for comparison"""
    return "Not parsed as Perl"

// JavaScript version
function jsExample() {
    console.log("Also ignored");
}

# Malformed syntax is fine here
sub { {{ unclosed everything
"no ending quote
/* no closing comment
my $var with spaces in name;

Parser assertions:
1. No syntax errors after __END__ marker
2. Document symbols don't include anything after __END__
3. Folding treats entire __END__ section as one region
4. No diagnostics for malformed code after __END__
5. DATA filehandle is NOT available (unlike __DATA__)