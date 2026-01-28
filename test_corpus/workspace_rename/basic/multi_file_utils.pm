package Utils;
# Test: Multi-file subroutine rename (library file)
# Input: Rename process to enhanced_process

use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(process);

sub process {
    my ($data) = @_;
    return "processed: $data";
}

1;
