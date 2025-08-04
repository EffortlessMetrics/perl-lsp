#!/usr/bin/env perl
# Test script for advanced LSP features

use strict;
use warnings;
use Test::More;

# This function demonstrates call hierarchy
sub calculate_total {
    my ($items) = @_;
    
    my $sum = 0;
    foreach my $item (@$items) {
        $sum += process_item($item);
    }
    
    return apply_tax($sum);
}

# Called by calculate_total
sub process_item {
    my ($item) = @_;
    return $item->{price} * $item->{quantity};
}

# Also called by calculate_total
sub apply_tax {
    my ($amount) = @_;
    return $amount * 1.08;  # 8% tax
}

# Test function for test runner
sub test_calculation {
    my $items = [
        { price => 10, quantity => 2 },
        { price => 5,  quantity => 3 },
    ];
    
    my $total = calculate_total($items);
    ok($total > 0, "Total should be positive");
    is(sprintf("%.2f", $total), "37.80", "Total with tax should be correct");
}

# Another test
sub test_empty_cart {
    my $total = calculate_total([]);
    is($total, 0, "Empty cart should have zero total");
}

# Run tests if this is the main script
if (!caller) {
    test_calculation();
    test_empty_cart();
    done_testing();
}

1;