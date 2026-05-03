package Accuracy::MethodCall;

sub invoke {
    my $object = shift;
    $object->run("arg");
}

1;
