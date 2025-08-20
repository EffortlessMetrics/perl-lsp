#!/usr/bin/env perl
# Test: __DATA__ and __END__ sections
# Impact: Parser must stop treating content as code after these markers

use strict;
use warnings;

# Normal code section
sub process_data {
    my @lines = <DATA>;
    return \@lines;
}

sub read_data_handle {
    seek(DATA, 0, 0);  # Reset to beginning
    my $content = do { local $/; <DATA> };
    return $content;
}

# Package with DATA section
package My::Module {
    sub get_template {
        return \*DATA;
    }
}

# Special filehandles
print "Before DATA section\n";

# Parser should recognize this as end of code
__DATA__
This is not Perl code anymore
It's just data that can be read via the DATA filehandle

# These look like Perl but shouldn't be parsed
my $not_a_variable = 42;
sub not_a_subroutine { }
if (this_is_not_code) {
    print "Should not appear in symbols";
}

# Can contain anything
{{{unbalanced braces are fine here}
"unclosed strings are fine
/* unmatched comments
BEGIN { malformed_code }

# Actual data formats often found in DATA sections
---
yaml_key: yaml_value
nested:
  key: value
  
[json]
{
  "type": "config",
  "values": [1, 2, 3]
}

# SQL queries
SELECT * FROM users WHERE active = 1;

# Template data
<template>
  <h1>[% title %]</h1>
  <p>[% content %]</p>
</template>

# CSV data
name,age,city
John,30,NYC
Jane,25,LA

# Another package after DATA is unusual but valid
# (though it won't be executed)