//! Continue and redo loop control statement test fixtures.
//!
//! This module provides comprehensive test coverage for:
//! - Continue blocks in while/until/for/foreach loops
//! - Redo statements with and without labels
//! - Continue interaction with next/last/redo
//! - Edge cases and nested loop scenarios

/// A continue/redo test fixture with metadata.
#[derive(Debug, Clone, Copy)]
pub struct ContinueRedoCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Tags for filtering and grouping.
    pub tags: &'static [&'static str],
    /// Perl source for the test case.
    pub source: &'static str,
    /// Whether this case is expected to parse successfully.
    pub should_parse: bool,
}

static CONTINUE_REDO_CASES: &[ContinueRedoCase] = &[
    // Basic continue block tests
    ContinueRedoCase {
        id: "continue.while.basic",
        description: "Basic continue block in while loop.",
        tags: &["continue", "while", "loop"],
        source: r#"my $i = 0;
while ($i < 3) {
    $i++;
} continue {
    print "iteration\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.until.basic",
        description: "Basic continue block in until loop.",
        tags: &["continue", "until", "loop"],
        source: r#"my $i = 0;
until ($i >= 3) {
    $i++;
} continue {
    print "iteration\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.for.basic",
        description: "Basic continue block in for loop.",
        tags: &["continue", "for", "loop"],
        source: r#"for my $i (1..3) {
    print "$i\n";
} continue {
    print "continue block\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.foreach.basic",
        description: "Basic continue block in foreach loop.",
        tags: &["continue", "foreach", "loop"],
        source: r#"my @items = (1, 2, 3);
foreach my $item (@items) {
    print "$item\n";
} continue {
    print "continue block\n";
}
"#,
        should_parse: true,
    },
    // Basic redo tests
    ContinueRedoCase {
        id: "redo.while.basic",
        description: "Basic redo statement in while loop.",
        tags: &["redo", "while", "loop"],
        source: r#"my $count = 0;
while ($count < 3) {
    $count++;
    redo if $count == 2;
    print "$count\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "redo.until.basic",
        description: "Basic redo statement in until loop.",
        tags: &["redo", "until", "loop"],
        source: r#"my $count = 0;
until ($count >= 3) {
    $count++;
    redo if $count == 2;
    print "$count\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "redo.for.basic",
        description: "Basic redo statement in for loop.",
        tags: &["redo", "for", "loop"],
        source: r#"my $redo_count = 0;
for my $i (1..3) {
    $redo_count++;
    redo if $redo_count == 2;
    print "$i\n";
}
"#,
        should_parse: true,
    },
    // Continue with next interaction
    ContinueRedoCase {
        id: "continue.next.interaction",
        description: "Continue block executed when next is called.",
        tags: &["continue", "next", "loop", "interaction"],
        source: r#"my @items = (1, 2, 3, 4, 5);
for my $item (@items) {
    next if $item % 2 == 0;
    print "$item\n";
} continue {
    print "continue: $item\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.last.interaction",
        description: "Continue block NOT executed when last is called.",
        tags: &["continue", "last", "loop", "interaction"],
        source: r#"my @items = (1, 2, 3, 4, 5);
for my $item (@items) {
    last if $item == 3;
    print "$item\n";
} continue {
    print "continue: $item\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.redo.interaction",
        description: "Continue block NOT executed when redo is called.",
        tags: &["continue", "redo", "loop", "interaction"],
        source: r#"my $count = 0;
my $redo_count = 0;
while ($count < 3) {
    $count++;
    if ($redo_count == 0 && $count == 2) {
        $redo_count++;
        redo;
    }
    print "$count\n";
} continue {
    print "continue\n";
}
"#,
        should_parse: true,
    },
    // Continue in nested loops
    ContinueRedoCase {
        id: "continue.nested.loops",
        description: "Continue blocks in nested loops.",
        tags: &["continue", "nested", "loop"],
        source: r#"for my $i (1..2) {
    for my $j (1..2) {
        print "$i,$j\n";
    } continue {
        print "inner continue\n";
    }
} continue {
    print "outer continue\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.nested.next",
        description: "Continue block with next in nested loops.",
        tags: &["continue", "next", "nested", "loop"],
        source: r#"for my $i (1..3) {
    for my $j (1..3) {
        next if $j == 2;
        print "$i,$j\n";
    } continue {
        print "inner continue: $i,$j\n";
    }
} continue {
    print "outer continue: $i\n";
}
"#,
        should_parse: true,
    },
    // Redo with labels
    ContinueRedoCase {
        id: "redo.labeled.loop",
        description: "Redo statement with loop label.",
        tags: &["redo", "label", "loop"],
        source: r#"my $count = 0;
LOOP: while ($count < 3) {
    $count++;
    redo LOOP if $count == 2;
    print "$count\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "redo.nested.labeled",
        description: "Redo statement in nested labeled loops.",
        tags: &["redo", "label", "nested", "loop"],
        source: r#"my $outer_redo = 0;
my $inner_redo = 0;
OUTER: for my $i (1..2) {
    INNER: for my $j (1..2) {
        $inner_redo++;
        redo INNER if $inner_redo == 1;
        print "$i,$j\n";
    }
    $outer_redo++;
    redo OUTER if $outer_redo == 1;
}
"#,
        should_parse: true,
    },
    // Continue block with multiple statements
    ContinueRedoCase {
        id: "continue.multiple.statements",
        description: "Continue block with multiple statements.",
        tags: &["continue", "loop", "block"],
        source: r#"my $sum = 0;
for my $i (1..3) {
    print "$i\n";
} continue {
    $sum += $i;
    print "sum: $sum\n";
    my $temp = $i * 2;
    print "temp: $temp\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.empty.block",
        description: "Continue block with no statements.",
        tags: &["continue", "loop", "block", "edge-case"],
        source: r#"for my $i (1..3) {
    print "$i\n";
} continue {
}
"#,
        should_parse: true,
    },
    // Edge cases
    ContinueRedoCase {
        id: "continue.do.while",
        description: "Continue block in do-while loop (valid Perl).",
        tags: &["continue", "do", "while", "loop", "edge-case"],
        source: r#"my $i = 0;
do {
    $i++;
    print "$i\n";
} while ($i < 3) {
    print "continue\n";
}
"#,
        should_parse: false, // This is actually invalid Perl syntax
    },
    ContinueRedoCase {
        id: "redo.do.while",
        description: "Redo statement in do-while loop.",
        tags: &["redo", "do", "while", "loop", "edge-case"],
        source: r#"my $count = 0;
my $redo_count = 0;
do {
    $count++;
    if ($redo_count == 0 && $count == 2) {
        $redo_count++;
        redo;
    }
    print "$count\n";
} while ($count < 3);
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.bare.block",
        description: "Continue block after bare block (invalid).",
        tags: &["continue", "block", "edge-case", "invalid"],
        source: r#"{
    my $x = 1;
    print "$x\n";
} continue {
    print "continue\n";
}
"#,
        should_parse: false, // Continue only works with loops
    },
    ContinueRedoCase {
        id: "redo.bare.block",
        description: "Redo statement in bare block (invalid).",
        tags: &["redo", "block", "edge-case", "invalid"],
        source: r#"{
    my $x = 1;
    redo;
    print "$x\n";
}
"#,
        should_parse: false, // Redo only works in loops
    },
    ContinueRedoCase {
        id: "continue.lexical.scope",
        description: "Continue block with lexical variable declarations.",
        tags: &["continue", "loop", "scope", "lexical"],
        source: r#"for my $i (1..3) {
    my $inner = $i * 2;
    print "$inner\n";
} continue {
    my $cont_var = $i + 1;
    print "next will be: $cont_var\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "redo.conditional",
        description: "Redo statement with conditional logic.",
        tags: &["redo", "loop", "conditional"],
        source: r#"my $attempts = 0;
my $success = 0;
while (!$success && $attempts < 5) {
    $attempts++;
    my $random = int(rand(10));
    if ($random < 3 && $attempts < 3) {
        redo;
    }
    $success = 1;
    print "success after $attempts attempts\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.postfix.modifier",
        description: "Continue block with postfix loop modifier.",
        tags: &["continue", "loop", "postfix"],
        source: r#"my @items = (1, 2, 3);
my $i = 0;
while ($i < scalar @items) {
    print $items[$i], "\n";
    $i++;
} continue {
    print "continue\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "redo.counter.reset",
        description: "Redo statement resetting loop counter.",
        tags: &["redo", "loop", "counter"],
        source: r#"my $retries = 0;
my $max_retries = 3;
for my $i (1..5) {
    if ($i == 3 && $retries < $max_retries) {
        $retries++;
        redo;
    }
    print "processing: $i\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.next.last.redo",
        description: "Continue block demonstrating all control flow statements.",
        tags: &["continue", "next", "last", "redo", "loop", "comprehensive"],
        source: r#"my $redo_done = 0;
OUTER: for my $i (1..5) {
    INNER: for my $j (1..3) {
        next INNER if $j == 1;
        if ($j == 2 && !$redo_done) {
            $redo_done = 1;
            redo INNER;
        }
        last OUTER if $i == 4 && $j == 3;
        print "$i,$j\n";
    } continue {
        print "inner continue: $i,$j\n";
    }
} continue {
    print "outer continue: $i\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "continue.grep.map",
        description: "Continue block with grep and map operations.",
        tags: &["continue", "loop", "grep", "map"],
        source: r#"my @numbers = (1..10);
my @result;
for my $n (@numbers) {
    push @result, $n if $n % 2 == 0;
} continue {
    my $doubled = $n * 2;
    print "doubled: $doubled\n";
}
"#,
        should_parse: true,
    },
    ContinueRedoCase {
        id: "redo.subroutine.call",
        description: "Redo statement with subroutine call determining retry.",
        tags: &["redo", "loop", "subroutine"],
        source: r#"sub should_retry {
    return $_[0] < 2;
}

my $count = 0;
while ($count < 5) {
    $count++;
    redo if should_retry($count);
    print "$count\n";
}
"#,
        should_parse: true,
    },
];

/// Return all continue/redo test fixtures.
pub fn continue_redo_cases() -> &'static [ContinueRedoCase] {
    CONTINUE_REDO_CASES
}

/// Find a continue/redo test case by ID.
pub fn find_case(id: &str) -> Option<&'static ContinueRedoCase> {
    CONTINUE_REDO_CASES.iter().find(|case| case.id == id)
}

/// Get all cases matching a specific tag.
pub fn cases_by_tag(tag: &str) -> Vec<&'static ContinueRedoCase> {
    CONTINUE_REDO_CASES.iter().filter(|case| case.tags.contains(&tag)).collect()
}

/// Get all cases that should parse successfully.
pub fn valid_cases() -> Vec<&'static ContinueRedoCase> {
    CONTINUE_REDO_CASES.iter().filter(|case| case.should_parse).collect()
}

/// Get all cases that should fail to parse.
pub fn invalid_cases() -> Vec<&'static ContinueRedoCase> {
    CONTINUE_REDO_CASES.iter().filter(|case| !case.should_parse).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn all_cases_have_ids() {
        assert!(continue_redo_cases().iter().all(|case| !case.id.is_empty()));
    }

    #[test]
    fn all_cases_have_descriptions() {
        assert!(continue_redo_cases().iter().all(|case| !case.description.is_empty()));
    }

    #[test]
    fn all_cases_have_source() {
        assert!(continue_redo_cases().iter().all(|case| !case.source.is_empty()));
    }

    #[test]
    fn all_case_ids_are_unique() {
        let mut seen = HashSet::new();
        for case in continue_redo_cases() {
            assert!(seen.insert(case.id), "Duplicate case id: {}", case.id);
        }
    }

    #[test]
    fn can_filter_by_tag() {
        let continue_cases = cases_by_tag("continue");
        assert!(!continue_cases.is_empty());
        assert!(continue_cases.iter().all(|case| case.tags.contains(&"continue")));
    }

    #[test]
    fn can_find_by_id() {
        let case = find_case("continue.while.basic");
        must_some(case);
        assert_eq!(must_some(case).id, "continue.while.basic");
    }

    #[test]
    fn valid_and_invalid_cases_partition() {
        let total = continue_redo_cases().len();
        let valid = valid_cases().len();
        let invalid = invalid_cases().len();
        assert_eq!(total, valid + invalid);
    }

    #[test]
    fn has_basic_continue_cases() {
        assert!(find_case("continue.while.basic").is_some());
        assert!(find_case("continue.until.basic").is_some());
        assert!(find_case("continue.for.basic").is_some());
        assert!(find_case("continue.foreach.basic").is_some());
    }

    #[test]
    fn has_basic_redo_cases() {
        assert!(find_case("redo.while.basic").is_some());
        assert!(find_case("redo.until.basic").is_some());
        assert!(find_case("redo.for.basic").is_some());
    }

    #[test]
    fn has_interaction_cases() {
        assert!(find_case("continue.next.interaction").is_some());
        assert!(find_case("continue.last.interaction").is_some());
        assert!(find_case("continue.redo.interaction").is_some());
    }

    #[test]
    fn has_labeled_redo_cases() {
        assert!(find_case("redo.labeled.loop").is_some());
        assert!(find_case("redo.nested.labeled").is_some());
    }

    #[test]
    fn has_edge_cases() {
        assert!(find_case("continue.empty.block").is_some());
        assert!(find_case("redo.do.while").is_some());
        assert!(find_case("continue.bare.block").is_some());
    }

    #[test]
    fn edge_cases_marked_correctly() {
        let bare_block = find_case("continue.bare.block");
        must_some(bare_block);
        assert!(!must_some(bare_block).should_parse, "continue on bare block should be invalid");

        let redo_bare = find_case("redo.bare.block");
        must_some(redo_bare);
        assert!(!must_some(redo_bare).should_parse, "redo in bare block should be invalid");
    }
}
