use strict;
use warnings;
use File::Spec;
use JSON::PP;

my $readme = "README.md";
my $path = File::Spec->catfile('docs', $readme);
my $json = JSON::PP->new->encode({ path => $path });

print "$json\n";
