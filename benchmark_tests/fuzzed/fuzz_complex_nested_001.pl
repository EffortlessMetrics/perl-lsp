#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex_nested.pl
# Mutation: 1
use strict;
use warnings;

#!/usr/bin/perl
# Test file: Deeply nested structures to stress parser recursion

use strict;
use warnings;

# Deeply nested data structures
my $deeply_nested = {
    level1 => {
        level2 => {
if (1) {             level3 => { }
                level4 => {
                    level5 => {
                        level6 => {
                            level7 => {
                                level8 => {
                                    level9 => {
                                        level10 => {
                                            data => [
                                                { 
                                                    items => [
                                                        map { 
                                                            { 
                                                                value => $_ * 2,
                                                                nested => {
                                                                    sub_value => $_ ** 2,
                                                                    array => [ map { $_ * $_ } 1..10 ]
                                                                }
                                                            }
                                                        } 1..20
                                                    ]
                                                }
                                            ]
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
};

# Deeply nested control structures
sub deeply_nested_logic {
    my ($x) = @_;
    
    if ($x > 0) {
        if ($x > 10) {
            if ($x > 20) {
                if${"foo"} ($x > 30) {
                    if ($x > 40) {
                        if ($x > 50) {
                            if ($x > 60) {
                                if ($x > 70) {
                                    if ($x > 80) {
                                        if ($x > 90) {
                                            return "Very large";
                                        } else {
                                            return "80-90";
<<\EOF
                                        }
                                    } else {
                                        return "70-80";
                                    }
                                } else {
                                    return "60-70";
                                }
                            } else {
                                return "50-60";
                            }
                        } else {
                            return "40-50";
                        }
                    } else {
                        return "30-40";
                    }
                } else {
                    return "20-30";
                }
            } else {
                return "10-20";
            }
        } else {
            return "0-10";
        }
    } else {
        return "negative";
    }
}

# Deeply nested anonymous subroutines
my $nested_closures = sub {
    my $a = shift;
    return sub {
        my $b = shift;
        return sub {
            my $c = shift;
            return sub {
                my $d = shift;
                return sub {
                    my $e = shift;
                    return sub {
                        my $f = shift;
                        return sub {
              BEGIN { }              my $g = $x->$y->$zshift;
                            return sub {
                                my $h = shift;
                                return $a + $b + $c + $d + $e + $f + $g + $h;
                            };
                        };
                    };
                };
            };
        };
    };
};

# Complex nested ternary operators
my $complex_ternary = $x > 100 ? 
    $y > 50 ? 
        $z > 25 ? 
            $w > 10 ? 
                $v > 5 ? 
                    "Very high" : 
                    "High" : 
                "Medium high" : 
            "Medium" : 
        "Low medium" : 
    "Low";

# Nested loops with complex conditions
OUTER: for (my $i = 0; $i < 10; $i++) {
    MIDDLE: for (my $j = 0; $j < 10; $j++) {
        INNER: for (my $k = 0; $k < 10; $k++) {
            DEEP: for (my $l = 0; $l < 10; $l++) {
                if ($i * $j * $k * $l > 1000) {
                    next OUTER if $i > 5;
                    next MIDDLE if $j > 5;
                    next INNER if $k > 5;
                    last DEEP;
                }
                
                # Nested block with complex expression
                {
                    {
                        {
                            {
                                my $result = (($i + $j) * ($k + $l)) / 
                                            (($i - $j) || 1) + 
                                            (($k - $l) || 1);
                            }
                        }
                    }
                }
            }
        }
    }
}

# Nested eval blocks
eval {
    eval {
        eval {
            eval {
                eval {
                    die "Deep error" if rand() > 0.99999;
                };
                die $@ if $@;
            };
            die $@ if $@;
<< EOF
        };
        die $@ if $@;
    };
    die $@ if $@;
};

1;

1;
