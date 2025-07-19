#!/usr/bin/perl

# Test 1: use/require statements (FIXED)
use strict;
use warnings;
use Data::Dumper;
use Foo::Bar::Baz qw(import1 import2);
require Module;
require Some::Other::Module;

# Test 2: package blocks (FIXED)
package Foo {
    sub new {
        my $class = shift;
        return bless {}, $class;
    }
}

package Bar::Baz {
    sub method {
        print "Hello from Bar::Baz\n";
    }
}

# Traditional package syntax still works
package Traditional;
sub old_style {
    print "Traditional package\n";
}

# Test 3: Function calls without parentheses
# These should work with builtin list operators
print "Hello", " ", "World\n";
say "Modern Perl" if $];
warn "Warning message\n";
die "Fatal error\n" if 0;

# Common functions that should work without parens
my $ref = bless {name => 'Test'}, 'MyClass';
open my $fh, '<', 'file.txt' or die;
close $fh;

push @array, 1, 2, 3;
my $first = shift @array;
my @sorted = sort @array;
my $joined = join ',', @array;

# Test 4: Complex real-world patterns
use constant PI => 3.14159;
use parent 'Base::Class';
use feature 'say';

package My::App {
    use Moo;
    
    has name => (is => 'ro');
    
    sub run {
        my $self = shift;
        say "Running ", $self->name;
    }
}

# Test 5: Verify statement modifiers still work
print "OK\n" if 1;
die "Error\n" unless 1;
next while 0;
last until 1;

# Test 6: Method calls (should already work)
my $obj = My::App->new(name => 'Test');
$obj->run();

print "All tests included\n";