#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use Utils;
use Database;

my $data = load_data();
my $processed = Utils::process_data($data);
Database::save($processed);

print "Done\n";
