package Accuracy::Regex;

sub matches {
    my $value = shift;
    return $value =~ /foo\d+/;
}

1;
