use strict;
use warnings;
use utf8;

package Fixture::Widget;

sub new {
    my ($class, $name) = @_;
    return bless {
        name => $name,
        kind => "fixture",
    }, $class;
}

package main;

our $GLOBAL_NAME = 'global-visible';
our %GLOBAL_LOOKUP = (
    alpha => 1,
    beta  => 2,
);
our $shared_symbol = 'package-shared';

my $shared_symbol = 'lexical-shadow';
my @large_200 = (0 .. 220);
my @large_500 = (0 .. 550);
my %unicode_hash = (
    "ключ" => "значение",
    "こんにちは" => "世界",
    "emoji_😀" => "値",
);
my %deep_hash = (
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    level5 => {
                        leaf => 'deep-value',
                        data => [0 .. 260],
                    },
                },
            },
        },
    },
);
my $coderef = sub {
    my ($x) = @_;
    return $x * 2;
};
my $object = Fixture::Widget->new('widget-1');
my $breakpoint_anchor = scalar(@large_200) + scalar(@large_500);

$GLOBAL_NAME = "$GLOBAL_NAME:$breakpoint_anchor";
print "$GLOBAL_NAME\n";
