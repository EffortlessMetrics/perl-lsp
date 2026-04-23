package Demo::User;
use Moo;
has 'name' => (is => 'ro', isa => 'Str', default => sub { 'anon' });

sub greet {
    my $self = shift;
    my $method = $self->name;
    return name();
}
