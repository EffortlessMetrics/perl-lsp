package Module;
use strict;
use warnings;

sub new {
    my $class = shift;
    my $self = {
        data => [],
        @_
    };
    return bless $self, $class;
}

sub process {
    my ($self, $input) = @_;
    
    # Process the input
    if (defined $input && length($input) > 0) {
        push @{$self->{data}}, $input;
        return "Processed: $input";
    }
    
    return undef;
}

sub get_data {
    my $self = shift;
    return @{$self->{data}};
}

1;