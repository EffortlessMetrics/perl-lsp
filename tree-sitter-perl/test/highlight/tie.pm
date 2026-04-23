### tie operations
tie %hash, 'Tie::StdHash';
# <- function
#   ^ variable.hash
#          ^ string
tie @array, 'Tie::Array', @options;
# <- function
#   ^ variable.array
#           ^ string
tie $scalar, 'Tie::Scalar';
# <- function
#   ^ variable.scalar
#            ^ string
tied %hash;
# <- function
#    ^ variable.hash
untie %hash;
# <- function
#     ^ variable.hash

### tie methods
sub TIEHASH {
# <- keyword
#   ^ function
    my ($class, $filename) = @_;
    #  ^ variable.scalar
    bless {}, $class;
}

sub FETCH {
# <- keyword
#   ^ function
    my ($self, $key) = @_;
    #  ^ variable.scalar
    return $self->{$key};
    # <- keyword
}

sub STORE {
# <- keyword
#   ^ function
    my ($self, $key, $value) = @_;
    $self->{$key} = $value;
}

sub DELETE {
# <- keyword
#   ^ function
    my ($self, $key) = @_;
    delete $self->{$key};
}

sub EXISTS {
# <- keyword
#   ^ function
    my ($self, $key) = @_;
    exists $self->{$key};
}
