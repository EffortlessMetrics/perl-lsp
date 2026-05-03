package Accuracy::EvalString;

my $code = 'sub generated { 1 }';
eval $code;

sub after_eval {
    return 1;
}

1;
