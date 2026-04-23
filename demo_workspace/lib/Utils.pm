package Utils;
use strict;
use warnings;
use List::Util qw(max min sum);
use Data::Dumper;

sub process_data {
    my ($data) = @_;
    
    # Some processing logic
    my $max_val = max(@$data);
    my $min_val = min(@$data);
    my $sum_val = sum(@$data);
    
    return {
        max => $max_val,
        min => $min_val,
        sum => $sum_val,
    };
}

sub load_data {
    return [1, 2, 3, 4, 5];
}

sub unused_helper {
    # This function is never called
    return 42;
}

1;
