package GoneModule;

use strict;
use warnings;

# This module exists on disk but must NOT be resolved because
# 'no lib' cancelled the earlier 'use lib' before the 'use GoneModule' line.

sub gone { return "I should not be found" }

1;
