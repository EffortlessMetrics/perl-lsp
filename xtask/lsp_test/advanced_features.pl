#!/usr/bin/env perl
use strict;
use warnings;

# Inlay hints test
sub greet {
    my ($name, $greeting, $punctuation) = @_;
    return "$greeting, $name$punctuation";
}

# Call with positional arguments (should show parameter hints)
my $message = greet("World", "Hello", "!");

# Type hints test
my $scalar = "test";       # Should show: string
my @array = (1, 2, 3);     # Should show: array
my %hash = (a => 1);       # Should show: hash
my $ref = \@array;         # Should show: arrayref

# Complex call hierarchy
sub main {
    setup();
    process();
    cleanup();
}

sub setup {
    initialize_config();
    load_data();
}

sub process {
    validate_data();
    transform_data();
    save_results();
}

sub cleanup {
    close_handles();
    free_resources();
}

# Stub implementations
sub initialize_config { }
sub load_data { }
sub validate_data { }
sub transform_data { }
sub save_results { }
sub close_handles { }
sub free_resources { }

main();
