#!/usr/bin/perl
# examples/perl/oop.pl
#
# Demonstrates: Moose/Moo OOP classes, method calls, inheritance, roles.
#
# LSP features exercised:
#   - hover       : hover over method names for signatures/docs
#   - go-to-def   : jump from method call site to method definition
#   - rename      : rename a method across all call sites
#   - completion  : type "$self->" to see available method list
#   - diagnostics : unused attributes, type mismatches

use strict;
use warnings;

# ---------------------------------------------------------------------------
# Base class using Moose
# ---------------------------------------------------------------------------
{
    package Animal;

    use Moose;

    has 'name' => (
        is       => 'ro',
        isa      => 'Str',
        required => 1,
    );

    has 'sound' => (
        is      => 'rw',
        isa     => 'Str',
        default => '...',
    );

    has 'legs' => (
        is      => 'ro',
        isa     => 'Int',
        default => 4,
    );

    sub speak {
        my ($self) = @_;
        printf "%s says %s\n", $self->name, $self->sound;
    }

    sub describe {
        my ($self) = @_;
        printf "%s has %d legs\n", $self->name, $self->legs;
    }

    __PACKAGE__->meta->make_immutable;
}

# ---------------------------------------------------------------------------
# Subclass
# ---------------------------------------------------------------------------
{
    package Dog;

    use Moose;
    extends 'Animal';

    has 'breed' => (
        is  => 'ro',
        isa => 'Str',
    );

    sub new_with_defaults {
        my ($class, %args) = @_;
        $args{sound} //= 'woof';
        return $class->new(%args);
    }

    # Override parent method
    sub speak {
        my ($self) = @_;
        printf "%s (%s) barks: %s!\n",
            $self->name, $self->breed // 'mixed', $self->sound;
    }

    sub fetch {
        my ($self, $item) = @_;
        printf "%s fetches the %s\n", $self->name, $item // 'ball';
    }

    __PACKAGE__->meta->make_immutable;
}

# ---------------------------------------------------------------------------
# Moo role
# ---------------------------------------------------------------------------
{
    package Printable;

    use Moo::Role;

    requires 'name';   # consuming class must provide ->name

    sub print_info {
        my ($self) = @_;
        printf "[Printable] object name: %s\n", $self->name;
    }
}

# ---------------------------------------------------------------------------
# Moo class consuming a role
# ---------------------------------------------------------------------------
{
    package Cat;

    use Moo;
    with 'Printable';

    has 'name' => ( is => 'ro', required => 1 );
    has 'indoor' => ( is => 'rw', default => sub { 1 } );

    sub speak {
        my ($self) = @_;
        my $env = $self->indoor ? 'indoor' : 'outdoor';
        printf "%s (%s cat) says meow\n", $self->name, $env;
    }
}

# ---------------------------------------------------------------------------
# Usage -- exercises go-to-def, hover, completion
# ---------------------------------------------------------------------------

my $dog = Dog->new(name => 'Rex', breed => 'Labrador', sound => 'woof');
$dog->speak;       # hover: see Dog::speak signature
$dog->describe;    # hover: see Animal::describe (inherited)
$dog->fetch('stick');

my $cat = Cat->new(name => 'Whiskers');
$cat->speak;
$cat->print_info;  # from Printable role -- go-to-def jumps to role

# Polymorphism
my @animals = ($dog, $cat);
for my $animal (@animals) {
    $animal->speak;    # rename 'speak' -> all call sites updated
}
