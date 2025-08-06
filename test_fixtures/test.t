#!/usr/bin/perl
use strict;
use warnings;
use Test::More tests => 3;
use lib 'lib';

BEGIN {
    use_ok('Module');
}

# Test object creation
my $module = Module->new();
isa_ok($module, 'Module', 'Object created successfully');

# Test processing
my $result = $module->process("test");
is($result, "Processed: test", 'Processing works correctly');