package Accuracy::Role;

sub provided {
    return 1;
}

package Accuracy::RoleConsumer;

sub local_method {
    return provided();
}

1;
