#!/usr/bin/perl
# Test cases for return statement with modifiers

# Simple return with if modifier
sub test1 {
    return $x if $cond;
}

# Return without value with if modifier
sub test2 {
    return if $cond;
}

# Return with or die and if modifier
sub test3 {
    return $x or die if $cond;
}

# Return with unless modifier
sub test4 {
    return $x unless $error;
}

# Return with while modifier (uncommon but valid)
sub test5 {
    return $x while $loop;
}

# Return with for modifier
sub test6 {
    return $_ for @list;
}

# Complex: return with word operator and modifier
sub test7 {
    return $x or warn "failed" if $debug;
}

# Multiple word operators with modifier
sub test8 {
    return $x and $y or die "error" if $check;
}