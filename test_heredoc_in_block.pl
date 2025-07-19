#!/usr/bin/env perl

# Test heredoc inside if block
if (1) {
    my $text = <<~'EOF';
        This is inside a block
        And should work correctly
        EOF
    print "Got: $text\n";
}

# Test heredoc in nested blocks
for my $i (1..2) {
    if ($i == 1) {
        my $msg = <<EOF;
Iteration $i
EOF
        print $msg;
    }
}

# Test heredoc in hash inside block
{
    my %config = (
        template => <<'TMPL',
<html>
<body>Hello</body>
</html>
TMPL
        name => 'test'
    );
    print "Template: $config{template}\n";
}