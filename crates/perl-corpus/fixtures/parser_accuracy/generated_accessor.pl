package Accuracy::GeneratedAccessor;

use Moo;

has name => (is => 'ro');

sub existing {
    return 1;
}

sub call_name_accessor {
    my $obj = Accuracy::GeneratedAccessor->new();
    return $obj->name;
}

1;
