# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Log;
use Moose;
use Carp 'croak';

has _body    => (is => 'rw', default => sub { [] });
has level    => (is => 'rw', default => 'error');
has _levels  => (is => 'ro', default => sub {
    { debug => 0, info => 1, warn => 2, error => 3, fatal => 4 }
});
has _level_num => (is => 'lazy');
has abort    => (is => 'rw', default => 0);
has autoflush => (is => 'rw', default => 1);

my @LEVELS = qw(debug info warn error fatal);

sub _build__level_num {
    my $self = shift;
    return $self->_levels->{ lc $self->level } // 1;
}

for my $level (@LEVELS) {
    my $num = { debug => 0, info => 1, warn => 2, error => 3, fatal => 4 }->{$level};
    no strict 'refs';
    *{"is_$level"} = sub {
        my $self = shift;
        return ($self->_levels->{ lc $self->level } // 1) <= $num;
    };
    *{$level} = sub {
        my ($self, @msgs) = @_;
        return unless $self->${\"is_$level"};
        my $msg = join "\n", @msgs;
        push @{$self->_body}, sprintf("[%s] %s", uc $level, $msg);
        $self->_flush if $self->autoflush;
    };
}

sub _dump { Carp::confess("Not implemented") }

sub _flush {
    my $self = shift;
    return unless @{$self->_body};
    print STDERR join("\n", @{$self->_body}), "\n" unless $self->abort;
    $self->_body([]);
}

sub body { join "\n", @{$_[0]->_body} }

__PACKAGE__->meta->make_immutable;
no Moose;

1;
