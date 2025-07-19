#!/usr/bin/env perl

# Test various heredoc scenarios

# Basic heredoc
print <<EOF;
This is a basic heredoc
With multiple lines
EOF

# Variable assignment with heredoc  
my $text = <<'END';
Single quoted heredoc
No interpolation here: $var
END

# Interpolated heredoc
my $name = "World";
my $greeting = <<"GREETING";
Hello, $name!
Welcome to heredocs.
GREETING

# Multiple heredocs
print <<FIRST, <<SECOND;
First heredoc
FIRST
Second heredoc  
SECOND

# Indented heredoc (Perl 5.26+)
if (1) {
    print <<~INDENTED;
        This heredoc
        can be indented
        INDENTED
}

print "All tests complete\n";