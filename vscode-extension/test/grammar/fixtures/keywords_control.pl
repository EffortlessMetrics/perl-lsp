use strict;
use warnings;
package My::Module;

sub greet {
    my ($self, $name) = @_;
    if ($name) {
        return "hi $name";
    } elsif (defined $self) {
        return 'hi';
    } else {
        return undef;
    }
}

foreach my $i (1 .. 10) {
    next if $i % 2;
    last if $i > 8;
}

while (my $line = <STDIN>) {
    chomp $line;
}
