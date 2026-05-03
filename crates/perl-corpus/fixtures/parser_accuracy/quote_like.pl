package Accuracy::QuoteLike;

sub quote {
    my $message = q{hello};
    return qq{$message};
}

1;
