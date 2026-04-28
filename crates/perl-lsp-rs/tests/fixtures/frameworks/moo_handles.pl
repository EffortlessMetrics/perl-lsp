package Demo::ProfileUser;
use Moo;

has 'profile' => (
    is => 'rw',
    builder => 1,
    predicate => 1,
    clearer => 1,
    handles => {
        full_name => 'name',
        timezone => 'tz',
    },
);

sub run {
    my $self = shift;
    my $method = $self->full_name;
    return full_name();
}
