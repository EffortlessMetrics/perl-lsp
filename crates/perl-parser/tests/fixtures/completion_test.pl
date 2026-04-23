#!/usr/bin/perl
use strict;
use warnings;

# Test fixture for code completion

# Scalar variables for completion
my $user_name = "Alice";
my $user_age = 30;
my $user_email = "alice@example.com";

# Array for completion
my @user_list = ("Alice", "Bob", "Charlie");

# Hash for completion
my %user_data = (
    name => "Alice",
    age => 30,
    email => "alice@example.com"
);

# Subroutines for completion
sub get_user_name {
    return $user_name;
}

sub get_user_age {
    return $user_age;
}

sub process_user_data {
    my ($name, $age) = @_;
    # Type $us[TAB] here - should complete to $user_name, $user_age, $user_email
    
    # Type get_[TAB] here - should complete to get_user_name, get_user_age
    
    # Type prin[TAB] here - should complete to print, printf
}

# Package for method completion
package UserManager;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub add_user {
    my ($self, $name) = @_;
    # Implementation
}

sub remove_user {
    my ($self, $name) = @_;
    # Implementation
}

sub list_users {
    my $self = shift;
    # Implementation
}

1;