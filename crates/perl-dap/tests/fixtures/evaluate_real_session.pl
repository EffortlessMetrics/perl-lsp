use strict;
use warnings;
use utf8;

our $GLOBAL_SCALAR = 'global-eval';
our @GLOBAL_ARRAY = (0 .. 549);
my $cyrillic_key = "\x{043A}\x{043B}\x{044E}\x{0447}";
my $cyrillic_value = "\x{0437}\x{043D}\x{0430}\x{0447}\x{0435}\x{043D}\x{0438}\x{0435}";
my $emoji_key = "emoji_\x{1F600}";
our %GLOBAL_UNICODE = (
    $cyrillic_key => $cyrillic_value,
    $emoji_key => 'smile',
);
our $GLOBAL_OBJECT = bless {
    kind => 'fixture-object',
}, 'Fixture::EvalThing';
our $GLOBAL_CODEREF = sub { return 42; };

my $anchor = 1; # BREAKPOINT_LINE
print "$anchor\n";
