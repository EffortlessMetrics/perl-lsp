#!/bin/bash
echo "Testing: return if 1;"
echo 'return if 1;' | ./target/debug/perl-parse - 2>&1

echo -e "\nTesting: return \$x if \$cond;"
echo 'return $x if $cond;' | ./target/debug/perl-parse - 2>&1

echo -e "\nTesting: return \$x or die if \$error;"
echo 'return $x or die if $error;' | ./target/debug/perl-parse - 2>&1