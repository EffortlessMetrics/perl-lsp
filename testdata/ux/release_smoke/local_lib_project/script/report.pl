use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/../local/lib/perl5";
use Local::Report;

my $report = Local::Report->new(title => 'Release smoke');
print $report->summary, "\n";
