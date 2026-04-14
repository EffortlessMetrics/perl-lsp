#!/usr/bin/perl
# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
use strict;
use warnings;
use Test::More;

{
    package MyApp;
    use Moose;
    extends 'Catalyst';

    MyApp->config(name => 'MyApp');
    MyApp->setup;
}

ok(defined MyApp->config, 'config is defined');
ok(MyApp->config->{name} eq 'MyApp', 'app name set correctly');

done_testing();
