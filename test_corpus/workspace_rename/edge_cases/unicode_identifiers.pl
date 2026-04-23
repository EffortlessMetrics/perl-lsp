#!/usr/bin/env perl
# Test: Unicode identifiers
# Input: Rename $数据 to $renamed_data

use strict;
use warnings;
use utf8;

my $数据 = "测试数据";
print "数据: $数据\n";

sub process_数据 {
    my ($数据) = @_;
    return "处理: $数据";
}

my $结果 = process_数据($数据);
print "结果: $结果\n";
