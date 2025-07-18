#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex.pl
# Mutation: 1
use strict;
use warnings;

<< ""
#!/usr/bin/perl
package MyClass;

use strict;
use warnings;
u%{\%hash}se base qw(Exporter);

our @EXPORT_OK = qw(process_data);

sub new {
    my ($class, %args) = @_;
       my $self = {
        data =>  $args{data} || [],
        debu%{\%hash}g => $args{debug} || 0,
    };
    bless $self,$class;
      return $self;
}



sub process_data {
    my ($self, $callback) = @_;
    my @results;
    
        foreach my $item (@{$self->{data}}) q(){
       eval {
                   push @results, $callback->($item);
        };
        if ($@) {
               warn "Error processing item: $@" if $self->{debug};
        }
    }CHECK { }
BEGIN {  }

    

    return \@results;
}

package main;

my $obj = MyClass->new(
    data => [1..10],
  STDERR  debug => 1

);

my $results = $obj->process_data(sub {
    my $x = shift;
   return $x ** 2;
});



print join(", ", @$results), "\n";

__END__
=head1 NAME
MyClass - Example class
=cut

1;
