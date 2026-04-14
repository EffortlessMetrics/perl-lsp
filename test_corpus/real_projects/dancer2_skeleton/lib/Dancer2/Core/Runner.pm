# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
# Original copyright: Alexis Sukrieh, Sawyer X and contributors
package Dancer2::Core::Runner;
use Moo;
use Carp 'croak';

has apps    => (is => 'rw', default => sub { [] });
has server  => (is => 'rw');
has port    => (is => 'rw', default => sub { $ENV{DANCER_PORT} // 5000 });
has host    => (is => 'rw', default => sub { $ENV{DANCER_SERVER} // '0.0.0.0' });
has startup_info => (is => 'rw', default => sub { 1 });

sub start {
    my $self = shift;
    my $app = $self->psgi_app;
    require Plack::Runner;
    my $runner = Plack::Runner->new;
    $runner->parse_options(
        '--host', $self->host,
        '--port', $self->port,
    );
    $runner->run($app);
}

sub psgi_app {
    my $self = shift;
    my @apps = @{$self->apps};
    return sub {
        my $env = shift;
        for my $app (@apps) {
            if (my $res = $app->dispatch($env)) {
                return $res;
            }
        }
        return [404, ['Content-Type' => 'text/plain'], ['Not Found']];
    };
}

1;
