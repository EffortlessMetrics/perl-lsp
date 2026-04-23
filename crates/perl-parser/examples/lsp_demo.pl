#!/usr/bin/env perl

# This file demonstrates various Perl constructs to test LSP features

# Variable declarations
my $scalar = "Hello, world!";
my @array = (1, 2, 3, 4, 5);
my %hash = (
    name => "John",
    age => 30,
    city => "New York"
);

# Undeclared variable (should trigger diagnostic)
$undeclared = "This should show an error";

# Function definition
sub greet {
    my ($name) = @_;
    print "Hello, $name!\n";
    return 1;
}

# Function call
greet("Alice");

# Control structures
if ($scalar =~ /world/) {
    print "Found world!\n";
}

for my $i (0..10) {
    print "$i ";
}
print "\n";

# Regular expressions
my $text = "The quick brown fox";
$text =~ s/quick/slow/;
$text =~ m!brown!;  # Non-standard delimiter

# Modern Perl features
use feature 'say';
say "Using modern features";

# Object-oriented
package Animal {
    sub new {
        my $class = shift;
        return bless {}, $class;
    }
    
    sub speak {
        say "Generic animal sound";
    }
}

my $animal = Animal->new();
$animal->speak();

# Error cases for testing diagnostics
my $incomplete = "This string is not closed
my $bad_regex = /[/;  # Unclosed regex
sub missing_brace {  # Missing closing brace

# Unicode support
my $π = 3.14159;
my $café = "coffee shop";

# Complex expressions
my $result = $scalar =~ /Hello/ ? "Found" : "Not found";
my @sorted = sort { $a <=> $b } @array;
my @mapped = map { $_ * 2 } grep { $_ > 2 } @array;

# File operations
open my $fh, '<', 'nonexistent.txt' or warn "Can't open file: $!";
close $fh if defined $fh;

# References and complex data structures
my $arrayref = \@array;
my $hashref = \%hash;
my $coderef = sub { return "Anonymous sub" };

# Nested data structure
my $complex = {
    users => [
        { name => "Alice", age => 25 },
        { name => "Bob", age => 30 },
    ],
    settings => {
        theme => "dark",
        language => "en",
    }
};

# Accessing nested data
say $complex->{users}->[0]->{name};
say $complex->{settings}{theme};

1;