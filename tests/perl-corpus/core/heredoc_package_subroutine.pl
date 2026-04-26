package Demo::Heredoc;
sub message {
    my $text = <<'TXT';
hello
TXT
    return $text;
}
1;
