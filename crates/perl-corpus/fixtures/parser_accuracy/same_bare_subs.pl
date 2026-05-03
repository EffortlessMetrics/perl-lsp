package Accuracy::Same::A;

sub run {
    return "A";
}

package Accuracy::Same::B;

sub run {
    return "B";
}

package Accuracy::Same::Main;

sub call_both {
    Accuracy::Same::A::run();
    Accuracy::Same::B::run();
}

1;
