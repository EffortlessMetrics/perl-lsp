use strict;
use warnings;

# Basic tie operations for all variable types
tie my %hash, "DB_File", "file.db", 0, 0666;
tie my @array, "Tie::Array";
tie my $scalar, "Tie::Scalar";
tie *FH, "Tie::Handle";

# tie with object capture
my $obj = tie my %cache, "Tie::StdHash";
$cache{a} = 1;

# tie with our/local declarations
tie our %config, "Config::Tie";
tie local $/, "Tie::Scalar", \$custom_rs;

# tied function - check if variable is tied
my $tied_obj = tied %hash;
if (tied @array) {
    print "array is tied\n";
}
my $is_tied = defined(tied $scalar);

# untie all variable types
untie %hash;
untie @array;
untie $scalar;
untie *FH;

# TIEHASH implementation
package MyTie;
use parent "Tie::Hash";

sub TIEHASH {
    my ($class, $filename) = @_;
    my $self = {};
    bless $self, $class;
    return $self;
}

sub FETCH {
    my ($self, $key) = @_;
    return $self->{$key};
}

sub STORE {
    my ($self, $key, $value) = @_;
    $self->{$key} = $value;
}

sub DELETE {
    my ($self, $key) = @_;
    delete $self->{$key};
}

sub EXISTS {
    my ($self, $key) = @_;
    return exists $self->{$key};
}

sub FIRSTKEY {
    my ($self) = @_;
    my $reset = keys %{$self};
    return each %{$self};
}

sub NEXTKEY {
    my ($self, $lastkey) = @_;
    return each %{$self};
}

sub SCALAR {
    my ($self) = @_;
    return scalar %{$self};
}

sub DESTROY {
    my ($self) = @_;
}

# TIESCALAR implementation
package MyTieScalar;
use parent "Tie::Scalar";

sub TIESCALAR {
    my ($class, $initial) = @_;
    my $val = $initial;
    return bless \$val, $class;
}

sub FETCH {
    my ($self) = @_;
    return ${$self};
}

sub STORE {
    my ($self, $value) = @_;
    ${$self} = $value;
}

# TIEARRAY implementation
package MyTieArray;
use parent "Tie::Array";

sub TIEARRAY {
    my ($class, @initial) = @_;
    return bless [@initial], $class;
}

sub FETCHSIZE {
    my ($self) = @_;
    return scalar @{$self};
}

sub FETCH {
    my ($self, $index) = @_;
    return $self->[$index];
}

sub STORE {
    my ($self, $index, $value) = @_;
    $self->[$index] = $value;
}

package main;
tie my %myhash, "MyTie";
tie my $myscalar, "MyTieScalar", 42;
tie my @myarray, "MyTieArray", 1, 2, 3;
