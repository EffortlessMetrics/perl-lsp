# Complex Perl constructs
use strict;
# <- keyword
#   ^ type

$var =~ s/pattern/replacement/g;
# ^ punctuation.special
# ^ variable
#    ^ operator

# Regex
$text =~ /pattern/i;
# ^ punctuation.special
# ^ variable
#      ^ operator

# Special variables
print $_;
# <- function
#       ^ punctuation.special
#        ^ variable

# Heredoc
my $doc = <<'EOF';
# <- keyword
#    ^ punctuation.special
#     ^ variable
#         ^ operator
This is a heredoc
EOF
# ^ label

# Array slicing
my @slice = @array[0..2];
# <- keyword
#    ^ punctuation.special
#     ^ variable
#              ^ punctuation.special
#               ^ variable