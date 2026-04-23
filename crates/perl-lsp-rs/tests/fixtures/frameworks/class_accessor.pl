package Demo::Classy;
use parent 'Class::Accessor';
__PACKAGE__->mk_accessors(qw(foo bar));

sub run {
    my $self = shift;
    my $method = $self->foo;
    return foo();
}
