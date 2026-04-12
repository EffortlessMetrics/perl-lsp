use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/lib";
use FindBinModule;

my $val = FindBinModule::value();
print "$val\n";
