package Clean::Exports;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(clean_name);

=head1 NAME

Clean::Exports - export native critic fixture

=head1 DESCRIPTION

Keeps simple Exporter package setup quiet under native critic.

=cut

sub clean_name {
    my ($name) = @_;
    return ucfirst lc $name;
}

1;
