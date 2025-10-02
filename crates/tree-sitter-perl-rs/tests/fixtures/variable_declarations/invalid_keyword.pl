#!/usr/bin/env perl
# Invalid variable declaration keyword test fixtures
# Tests for AC1: Variable declaration error handling
# These should trigger descriptive errors instead of unreachable!()

# Invalid keyword 'const' (Perl doesn't have const keyword for variables)
const $invalid = 1;

# Invalid keyword 'var' (not a Perl keyword)
var $invalid = 2;

# Invalid keyword 'let' (not a Perl keyword)
let $invalid = 3;

# Typo in keyword 'may' instead of 'my'
may $typo = 4;

# Typo in keyword 'oar' instead of 'our'
oar $typo2 = 5;

# Using 'sub' where declaration expected
sub $invalid = 6;

# Using 'package' where declaration expected
package $invalid = 7;
