use strict;
use warnings;

my @files = glob "*.pl";
my @modules = <*.pm>;
my @all = glob "**/*.pm";
my @hidden = glob ".*";
my @chars = glob "[a-z]*.pl";
my @brace = glob "file{1,2,3}.txt";
my @nested = glob "dir1/*/dir2/*.pm";
my @mix = glob "/tmp/{a,b,c}*.txt";

my $single = glob "*.log";

while (my $file = glob "*.txt") {
    print "Found $file\n";
}
