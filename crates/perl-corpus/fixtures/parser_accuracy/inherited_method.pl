package Accuracy::Parent;

sub inherited {
    return 1;
}

package Accuracy::Child;

our @ISA = qw(Accuracy::Parent);

sub call_parent {
    return Accuracy::Child->inherited();
}

1;
