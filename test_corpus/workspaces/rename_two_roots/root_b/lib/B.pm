package B;

use A;

sub run {
    my $obj = A->new();
    return A::target_name($obj);
}

1;
