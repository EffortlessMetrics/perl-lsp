package Local::Report;

use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless { title => $args{title} // 'untitled' }, $class;
}

sub summary {
    my ($self) = @_;
    return "report: $self->{title}";
}

1;
