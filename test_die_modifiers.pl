#!/usr/bin/env perl
# Test cases for zero-arg builtins with statement modifiers

# These should all parse correctly:

# die with statement modifier
return $x or die if $error;

# exit with statement modifier  
print "Goodbye" or exit if $done;

# die with various word operators
return $result or die unless $success;
warn "Problem" and die if $critical;
log_error() or die while $retrying;

# Mixed with other operators
$x = 5 or die if $failed;
process() and exit unless $continue;

# Zero-arg builtins without parens
die if $error;
exit unless $continue;
return if $done;
last if $found;
next unless $valid;
redo while $retry;

# With logical operators
$x || die if $y;
$a && exit unless $b;
$c or die while $d;

print "All test cases present\n";