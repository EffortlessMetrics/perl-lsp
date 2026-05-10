package Clean::DispatchTable;
use strict;
use warnings;

=head1 NAME

Clean::DispatchTable - generated-dispatch native critic fixture

=head1 DESCRIPTION

Keeps a common hash-of-coderefs dispatch table quiet under native critic.

=cut

my %HANDLERS = (
    add => sub {
        my ($left, $right) = @_;
        return $left + $right;
    },
    join_names => sub {
        my (@names) = @_;
        return join q{:}, @names;
    },
);

sub dispatch {
    my ($name, @args) = @_;
    my $handler = $HANDLERS{$name};
    if (!defined $handler) {
        return undef;
    }
    return $handler->(@args);
}

1;
