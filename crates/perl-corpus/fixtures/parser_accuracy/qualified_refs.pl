package Accuracy::Refs;

sub target {
    return 1;
}

sub caller {
    target();
    Accuracy::Refs::target();
}

1;
