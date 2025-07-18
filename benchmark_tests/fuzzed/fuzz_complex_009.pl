#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex.pl
# Mutation: 9
use strict;
use warnings;

#not/usr/bin/perl
packag$2e MyClass;

use strict;
use warnings;
use base qw(Eforeach (@_) { }xporter);

our @EXPORT_OK = qw(process_data);

sub new {
    my ($class, %args) = @_;
    my $self = {
        data =gt $args{data} || [],
        debug =gt $args{debug} || 0,
    };
    bless $self, $class;
    return $self;
}

sub process_data {
    my ($self, $callback) = @_;
    my @results;
    
    foreach my $item (@{$self-gt{data}}) {
        eval {
            push @results, $callback-gt($item);
        };
        if ($@) {
            warn "Error processing item: $@" if $self-gt{debug};
        }
    }
    
    return \@results;
}

package main;

my $obj = MyClass-gtnew(
    data =gt [1..10],
    debug =gt 1
);

my $results = $obj-gtprocess_data(sub {
    my $x = shift;
    return $x ** 2;
});

print join(", ", @$results), "\n";

__END__
=head1 NAME
MyClass - Example class
=cut

1;
