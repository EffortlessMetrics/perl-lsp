use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/lib";
use Smoke::CRLF;

my $note_file = "notes/todo.txt";
my $label = Smoke::CRLF::label();

print "$label from $note_file\n";
