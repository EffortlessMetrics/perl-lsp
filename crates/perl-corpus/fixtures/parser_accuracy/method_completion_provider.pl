# provider-index-support:start
package Accuracy::Provider::Parent;

sub inherited_method { 1 }

package Accuracy::Provider::Foo;
use parent 'Accuracy::Provider::Parent';

sub own_method { 1 }
sub shared_name { 1 }

package Accuracy::Provider::Bar;

sub unrelated_method { 1 }

package Accuracy::Provider::Imported;

sub imported_method { 1 }
# provider-index-support:end

package Accuracy::Provider::Foo;

sub self_case {
    my $self = shift;
    $self-> # cursor:self
}

package Accuracy::Provider::UseCases;
use Accuracy::Provider::Imported;

sub constructor_case {
    my $ctor = Accuracy::Provider::Foo->new;
    $ctor-> # cursor:constructor
}

sub literal_bless_case {
    my $literal = bless {}, "Accuracy::Provider::Foo";
    $literal-> # cursor:literal_bless
}

sub qualified_literal_bless_case {
    my $qualified = bless({}, "Accuracy::Provider::Foo");
    $qualified-> # cursor:qualified_bless
}

sub dynamic_bless_case {
    my $class = "Accuracy::Provider::Foo";
    my $dynamic = bless {}, $class;
    $dynamic-> # cursor:dynamic_bless
}

sub concat_bless_case {
    my $concat = bless {}, "Accuracy::Provider::" . "Foo";
    $concat-> # cursor:concat_bless
}

sub nested_bless_case {
    my $nested = wrapper(bless {}, "Accuracy::Provider::Foo");
    $nested-> # cursor:nested_bless
}

sub unknown_case {
    my $unknown = get_object();
    $unknown-> # cursor:unknown
}

sub imported_unknown_case {
    my $imported_unknown = get_object();
    $imported_unknown-> # cursor:imported_unknown
}

1;
