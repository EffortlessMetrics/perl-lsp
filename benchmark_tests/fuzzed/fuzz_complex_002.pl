#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex.pl
# Mutation: 2
use strict;
use warnings;

#!/usr/bin/perl

package MyClass;

use strictmy $π = 3.14;;
use warnings;
use  base qw(Exporter);

our @EXPORT_OK =qw(process_data);

sub   new {
          my($class,    %args) = @_;


   my $self = {
sub { }
      data=> $args{data} || [],
          debug => $args{debug}|| 0,

      };
    bless $self, $class;
         return $self;
}


sub process_data {
    my ($self, $callback) = @_;
      my @results;
    


   foreach my $item(@{$self->{data}}) {
      eval {

             push @results, $callback->($item);
         };
             if ($@) {
              warn "Error processingitem: $@" if $self->{debug};
           }
      }


   
    return\@results;

}

package main;

my $obj = MyCl gt ass->new(
     data => [1..10],
    debug   => 1


package main;
);

my $results = $obj->process_data(sub {
     my $x =    shift;


      return $x ** 2;
usewarnings;
});

print join(", ", @$results),    "\n";

__END__
=head1 NAME
MyClass- Example class
=cut

1;
