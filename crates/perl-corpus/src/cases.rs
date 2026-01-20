//! Static edge case fixtures and complex data structure samples.

/// A single edge case fixture with metadata.
#[derive(Debug, Clone, Copy)]
pub struct EdgeCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Tags for filtering and grouping.
    pub tags: &'static [&'static str],
    /// Perl source for the edge case.
    pub source: &'static str,
}

/// A complex data structure fixture for DAP/LSP variable inspection.
#[derive(Debug, Clone, Copy)]
pub struct ComplexDataStructureCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Perl source for the fixture.
    pub source: &'static str,
}

static EDGE_CASES: &[EdgeCase] = &[
    EdgeCase {
        id: "heredoc.basic",
        description: "Basic quoted heredoc with multiple lines.",
        tags: &["heredoc", "edge-case"],
        source: r#"my $text = <<'EOF';
line one
line two
EOF
"#,
    },
    EdgeCase {
        id: "heredoc.indented",
        description: "Indented heredoc using <<~ syntax.",
        tags: &["heredoc", "edge-case"],
        source: r#"my $text = <<~EOF;
  indented line
EOF
"#,
    },
    EdgeCase {
        id: "quote.like",
        description: "Quote-like operator with interpolation.",
        tags: &["quote-like", "interpolation"],
        source: r#"my $name = "Ada";
my $text = qq{Hello $name};
"#,
    },
    EdgeCase {
        id: "regex.code",
        description: "Regex with embedded code block.",
        tags: &["regex", "regex-code", "edge-case"],
        source: r#"my $count = 0;
"x" =~ /(?{ $count++ })x/;
"#,
    },
    EdgeCase {
        id: "map.grep",
        description: "Map/grep with block syntax.",
        tags: &["map", "grep", "list-context"],
        source: r#"my @nums = (1, 2, 3);
my @doubled = map { $_ * 2 } @nums;
my @even = grep { $_ % 2 == 0 } @nums;
"#,
    },
    EdgeCase {
        id: "format.statement",
        description: "Format statement with picture lines.",
        tags: &["format", "legacy", "edge-case"],
        source: r#"my ($name, $age) = ("Ada", 37);
format STDOUT =
@<<<<<< @>>>>>
$name, $age
.
write;
"#,
    },
    EdgeCase {
        id: "glob.angle",
        description: "Glob expression using angle brackets.",
        tags: &["glob", "file", "edge-case"],
        source: r#"my @files = <*.pl>;
my @more = glob "*.pm";
"#,
    },
    EdgeCase {
        id: "tie.hash",
        description: "Tie and untie a hash.",
        tags: &["tie", "hash", "edge-case"],
        source: r#"tie my %cache, "Tie::StdHash";
$cache{a} = 1;
untie %cache;
"#,
    },
    EdgeCase {
        id: "redo.loop",
        description: "Redo inside a loop.",
        tags: &["redo", "loop", "edge-case"],
        source: r#"my $count = 0;
while ($count < 3) {
    $count++;
    redo if $count == 2;
}
"#,
    },
    EdgeCase {
        id: "continue.block",
        description: "Continue block after a for loop.",
        tags: &["continue", "loop", "edge-case"],
        source: r#"for my $i (1..3) {
    next if $i == 2;
} continue {
    my $j = $i * 2;
}
"#,
    },
    EdgeCase {
        id: "package.qualified",
        description: "Package-qualified subroutine call.",
        tags: &["package", "subroutine", "edge-case"],
        source: r#"My::Pkg::helper();
"#,
    },
];

static COMPLEX_DATA_STRUCTURE_CASES: &[ComplexDataStructureCase] = &[
    ComplexDataStructureCase {
        id: "nested.hash.array",
        description: "Nested hash/array structure.",
        source: r#"my $data = {
    users => [
        { id => 1, name => "Ada" },
        { id => 2, name => "Bob" },
    ],
    flags => { active => 1, admin => 0 },
};
"#,
    },
    ComplexDataStructureCase {
        id: "circular.reference",
        description: "Self-referential hash.",
        source: r#"my $node = {};
$node->{self} = $node;
"#,
    },
    ComplexDataStructureCase {
        id: "blessed.object",
        description: "Blessed hash reference.",
        source: r#"my $obj = bless { name => "Widget", count => 3 }, "My::Class";
"#,
    },
    ComplexDataStructureCase {
        id: "mapped.records",
        description: "Array of hash records created via map.",
        source: r#"my @values = map { { id => $_, name => "item_$_" } } (1..5);
"#,
    },
    ComplexDataStructureCase {
        id: "typeglob.alias",
        description: "Typeglob aliasing and filehandle.",
        source: r#"open my $fh, "<", "file.txt";
*ALIAS = *STDOUT;
"#,
    },
];

/// Return the static edge case fixtures.
pub fn edge_cases() -> &'static [EdgeCase] {
    EDGE_CASES
}

/// Return the static complex data structure fixtures.
pub fn complex_data_structure_cases() -> &'static [ComplexDataStructureCase] {
    COMPLEX_DATA_STRUCTURE_CASES
}

/// Backwards-compatible accessor for complex data structure fixtures.
pub fn get_complex_data_structure_tests() -> &'static [ComplexDataStructureCase] {
    complex_data_structure_cases()
}

/// Convenience helper for working with static edge cases.
pub struct EdgeCaseGenerator;

impl EdgeCaseGenerator {
    /// Return all available edge cases.
    pub fn all_cases() -> &'static [EdgeCase] {
        edge_cases()
    }

    /// Return edge cases with a matching tag.
    pub fn by_tag(tag: &str) -> Vec<&'static EdgeCase> {
        edge_cases()
            .iter()
            .filter(|case| case.tags.iter().any(|t| *t == tag))
            .collect()
    }

    /// Find a single edge case by ID.
    pub fn find(id: &str) -> Option<&'static EdgeCase> {
        edge_cases().iter().find(|case| case.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_cases_have_ids() {
        assert!(edge_cases().iter().all(|case| !case.id.is_empty()));
    }

    #[test]
    fn edge_cases_can_filter_by_tag() {
        let heredocs = EdgeCaseGenerator::by_tag("heredoc");
        assert!(!heredocs.is_empty());
    }
}
