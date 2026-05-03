package Accuracy::Postderef;

sub values {
    my $array = [1, 2, 3];
    return $array->@*;
}

1;
