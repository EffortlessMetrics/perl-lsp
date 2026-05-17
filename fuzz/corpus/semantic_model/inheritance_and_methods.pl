package Fuzz::Parent;
sub new { bless {}, shift }
sub parent_method { return 1 }
package Fuzz::Child;
our @ISA = qw(Fuzz::Parent);
sub child_method { my $self = shift; return $self->parent_method; }
