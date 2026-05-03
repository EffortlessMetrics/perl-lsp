package Accuracy::Signatures;

use feature 'signatures';
no warnings 'experimental::signatures';

sub add ($left, $right) {
    return $left + $right;
}

1;
