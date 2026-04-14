# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
# Original copyright: Alexis Sukrieh, Sawyer X and contributors
package Dancer2::Core::DSL;
use Moo;
use Carp 'croak';

has app => (is => 'ro', required => 1, weak_ref => 1);

my @KEYWORDS = qw(
    get post put del options patch any
    route hook before after
    params body header headers status
    request response
    send_file send_error halt redirect
    session cookie
    template
    set setting config
    dance start
    log debug info warning error
    from_json to_json from_yaml to_yaml
    encode_json decode_json
    var vars
    captures splat
);

sub dsl_keywords { @KEYWORDS }

sub get {
    my ($self, $pattern, @args) = @_;
    my $code = pop @args;
    $self->app->add_route(method => 'GET', regexp => _compile($pattern), code => $code);
}

sub post {
    my ($self, $pattern, @args) = @_;
    my $code = pop @args;
    $self->app->add_route(method => 'POST', regexp => _compile($pattern), code => $code);
}

sub put {
    my ($self, $pattern, @args) = @_;
    my $code = pop @args;
    $self->app->add_route(method => 'PUT', regexp => _compile($pattern), code => $code);
}

sub del {
    my ($self, $pattern, @args) = @_;
    my $code = pop @args;
    $self->app->add_route(method => 'DELETE', regexp => _compile($pattern), code => $code);
}

sub hook {
    my ($self, $name, $code) = @_;
    $self->app->add_hook($name, $code);
}

sub set {
    my ($self, $key, $value) = @_;
    $self->app->config->set($key, $value);
}

sub redirect {
    my ($self, $url, $status) = @_;
    $status //= 302;
    # no-op in skeleton
}

sub template {
    my ($self, $name, $tokens, $opts) = @_;
    return '';
}

sub _compile {
    my $pattern = shift;
    return $pattern if ref $pattern eq 'Regexp';
    $pattern =~ s{:(\w+)}{([^/]+)}g;
    $pattern =~ s{\*}{(.+)}g;
    return qr{^$pattern$};
}

1;
