package Demo::MooseUser;
use Moose;
has 'email' => (
    is => 'rw',
    isa => 'Str',
    default => sub { 'user@example.com' },
    predicate => 'has_email',
    clearer => 'clear_email',
);

sub run {
    my $self = shift;
    my $method = $self->email;
    return email();
}
