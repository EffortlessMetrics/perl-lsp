#!/usr/bin/perl
# Corpus strengthening fixture for issue #1383.
#
# Combines special package sections and lifecycle phasers in a single realistic
# module context so the parser is exercised on their interaction, not just each
# construct in isolation: phaser blocks (BEGIN/UNITCHECK/CHECK/INIT/END),
# AUTOLOAD/DESTROY magic methods, a package version token, and a terminal
# __END__ data section.
use strict;
use warnings;

package Acme::Special;

our $VERSION = '1.00';

# Lifecycle phasers — each parses as a PhaseBlock node.
BEGIN     { our @order; push @order, 'BEGIN'; }
UNITCHECK { our @order; push @order, 'UNITCHECK'; }
CHECK     { our @order; push @order, 'CHECK'; }
INIT      { our @order; push @order, 'INIT'; }
END       { our @order; push @order, 'END'; }

# AUTOLOAD magic dispatch for unknown methods.
our $AUTOLOAD;
sub AUTOLOAD {
    my $self = shift;
    my $name = $AUTOLOAD;
    $name =~ s/.*:://;
    return if $name eq 'DESTROY';
    return "autoloaded:$name";
}

# DESTROY magic destructor.
sub DESTROY {
    my $self = shift;
    return;
}

sub new {
    my $class = shift;
    return bless {}, $class;
}

package main;

# Inline version requirement appearing between statements.
use v5.10;

my $obj = Acme::Special->new;
my $value = $obj->some_autoloaded_method;

__END__

This is the literal __END__ body. Normal statement parsing stops here, and the
remaining lines are captured as the data section rather than parsed as code.
=pod
Even POD-like markers below __END__ are inert text.
=cut
