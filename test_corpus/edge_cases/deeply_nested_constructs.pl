#!/usr/bin/env perl
# Test: Deeply nested constructs
# Impact: Test parser recursion limits and stack overflow protection

use strict;
use warnings;

# Test 1: Deeply nested if-else statements (20 levels)
my $result = 0;
if ($result) {
    if ($result > 1) {
        if ($result > 2) {
            if ($result > 3) {
                if ($result > 4) {
                    if ($result > 5) {
                        if ($result > 6) {
                            if ($result > 7) {
                                if ($result > 8) {
                                    if ($result > 9) {
                                        if ($result > 10) {
                                            if ($result > 11) {
                                                if ($result > 12) {
                                                    if ($result > 13) {
                                                        if ($result > 14) {
                                                            if ($result > 15) {
                                                                if ($result > 16) {
                                                                    if ($result > 17) {
                                                                        if ($result > 18) {
                                                                            if ($result > 19) {
                                                                                $result = 20;
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

# Test 2: Deeply nested loops (15 levels)
my $count = 0;
for my $i (0..1) {
    for my $j (0..1) {
        for my $k (0..1) {
            for my $l (0..1) {
                for my $m (0..1) {
                    for my $n (0..1) {
                        for my $o (0..1) {
                            for my $p (0..1) {
                                for my $q (0..1) {
                                    for my $r (0..1) {
                                        for my $s (0..1) {
                                            for my $t (0..1) {
                                                for my $u (0..1) {
                                                    for my $v (0..1) {
                                                        $count++;
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
            }
        }
    }
}

# Test 3: Mixed nesting types (loops within conditionals within blocks)
my $value = 0;
BLOCK1: {
    if ($value) {
        for my $i (0..1) {
            if ($i == $value) {
                while ($i < 2) {
                    eval {
                        if ($i == 1) {
                            do {
                                until ($value > 5) {
                                    for my $j (0..1) {
                                        if ($j == $i) {
                                            last if $value > 3;
                                            $value++;
                                        }
                                    }
                                    $value++;
                                }
                            } while ($value < 10);
                        }
                    };
                    $i++;
                }
            }
        }
    }
}

# Test 4: Deeply nested data structures
my $deep_ref = {
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    level5 => {
                        level6 => {
                            level7 => {
                                level8 => {
                                    level9 => {
                                        level10 => {
                                            data => [
                                                {
                                                    nested_array => [
                                                        {
                                                            deep_hash => {
                                                                final_value => "deep"
                                                            }
                                                        }
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

# Test 5: Complex nested array references with mixed types
my $complex_nested = [
    [1, 2, [3, 4, [5, 6, [7, 8, [9, 10]]]]],
    {
        nested => {
            arrays => [
                [1, 2, { deep => { structure => [1, 2, 3] } }],
                ["string", { hash => { in => { array => [1, 2, 3] } } }]
            ]
        }
    },
    sub {
        my $deep = shift;
        return {
            result => $deep->{level1}->{level2}->{level3} || "default"
        };
    }
];

# Test 6: Nested function calls with complex arguments
my $nested_result = func1(
    func2(
        func3(
            func4(
                func5(
                    func6(
                        func7(
                            func8(
                                func9(
                                    func10("deep")
                                )
                            )
                        )
                    )
                )
            )
        )
    )
);

# Test 7: Deeply nested ternary operations
my $ternary_deep = $value ? 
    ($value > 1 ? 
        ($value > 2 ? 
            ($value > 3 ? 
                ($value > 4 ? 
                    ($value > 5 ? 
                        ($value > 6 ? 
                            ($value > 7 ? 
                                ($value > 8 ? 
                                    ($value > 9 ? 
                                        "very deep" : "level 9")
                                    : "level 8")
                                : "level 7")
                            : "level 6")
                        : "level 5")
                    : "level 4")
                : "level 3")
            : "level 2")
        : "level 1")
    : "zero";

# Test 8: Nested grep/map/sort operations
my $nested_operations = 
    map { $_ * 2 } 
    grep { $_ % 2 == 0 } 
    sort { $b <=> $a } 
    map { $_ + 1 } 
    grep { $_ > 5 } 
    map { $_ * 3 } 
    grep { $_ < 20 } 
    (1..50);

# Test 9: Complex regex with nested groups and alternations
my $complex_regex = qr/
    ^(
        (
            (
                (?:[a-zA-Z]+)
                |
                (?:\d+)
            )
            (
                (?:\.[a-zA-Z]+)?
                |
                (?:\.\d+)?
            )
        )
        |
        (
            (
                (?:[+-]?\d*\.\d+)
                |
                (?:[+-]?\d+)
            )
            (?:[eE][+-]?\d+)?
        )
    )
$/x;

# Test 10: Deeply nested BEGIN/END blocks with complex interactions
BEGIN {
    my $init = 0;
    BEGIN {
        $init++;
        BEGIN {
            $init++;
            BEGIN {
                $init++;
                BEGIN {
                    $init++;
                }
            }
        }
    }
}

END {
    my $cleanup = 0;
    END {
        $cleanup++;
        END {
            $cleanup++;
            END {
                $cleanup++;
                END {
                    $cleanup++;
                }
            }
        }
    }
}

# Test 11: Nested try-catch blocks with eval
try {
    try {
        try {
            try {
                try {
                    eval {
                        if ($value) {
                            die "Deep error";
                        }
                    };
                } catch ($e) {
                    warn "Caught at level 4: $e";
                }
            } catch ($e) {
                warn "Caught at level 3: $e";
            }
        } catch ($e) {
            warn "Caught at level 2: $e";
        }
    } catch ($e) {
        warn "Caught at level 1: $e";
    }
} catch ($e) {
    warn "Caught at level 0: $e";
}

# Test 12: Complex nested heredocs with interpolation
my $heredoc_nested = <<'OUTER';
This is outer heredoc
@{[
    <<'INNER',
This is inner heredoc
INNER
    " and interpolated value"
]}
OUTER

print "Deeply nested constructs test completed\n";
print "Count: $count, Result: $result, Value: $value\n";