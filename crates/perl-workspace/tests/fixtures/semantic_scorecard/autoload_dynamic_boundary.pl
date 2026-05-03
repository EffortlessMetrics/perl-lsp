package AutoloadDynamicBoundary;

our $AUTOLOAD;

sub AUTOLOAD {
    return $AUTOLOAD;
}

missing_method();
