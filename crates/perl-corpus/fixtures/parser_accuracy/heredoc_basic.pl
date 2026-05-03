package Accuracy::Heredoc;

my $text = <<'END_TEXT';
hello
END_TEXT

sub after {
    return $text;
}

1;
