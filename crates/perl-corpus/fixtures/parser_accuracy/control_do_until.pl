package Accuracy::Control;

sub loop_until {
    my $count = 0;
    do {
        $count++;
    } until $count > 2;
    return $count;
}

1;
