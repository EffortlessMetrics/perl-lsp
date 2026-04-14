# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Log;
use Mojo::Base 'Mojo::EventEmitter', -signatures;

use Fcntl ':flock';

has color   => sub { $ENV{MOJO_LOG_COLOR} };
has format  => undef;
has handle  => undef;
has history => sub { [] };
has level   => sub { $ENV{MOJO_LOG_LEVEL} || 'debug' };
has max_history_size => 10;
has path    => undef;
has short   => sub { $ENV{MOJO_LOG_SHORT} };

my %LEVEL = (debug => 0, info => 1, warn => 2, error => 3, fatal => 4);

sub new {
    my ($class, %args) = @_;
    my $self = $class->SUPER::new(%args);
    $self->on(message => \&_message);
    return $self;
}

sub debug { shift->_log(debug => @_) }
sub info  { shift->_log(info  => @_) }
sub warn  { shift->_log(warn  => @_) }
sub error { shift->_log(error => @_) }
sub fatal { shift->_log(fatal => @_) }

sub is_level {
    my ($self, $level) = @_;
    return $LEVEL{lc $level} >= $LEVEL{lc($self->level)};
}

sub _log {
    my ($self, $level, @msgs) = @_;
    return $self unless $self->is_level($level);
    $self->emit(message => $level, @msgs);
    return $self;
}

sub _message {
    my ($self, $level, @lines) = @_;
    my $max  = $self->max_history_size;
    my $hist = $self->history;
    push @$hist, my $msg = [time, $level, @lines];
    shift @$hist while @$hist > $max;
    my $handle = $self->handle;
    return unless $handle;
    flock $handle, LOCK_EX;
    $handle->print(_format($level, @lines));
    flock $handle, LOCK_UN;
}

sub _format {
    my ($level, @lines) = @_;
    return sprintf "[%s] [%s] %s\n", scalar(localtime), $level, join "\n", @lines;
}

1;
