package Accuracy::MethodCall;

sub invoke {
    my $object = shift;
    $object->run;
    Accuracy::MethodCall->run();
}

sub run {
    return 1;
}

1;
