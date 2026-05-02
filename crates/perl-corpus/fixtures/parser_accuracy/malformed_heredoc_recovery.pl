package Accuracy::MalformedHeredocRecovery;

my $text = <<'BROKEN';
unterminated body

sub after_recovery {
    return 1;
}

1;
