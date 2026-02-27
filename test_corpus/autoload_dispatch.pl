#!/usr/bin/env perl
# Test: AUTOLOAD dispatch patterns, method resolution, and dynamic dispatch
# NodeKinds exercised: Subroutine, FunctionCall, MethodCall, Variable, Package, If, Return
# Coverage gap: AUTOLOAD is fundamental to Perl's dynamic dispatch but was missing

use strict;
use warnings;

# --- Basic AUTOLOAD ---
package Logger;

sub new {
    my ($class, %opts) = @_;
    return bless { level => $opts{level} // "info", messages => [] }, $class;
}

sub AUTOLOAD {
    my ($self, @args) = @_;
    our $AUTOLOAD;               # fully-qualified method name
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;         # strip package prefix

    return if $method eq 'DESTROY';  # never autoload DESTROY

    # Dynamic log-level methods: ->debug(), ->info(), ->warn(), ->error()
    if ($method =~ /^(debug|info|warn|error)$/) {
        push @{$self->{messages}}, "[$method] @args";
        return 1;
    }

    die "Unknown method: $method called on " . ref($self);
}

# Prevent AUTOLOAD from catching DESTROY
sub DESTROY { }

package AccessorGenerator;

sub new {
    my ($class, %fields) = @_;
    return bless { %fields }, $class;
}

# AUTOLOAD that installs real methods on first call
sub AUTOLOAD {
    my ($self, @args) = @_;
    our $AUTOLOAD;
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;

    return if $method eq 'DESTROY';

    # Check if it's a valid field
    if (exists $self->{$method}) {
        # Install a real accessor to avoid future AUTOLOAD overhead
        no strict 'refs';
        *{"AccessorGenerator::$method"} = sub {
            my $self = shift;
            if (@_) {
                $self->{$method} = $_[0];
                return $self;    # chainable
            }
            return $self->{$method};
        };
        # Now call the installed method
        return $self->$method(@args);
    }

    die "No field '$method' in " . ref($self);
}

sub DESTROY { }

# --- AUTOLOAD chaining across inheritance ---
package Base;

sub new { bless {}, shift }

sub AUTOLOAD {
    my ($self, @args) = @_;
    our $AUTOLOAD;
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;
    return if $method eq 'DESTROY';

    # Base knows about 'base_method'
    if ($method eq 'base_method') {
        return "from Base";
    }
    die "Base: unknown method '$method'";
}

sub DESTROY { }

package Derived;
our @ISA = ('Base');

sub new { bless {}, shift }

sub AUTOLOAD {
    my ($self, @args) = @_;
    our $AUTOLOAD;
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;
    return if $method eq 'DESTROY';

    # Derived knows about 'derived_method'
    if ($method eq 'derived_method') {
        return "from Derived";
    }

    # Fall through to parent's AUTOLOAD via SUPER
    my $super_method = "SUPER::$method";
    return $self->$super_method(@args);
}

sub DESTROY { }

# --- can() override with AUTOLOAD ---
package SmartAutoload;

my %known_methods = map { $_ => 1 } qw(foo bar baz);

sub new { bless {}, shift }

# Override can() to report AUTOLOAD-able methods
sub can {
    my ($self, $method) = @_;
    return $self->SUPER::can($method)    # check real methods first
        || ($known_methods{$method} ? sub { "autoloaded $method" } : undef);
}

sub AUTOLOAD {
    my ($self, @args) = @_;
    our $AUTOLOAD;
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;
    return if $method eq 'DESTROY';

    if ($known_methods{$method}) {
        return "autoloaded: $method(@args)";
    }
    die "Cannot autoload '$method'";
}

sub DESTROY { }

# --- AUTOLOAD for delegation/proxy ---
package Proxy;

sub new {
    my ($class, $target) = @_;
    return bless { target => $target }, $class;
}

sub AUTOLOAD {
    my ($self, @args) = @_;
    our $AUTOLOAD;
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;
    return if $method eq 'DESTROY';

    my $target = $self->{target};
    if ($target->can($method)) {
        return $target->$method(@args);
    }
    die "Proxy target cannot '$method'";
}

sub DESTROY { }

# --- AUTOLOAD with $AUTOLOAD parsing ---
package NamespacedDispatch;

sub new { bless {}, shift }

sub AUTOLOAD {
    my ($self, @args) = @_;
    our $AUTOLOAD;

    # Parse the fully-qualified name
    my ($pkg, $method) = $AUTOLOAD =~ /^(.*)::(.*)$/;

    return if $method eq 'DESTROY';

    # Use the package info for dispatch decisions
    return "pkg=$pkg method=$method args=@args";
}

sub DESTROY { }

# --- main test ---
package main;

# Basic AUTOLOAD
my $log = Logger->new(level => "debug");
$log->debug("test message");
$log->info("info message");
$log->error("error message");

# Accessor generation via AUTOLOAD
my $obj = AccessorGenerator->new(name => "test", value => 42);
my $name = $obj->name;         # first call: installs accessor
my $name2 = $obj->name;        # second call: uses installed accessor
$obj->value(100);              # setter via chaining

# Inheritance chain
my $derived = Derived->new;
my $dr = $derived->derived_method;
my $br = $derived->base_method;   # falls through to Base::AUTOLOAD

# can() with AUTOLOAD
my $smart = SmartAutoload->new;
if ($smart->can("foo")) {
    $smart->foo("arg1", "arg2");
}
my $cannot = $smart->can("nonexistent");  # returns undef

# Proxy pattern
my $target = AccessorGenerator->new(x => 10);
my $proxy = Proxy->new($target);
my $x = $proxy->x;

print "AUTOLOAD dispatch test complete\n";
