use strict;
use warnings;

my $name = shift @ARGV // 'world';
my $message = greet($name);

print "$message\n";

sub greet {
    my ($target) = @_;
    return "hello $target";
}
