#!/usr/bin/perl
use strict;
use warnings;

# Test file for completion functionality
# Contains variables and functions for completion testing

package CompletionTest;

# User-related variables for completion testing
my $user_name = "John Doe";
my $user_age = 30;
my $user_email = "john@example.com";

# Array and hash variables
my @users = ();
my %user_data = ();

sub get_user_info {
    my ($id) = @_;
    return {
        name => $user_name,
        age => $user_age,
    };
}

sub update_user {
    my ($name, $age) = @_;
    $user_name = $name;
    # Complete $us here at line 31, character 14
    print $us
}

# Built-in function usage for completion
my $uppercase = uc($user_name);
my $length = length($user_name);

# More variables for completion variety
my $username_short = substr($user_name, 0, 5);
my $user_active = 1;

1;
