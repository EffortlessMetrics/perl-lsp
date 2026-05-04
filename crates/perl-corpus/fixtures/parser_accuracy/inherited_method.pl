package Accuracy::Parent;

sub inherited {
    return 1;
}

package Accuracy::Child;

our @ISA = qw(Accuracy::Parent);

sub call_parent {
    return Accuracy::Parent::inherited();
}

1;
