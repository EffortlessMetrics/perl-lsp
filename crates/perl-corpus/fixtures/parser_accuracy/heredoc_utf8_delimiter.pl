package Accuracy::HeredocUtf8Delimiter;

use utf8;

my $text = <<עד_כאן;
body
עד_כאן

sub after_utf8_heredoc { return $text; }

1;
