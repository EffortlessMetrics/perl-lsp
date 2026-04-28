package Demo::WithModifiers;
use Moo;

sub save {
    my ($self) = @_;
    return 1;
}

before 'save' => sub {
    my ($self) = @_;
    # validation before save
};

after 'save' => sub {
    my ($self) = @_;
    # cleanup after save
};

around 'save' => sub {
    my ($orig, $self, @args) = @_;
    return $self->$orig(@args);
};
