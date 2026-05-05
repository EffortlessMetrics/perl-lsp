# provider-index-support:start
package Accuracy::Navigation::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(imported_nav);
sub imported_nav { 1 }

package Accuracy::Navigation::Parent;
sub inherited_method { 1 }

package Accuracy::Navigation::Generated;
sub generated_accessor { 1 }
# provider-index-support:end

package Accuracy::Navigation::UseCases;
use strict;
use warnings;
use parent 'Accuracy::Navigation::Parent';
use Accuracy::Navigation::Exporter qw(imported_nav);

sub own_sub { 1 }
sub own_method { 1 }

sub bare_call_case {
    own_sub(); # cursor:bare_call
}

sub qualified_call_case {
    Accuracy::Navigation::UseCases::own_sub(); # cursor:qualified_call
}

sub imported_symbol_case {
    imported_nav(); # cursor:imported_nav
}

sub method_receiver_case {
    my $self = shift;
    $self->inherited_method(); # cursor:inherited_method
}

sub generated_accessor_case {
    Accuracy::Navigation::Generated::generated_accessor(); # cursor:generated_accessor
}

sub dynamic_boundary_case {
    my $name = "own_sub";
    no strict 'refs';
    &$name(); # cursor:dynamic_ref
}

1;
