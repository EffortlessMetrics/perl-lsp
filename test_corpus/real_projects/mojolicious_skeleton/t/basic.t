#!/usr/bin/perl
# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
use strict;
use warnings;
use Test::More;
use Test::Mojo;

my $t = Test::Mojo->new('Mojolicious');
$t->get_ok('/')->status_is(200);

done_testing();
