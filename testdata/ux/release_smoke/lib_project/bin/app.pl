use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/../lib";
use Smoke::Greeter;

my $greeter = Smoke::Greeter->new(prefix => 'hi');
print $greeter->greet('release tester'), "\n";
