package Accuracy::Autoload;

our $AUTOLOAD;

sub AUTOLOAD {
    return $AUTOLOAD;
}

sub call_missing {
    Accuracy::Autoload->missing();
}

1;
