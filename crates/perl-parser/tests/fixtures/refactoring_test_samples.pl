#!/usr/bin/perl
# Refactoring Test Samples for refactoring.rs validation
# Contains code patterns that should trigger various refactoring suggestions

package Refactoring::TestSamples;
use strict;
use warnings;

# EXTRACT VARIABLE refactoring opportunity
sub extract_variable_example {
    my ($user_data) = @_;

    # Complex expression that should be extracted to a variable
    my $result = process_data(
        $user_data->{profile}->{settings}->{preferences}->{display_format} || 'default'
    );

    return format_output(
        $user_data->{profile}->{settings}->{preferences}->{display_format} || 'default',
        $result
    );
}

# EXTRACT SUBROUTINE refactoring opportunity
sub extract_subroutine_example {
    my ($orders) = @_;

    my @processed_orders = ();

    for my $order (@$orders) {
        # This block should be extracted to a separate subroutine
        my $total = 0;
        for my $item (@{$order->{items}}) {
            $total += $item->{price} * $item->{quantity};
        }

        my $tax = $total * 0.08;
        my $shipping = $total > 50 ? 0 : 5.99;
        my $final_total = $total + $tax + $shipping;

        push @processed_orders, {
            %$order,
            subtotal => $total,
            tax => $tax,
            shipping => $shipping,
            total => $final_total,
        };
    }

    return \@processed_orders;
}

# DUPLICATE CODE refactoring opportunity
sub user_validation {
    my ($user) = @_;

    # Validation logic that's duplicated in admin_validation
    unless (defined $user->{name} && length $user->{name} > 0) {
        return { error => "Name is required" };
    }

    unless (defined $user->{email} && $user->{email} =~ /\@/) {
        return { error => "Valid email is required" };
    }

    unless (defined $user->{age} && $user->{age} >= 0) {
        return { error => "Valid age is required" };
    }

    return { valid => 1 };
}

sub admin_validation {
    my ($admin) = @_;

    # Duplicate validation logic (should be extracted to common function)
    unless (defined $admin->{name} && length $admin->{name} > 0) {
        return { error => "Name is required" };
    }

    unless (defined $admin->{email} && $admin->{email} =~ /\@/) {
        return { error => "Valid email is required" };
    }

    unless (defined $admin->{age} && $admin->{age} >= 0) {
        return { error => "Valid age is required" };
    }

    # Additional admin-specific validation
    unless ($admin->{permissions}) {
        return { error => "Admin permissions required" };
    }

    return { valid => 1 };
}

# LONG METHOD refactoring opportunity
sub overly_long_method {
    my ($complex_data, $options) = @_;

    # This method is too long and does too many things
    my $result = {};

    # Phase 1: Data validation
    unless ($complex_data && ref $complex_data eq 'HASH') {
        die "Invalid data provided";
    }

    # Phase 2: Data transformation
    my @transformed_items = ();
    for my $key (keys %$complex_data) {
        my $item = $complex_data->{$key};

        if ($item->{type} eq 'numeric') {
            push @transformed_items, {
                key => $key,
                value => $item->{value} * 2,
                processed => 1,
            };
        } elsif ($item->{type} eq 'string') {
            push @transformed_items, {
                key => $key,
                value => uc($item->{value}),
                processed => 1,
            };
        } else {
            push @transformed_items, {
                key => $key,
                value => $item->{value},
                processed => 0,
            };
        }
    }

    # Phase 3: Aggregation
    my $summary = {};
    for my $item (@transformed_items) {
        $summary->{processed_count}++ if $item->{processed};
        $summary->{total_count}++;
    }

    # Phase 4: Output formatting
    if ($options->{format} eq 'json') {
        require JSON;
        $result->{data} = JSON::encode_json(\@transformed_items);
        $result->{summary} = JSON::encode_json($summary);
    } elsif ($options->{format} eq 'yaml') {
        require YAML;
        $result->{data} = YAML::Dump(\@transformed_items);
        $result->{summary} = YAML::Dump($summary);
    } else {
        $result->{data} = \@transformed_items;
        $result->{summary} = $summary;
    }

    return $result;
}

# TOO MANY PARAMETERS refactoring opportunity
sub function_with_too_many_params {
    my ($name, $address, $phone, $email, $age, $gender, $occupation,
        $salary, $department, $manager, $start_date, $end_date,
        $benefits, $status, $notes, $preferences) = @_;

    # This function has too many parameters and should be refactored
    # to use an object or hash reference instead

    return {
        personal => {
            name => $name,
            address => $address,
            phone => $phone,
            email => $email,
            age => $age,
            gender => $gender,
        },
        employment => {
            occupation => $occupation,
            salary => $salary,
            department => $department,
            manager => $manager,
            start_date => $start_date,
            end_date => $end_date,
            benefits => $benefits,
            status => $status,
        },
        metadata => {
            notes => $notes,
            preferences => $preferences,
        }
    };
}

# DEAD CODE refactoring opportunity
sub function_with_dead_code {
    my ($active_feature) = @_;

    if ($active_feature) {
        return process_active_feature($active_feature);
    }

    # This code is never reached and should be removed
    my $unused_variable = "This is never used";
    my $another_unused = calculate_something_never_called();

    return format_unused_result($unused_variable, $another_unused);
}

# RENAME SYMBOL refactoring opportunity
sub poorly_named_function {
    my ($d) = @_;  # Poor parameter name

    my $tmp = $d * 2;  # Poor variable name
    my $x = $tmp + 5;  # Poor variable name

    return $x;
}

# MODERNIZE CODE refactoring opportunity (old Perl patterns)
sub old_perl_patterns {
    my @arr = @_;

    # Old-style array iteration
    for (my $i = 0; $i < @arr; $i++) {
        my $elem = $arr[$i];
        print "$i: $elem\n";
    }

    # Old-style file handling
    open(FILE, "< /tmp/test.txt") or die $!;
    my @lines = <FILE>;
    close(FILE);

    # Old-style string concatenation
    my $str = "";
    $str = $str . "Hello ";
    $str = $str . "World";

    return $str;
}

1;

__END__

=head1 NAME

Refactoring::TestSamples - Test samples for refactoring analysis

=head1 DESCRIPTION

This module contains various Perl code patterns that should trigger
different types of refactoring suggestions:

- Extract Variable
- Extract Subroutine
- Duplicate Code Detection
- Long Method
- Too Many Parameters
- Dead Code Detection
- Symbol Renaming
- Code Modernization

These samples are used to test the refactoring.rs module functionality.

=cut