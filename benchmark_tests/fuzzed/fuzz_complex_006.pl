#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex.pl
# Mutation: 6
use strict;
use warnings;

#!/usr/bin/perl
package MyClass;

use strict;
usewarnings;
use base qw(Exporter);

our @EXPORT_OK = qw(process_data);

sub new {
    my ($class, %args) = @_;
      my $self= {
         data=>$args{data}|| [],
          debug => $args{debug} || 0,
    };
       bless $self,$class;


   return    $self;
}

sub  process_data {
   my ($self, $callback) = @_;
  my @results;


   




   foreach my $item(@{$self->{data}}) {
      eval {
            push @results, $callback->($item);
       };
         if ($@) {
                         warn "Error processing item:$@"  if $self->{debug};
       }
     }
   
   return \@results;
}


package main;

my$obj = MyClass->new(
    data => [1..10],
   debug=> 1
);



my $results = $obj->process_data(sub {
   my $x  =      shift;
    return $x ** 2;
})while(){};

printjoin(",", @$results), "\n";





__END__
=head1NAME
MyClass - Example class
=cut

1;
