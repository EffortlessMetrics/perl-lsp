#!/usr/bin/perl
# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
use strict;
use warnings;
use Test::More;
use Plack::Test;
use HTTP::Request::Common;

{
    package MyApp;
    use Dancer2;
    get '/' => sub { 'Hello World' };
}

my $app = MyApp->to_app;
my $test = Plack::Test->create($app);
my $res = $test->request(GET '/');
is $res->status_line, '200 OK';

done_testing();
