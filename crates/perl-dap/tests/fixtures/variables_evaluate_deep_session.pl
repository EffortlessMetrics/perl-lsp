use strict;
use warnings;
use utf8;

our $GLOBAL_SCALAR = 'global-visible';
my $snowman_key = "\x{2603}";
our %GLOBAL_HASH = (
    greeting => 'hello',
    $snowman_key => 'snowman',
);

package Fixture::Thing;

sub new {
    my ($class, $name) = @_;
    return bless {
        class_name => $name,
        status => 'ready',
    }, $class;
}

package main;

my @big_array = (0 .. 549);
my %deep_hash = (
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    level5 => {
                        level6 => {
                            terminal => 'done',
                            unicode => '雪',
                        },
                    },
                },
            },
        },
    },
);
my $lexical_scalar = 'lexical-visible';
my $coderef = sub { return $lexical_scalar; };
my $object = Fixture::Thing->new('demo-object');
my $cyrillic_key = "\x{043A}\x{043B}\x{044E}\x{0447}";
my $cyrillic_value = "\x{0437}\x{043D}\x{0430}\x{0447}\x{0435}\x{043D}\x{0438}\x{0435}";
my $emoji_key = "emoji_\x{1F600}";
my %unicode_map = (
    $cyrillic_key => $cyrillic_value,
    $emoji_key => 'smile',
);
my $truncated_probe = {
    array => \@big_array,
    nested => \%deep_hash,
    object => $object,
    cb => $coderef,
};

my $ready = $lexical_scalar; # BREAKPOINT_LINE
print "$ready\n";
print scalar(keys %unicode_map), "\n";
print $GLOBAL_SCALAR, "\n";
