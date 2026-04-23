#!/usr/bin/perl
use strict;
use warnings;
use feature 'say';

# Sample Perl file to test VSCode extension features

package Calculator;

=head1 NAME

Calculator - A simple calculator class

=head1 SYNOPSIS

    my $calc = Calculator->new();
    my $result = $calc->add(5, 3);

=cut

# Constructor
sub new {
    my $class = shift;
    my $self = {
        _history => [],
    };
    bless $self, $class;
    return $self;
}

# Add two numbers
sub add {
    my ($self, $x, $y) = @_;
    my $result = $x + $y;
    push @{$self->{_history}}, "add($x, $y) = $result";
    return $result;
}

# Subtract two numbers
sub subtract {
    my ($self, $x, $y) = @_;
    my $result = $x - $y;
    push @{$self->{_history}}, "subtract($x, $y) = $result";
    return $result;
}

# Get calculation history
sub get_history {
    my $self = shift;
    return @{$self->{_history}};
}

# Modern Perl features test
sub modern_features {
    # Try/catch (Perl 5.34+)
    eval {
        die "Test error";
    };
    if ($@) {
        say "Caught error: $@";
    }
    
    # Signatures (Perl 5.20+)
    state $counter = 0;
    $counter++;
    
    # Smart match (~~)
    my @array = (1, 2, 3);
    say "Found!" if 2 ~~ @array;
    
    # Defined-or operator
    my $value = undef // "default";
    
    # Postfix dereferencing
    my $arrayref = [1, 2, 3];
    my @values = $arrayref->@*;
}

# Main program
package main;

my $calc = Calculator->new();

# Test basic operations
my $sum = $calc->add(10, 5);
say "10 + 5 = $sum";

my $diff = $calc->subtract(10, 3);
say "10 - 3 = $diff";

# Test formatting (intentionally messy)
sub messy_function{my($a,$b,$c)=@_;return$a+$b+$c;}

# Test regex with different delimiters
my $text = "Hello, World!";
$text =~ m!World!;
$text =~ s{Hello}{Hi};

# Show history
say "\nCalculation history:";
say "  $_" for $calc->get_history();

# Unicode test
my $café = "coffee shop";
my $π = 3.14159;
say "Unicode variables work: $café has π = $π";

1;