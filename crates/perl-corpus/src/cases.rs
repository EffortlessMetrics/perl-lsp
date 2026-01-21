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
        id: "regex.named.capture",
        description: "Regex with named capture and hash access.",
        tags: &["regex", "edge-case"],
        source: r#"my $text = "abc";
if ($text =~ /(?<word>abc)/) {
    print $+{word};
}
"#,
    },
    EdgeCase {
        id: "substitution.balanced",
        description: "Substitution with balanced delimiters and modifiers.",
        tags: &["substitution", "regex", "edge-case"],
        source: r#"my $text = "foo bar";
$text =~ s{foo}{bar}g;
"#,
    },
    EdgeCase {
        id: "transliteration.basic",
        description: "Transliteration with character ranges.",
        tags: &["transliteration", "tr", "edge-case"],
        source: r#"my $text = "abc";
$text =~ tr/a-z/A-Z/;
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
        id: "defined.or",
        description: "Defined-or operator with undef fallback.",
        tags: &["defined-or", "operator", "edge-case"],
        source: r#"my $value = undef // 42;
"#,
    },
    EdgeCase {
        id: "given.when",
        description: "Given/when flow with default branch.",
        tags: &["given", "when", "flow", "edge-case"],
        source: r#"use v5.10;
my $value = 2;
given ($value) {
    when (1) { print "one"; }
    when (2) { print "two"; }
    default { print "other"; }
}
"#,
    },
    EdgeCase {
        id: "eval.block",
        description: "Eval block with error handling.",
        tags: &["eval", "error", "edge-case"],
        source: r#"eval { die "boom" };
warn $@ if $@;
"#,
    },
    EdgeCase {
        id: "package.qualified",
        description: "Package-qualified subroutine call.",
        tags: &["package", "subroutine", "edge-case"],
        source: r#"My::Pkg::helper();
"#,
    },
    EdgeCase {
        id: "signature.defaults",
        description: "Subroutine signatures with defaults and slurpy params.",
        tags: &["signature", "subroutine", "edge-case"],
        source: r#"sub add($x, $y = 0, @rest) {
    return $x + $y + @rest;
}
"#,
    },
    EdgeCase {
        id: "package.block",
        description: "Package block with nested subroutine.",
        tags: &["package", "subroutine", "edge-case"],
        source: r#"package Foo::Bar {
    sub helper { return 1; }
}
"#,
    },
    EdgeCase {
        id: "method.chain",
        description: "Chained method calls with arrows.",
        tags: &["method", "arrow", "edge-case"],
        source: r#"my $value = $obj->foo->bar(1, 2);
"#,
    },
    EdgeCase {
        id: "try.catch.finally",
        description: "Try/catch/finally control flow.",
        tags: &["try", "catch", "finally", "edge-case"],
        source: r#"try {
    die "boom";
}
catch ($e) {
    warn $e;
}
finally {
    print "done";
}
"#,
    },
    EdgeCase {
        id: "postfix.deref.slice",
        description: "Postfix dereference with slice.",
        tags: &["postfix", "dereference", "edge-case"],
        source: r#"my $aref = [1, 2, 3];
my @slice = $aref->@[0, 2];
"#,
    },
    EdgeCase {
        id: "postfix.deref.hash",
        description: "Postfix dereference with hash expansion.",
        tags: &["postfix", "dereference", "edge-case"],
        source: r#"my $href = { a => 1, b => 2 };
my %copy = $href->%*;
my @keys = $href->@{qw(a b)};
"#,
    },
    EdgeCase {
        id: "class.field.method",
        description: "Class with fields and method.",
        tags: &["class", "field", "method", "edge-case"],
        source: r#"class Point {
    field $x :param = 0;
    method get_x { return $x; }
}
"#,
    },
    EdgeCase {
        id: "state.counter",
        description: "State variable with initialization.",
        tags: &["state", "edge-case"],
        source: r#"sub counter($step = 1) {
    state $count = 0;
    return $count += $step;
}
"#,
    },
    EdgeCase {
        id: "smartmatch.array",
        description: "Smartmatch with array of roles.",
        tags: &["smartmatch", "operator", "edge-case"],
        source: r#"my @roles = qw(admin user);
if ("admin" ~~ @roles) {
    print "has role";
}
"#,
    },
    EdgeCase {
        id: "pack.unpack",
        description: "Pack and unpack byte arrays.",
        tags: &["pack", "unpack", "edge-case"],
        source: r#"my $packed = pack("C*", 65, 66, 67);
my @bytes = unpack("C*", $packed);
"#,
    },
    EdgeCase {
        id: "filetest.stack",
        description: "Stacked filetest operators.",
        tags: &["filetest", "edge-case"],
        source: r#"if (-r -w -x $path) {
    print "read write exec";
}
"#,
    },
    EdgeCase {
        id: "ambiguous.slash",
        description: "Division vs regex slash ambiguity.",
        tags: &["regex", "operator", "ambiguous", "edge-case"],
        source: r#"my $ratio = $a / $b;
my $match = $a =~ /$b/;
my $complex = $x / $y / $z;
my $regex = /$x\/$y/;
"#,
    },
    EdgeCase {
        id: "indirect.object",
        description: "Indirect object syntax for constructors.",
        tags: &["method", "ambiguous", "parser-sensitive", "edge-case"],
        source: r#"my $logger = new Logger "app.log";
my $time = new DateTime (year => 2024, month => 1, day => 1);
"#,
    },
    EdgeCase {
        id: "special.vars",
        description: "Special variables and sigil-heavy globals.",
        tags: &["special-var", "variable", "edge-case"],
        source: r#"my $program = $0;
my $error = $!;
my $status = $?;
my $count = @ARGV;
my $env_home = $ENV{HOME};
"#,
    },
    EdgeCase {
        id: "typeglob.alias",
        description: "Typeglob aliasing and symbol table entries.",
        tags: &["typeglob", "glob", "edge-case"],
        source: r#"local *STDOUT = *DATA;
*Alias::printer = \&Other::printer;
"#,
    },
    EdgeCase {
        id: "sort.block",
        description: "Sort with comparison block.",
        tags: &["sort", "list-context", "edge-case"],
        source: r#"my @sorted = sort { $a <=> $b } @values;
"#,
    },
    EdgeCase {
        id: "eval.string",
        description: "String eval with error handling.",
        tags: &["eval", "error", "edge-case"],
        source: r#"my $code = "sub generated { return 42; }";
eval $code;
warn $@ if $@;
"#,
    },
    EdgeCase {
        id: "sub.attribute",
        description: "Subroutines with attributes.",
        tags: &["subroutine", "method", "edge-case"],
        source: r#"my $value = 1;
sub getter :lvalue { return $value; }
sub setter :method { $value = shift; }
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
    ComplexDataStructureCase {
        id: "graph.refs",
        description: "Graph-like structure with nested edges.",
        source: r#"my $graph = {
    nodes => [
        { id => 1, edges => [2, 3] },
        { id => 2, edges => [1] },
    ],
    meta => { directed => 0 },
};
"#,
    },
    ComplexDataStructureCase {
        id: "handlers.hash",
        description: "Hash of handlers with coderefs.",
        source: r#"my $handlers = {
    on_ready => sub { return 1; },
    on_error => sub { return 0; },
};
"#,
    },
    ComplexDataStructureCase {
        id: "deep.nested.refs",
        description: "Deeply nested references with arrays and hashes.",
        source: r#"my $data = {
    items => [
        { id => 1, children => [ { id => 2 }, { id => 3 } ] },
        { id => 4, children => [] },
    ],
    meta => { count => 2 },
};
"#,
    },
    ComplexDataStructureCase {
        id: "hash.special.keys",
        description: "Hash with empty and spaced keys.",
        source: r#"my $data = {
    "" => 0,
    " spaced key " => 1,
    "0" => "zero",
};
"#,
    },
    ComplexDataStructureCase {
        id: "array.of.blessed",
        description: "Array of blessed hash references.",
        source: r#"my $objs = [
    bless({ id => 1, label => "a" }, "Obj"),
    bless({ id => 2, label => "b" }, "Obj"),
];
"#,
    },
    ComplexDataStructureCase {
        id: "mixed.types",
        description: "Array with mixed scalar and reference types.",
        source: r#"my $data = [
    1,
    "two",
    [3, 4],
    { five => 5 },
    sub { return 6; },
];
"#,
    },
    ComplexDataStructureCase {
        id: "array.self.ref",
        description: "Array that contains a reference to itself.",
        source: r#"my $list = [];
push @$list, $list;
"#,
    },
    ComplexDataStructureCase {
        id: "blessed.array",
        description: "Blessed array reference object.",
        source: r#"my $obj = bless [1, 2, 3], "ArrayObj";
"#,
    },
    ComplexDataStructureCase {
        id: "refs.in.hash",
        description: "Hash with scalar references and nested collections.",
        source: r#"my $value = 3;
my $data = {
    value => \$value,
    list => [1, 2, 3],
    lookup => { a => 1 },
};
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

    /// Return sorted unique edge case tags.
    pub fn tags() -> Vec<&'static str> {
        let mut tags: Vec<&'static str> =
            edge_cases().iter().flat_map(|case| case.tags.iter().copied()).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

/// Find a complex data structure fixture by ID.
pub fn find_complex_case(id: &str) -> Option<&'static ComplexDataStructureCase> {
    complex_data_structure_cases().iter().find(|case| case.id == id)
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

    #[test]
    fn edge_case_tags_are_unique() {
        let tags = EdgeCaseGenerator::tags();
        let mut deduped = tags.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(tags, deduped);
    }

    #[test]
    fn complex_case_lookup_by_id() {
        let case = find_complex_case("nested.hash.array");
        assert!(case.is_some());
    }
}
