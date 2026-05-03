package Accuracy::Format;

format STDOUT =
@<<<<
$main::value
.

sub after_format {
    return 1;
}

1;
