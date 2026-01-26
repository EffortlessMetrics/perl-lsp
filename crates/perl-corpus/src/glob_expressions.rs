//! Glob expression test fixtures for file pattern matching and diamond operator.

/// A glob expression test fixture with metadata.
#[derive(Debug, Clone, Copy)]
pub struct GlobExpressionCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Tags for filtering and grouping.
    pub tags: &'static [&'static str],
    /// Perl source for the glob expression.
    pub source: &'static str,
}

static GLOB_EXPRESSION_CASES: &[GlobExpressionCase] = &[
    GlobExpressionCase {
        id: "glob.function.basic",
        description: "Basic glob() function with simple wildcard.",
        tags: &["glob", "file", "builtin"],
        source: r#"my @files = glob("*.txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.function.star",
        description: "Glob function with star wildcard.",
        tags: &["glob", "file", "wildcard"],
        source: r#"my @perl_files = glob("*.pl");
my @pm_files = glob("*.pm");
"#,
    },
    GlobExpressionCase {
        id: "glob.diamond.basic",
        description: "Diamond operator with simple pattern.",
        tags: &["glob", "diamond", "file"],
        source: r#"my @files = <*.pl>;
"#,
    },
    GlobExpressionCase {
        id: "glob.diamond.multiple",
        description: "Multiple diamond glob expressions.",
        tags: &["glob", "diamond", "file"],
        source: r#"my @files = <*.pl>;
my @more = <*.pm>;
my @libs = <lib/*.pm>;
"#,
    },
    GlobExpressionCase {
        id: "glob.wildcard.question",
        description: "Glob with question mark wildcard matching single character.",
        tags: &["glob", "file", "wildcard"],
        source: r#"my @files = glob("file?.txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.wildcard.brackets",
        description: "Glob with character class brackets.",
        tags: &["glob", "file", "wildcard", "character-class"],
        source: r#"my @files = glob("file[abc].txt");
my @numbered = glob("test[0-9].pl");
"#,
    },
    GlobExpressionCase {
        id: "glob.wildcard.brackets.negated",
        description: "Glob with negated character class.",
        tags: &["glob", "file", "wildcard", "character-class"],
        source: r#"my @files = glob("file[!0-9].txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.brace.expansion",
        description: "Glob with brace expansion for alternatives.",
        tags: &["glob", "file", "brace-expansion"],
        source: r#"my @files = glob("{foo,bar,baz}.txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.brace.nested",
        description: "Glob with nested brace expansion.",
        tags: &["glob", "file", "brace-expansion"],
        source: r#"my @files = glob("file{1,{2,3}}.txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.directory.pattern",
        description: "Glob with directory path pattern.",
        tags: &["glob", "file", "directory"],
        source: r#"my @files = glob("lib/*.pm");
my @tests = glob("t/*.t");
"#,
    },
    GlobExpressionCase {
        id: "glob.directory.nested",
        description: "Glob with nested directory patterns.",
        tags: &["glob", "file", "directory"],
        source: r#"my @files = glob("lib/Foo/*.pm");
my @more = glob("lib/*/Helper.pm");
"#,
    },
    GlobExpressionCase {
        id: "glob.recursive.doublestar",
        description: "Glob with recursive doublestar pattern (system-dependent).",
        tags: &["glob", "file", "recursive"],
        source: r#"use File::Glob ':bsd_glob';
my @files = bsd_glob("lib/**/*.pm", GLOB_BRACE);
"#,
    },
    GlobExpressionCase {
        id: "glob.list.context",
        description: "Glob in list context returning multiple files.",
        tags: &["glob", "file", "list-context"],
        source: r#"my @all = glob("*.{pl,pm,t}");
for my $file (@all) {
    print "$file\n";
}
"#,
    },
    GlobExpressionCase {
        id: "glob.scalar.context",
        description: "Glob in scalar context returning one result at a time.",
        tags: &["glob", "file", "scalar-context"],
        source: r#"while (my $file = glob("*.txt")) {
    print "$file\n";
}
"#,
    },
    GlobExpressionCase {
        id: "glob.variable.pattern",
        description: "Glob with pattern from variable.",
        tags: &["glob", "file", "variable"],
        source: r#"my $pattern = "*.pl";
my @files = glob($pattern);
"#,
    },
    GlobExpressionCase {
        id: "glob.variable.interpolation",
        description: "Glob with interpolated variable in pattern.",
        tags: &["glob", "file", "variable", "interpolation"],
        source: r#"my $ext = "txt";
my @files = glob("*.$ext");
my $dir = "lib";
my @modules = glob("$dir/*.pm");
"#,
    },
    GlobExpressionCase {
        id: "glob.diamond.variable",
        description: "Diamond operator with variable interpolation.",
        tags: &["glob", "diamond", "file", "variable"],
        source: r#"my $pattern = "*.pl";
my @files = <$pattern>;
"#,
    },
    GlobExpressionCase {
        id: "glob.combined.wildcards",
        description: "Glob combining multiple wildcard types.",
        tags: &["glob", "file", "wildcard"],
        source: r#"my @files = glob("test?[0-9]*.{pl,pm}");
"#,
    },
    GlobExpressionCase {
        id: "glob.absolute.path",
        description: "Glob with absolute path pattern.",
        tags: &["glob", "file", "path"],
        source: r#"my @files = glob("/tmp/*.txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.tilde.expansion",
        description: "Glob with tilde home directory expansion.",
        tags: &["glob", "file", "path"],
        source: r#"my @files = glob("~/*.txt");
my @config = glob("~/.config/*");
"#,
    },
    GlobExpressionCase {
        id: "glob.empty.result",
        description: "Glob pattern that may return empty list.",
        tags: &["glob", "file", "edge-case"],
        source: r#"my @files = glob("nonexistent*.txt");
if (@files) {
    print "found files\n";
} else {
    print "no matches\n";
}
"#,
    },
    GlobExpressionCase {
        id: "glob.special.chars.escaped",
        description: "Glob with escaped special characters.",
        tags: &["glob", "file", "escape"],
        source: r#"my @files = glob("file\\*.txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.file.glob.module",
        description: "File::Glob module with explicit flags.",
        tags: &["glob", "file", "module"],
        source: r#"use File::Glob ':glob';
my @files = bsd_glob("*.txt", GLOB_NOCHECK);
"#,
    },
    GlobExpressionCase {
        id: "glob.file.glob.brace",
        description: "File::Glob with brace expansion enabled.",
        tags: &["glob", "file", "module", "brace-expansion"],
        source: r#"use File::Glob qw(:globally :nocase);
my @files = glob("{Foo,Bar}/*.pm");
"#,
    },
    GlobExpressionCase {
        id: "glob.for.loop",
        description: "Glob in for loop iteration.",
        tags: &["glob", "file", "loop"],
        source: r#"for my $file (glob("*.pl")) {
    print "processing $file\n";
}
"#,
    },
    GlobExpressionCase {
        id: "glob.map.grep",
        description: "Glob with map and grep filtering.",
        tags: &["glob", "file", "map", "grep"],
        source: r#"my @files = glob("*.pl");
my @filtered = grep { -f $_ } @files;
my @names = map { s/\.pl$//r } @filtered;
"#,
    },
    GlobExpressionCase {
        id: "glob.diamond.stdin",
        description: "Diamond operator reading from stdin or files.",
        tags: &["glob", "diamond", "io", "stdin"],
        source: r#"while (my $line = <>) {
    print $line;
}
"#,
    },
    GlobExpressionCase {
        id: "glob.diamond.filehandle",
        description: "Diamond operator with filehandle.",
        tags: &["glob", "diamond", "io", "filehandle"],
        source: r#"open my $fh, "<", "file.txt" or die $!;
while (my $line = <$fh>) {
    print $line;
}
"#,
    },
    GlobExpressionCase {
        id: "glob.diamond.data",
        description: "Diamond operator reading from DATA section.",
        tags: &["glob", "diamond", "io", "data"],
        source: r#"while (my $line = <DATA>) {
    print $line;
}
__DATA__
line 1
line 2
"#,
    },
    GlobExpressionCase {
        id: "glob.function.join",
        description: "Glob function with joined multiple patterns.",
        tags: &["glob", "file"],
        source: r#"my @files = glob("*.pl *.pm *.t");
"#,
    },
    GlobExpressionCase {
        id: "glob.hidden.files",
        description: "Glob pattern matching hidden files (dotfiles).",
        tags: &["glob", "file", "hidden"],
        source: r#"my @hidden = glob(".*");
my @config = glob(".config*");
"#,
    },
    GlobExpressionCase {
        id: "glob.extension.multiple",
        description: "Glob matching multiple file extensions.",
        tags: &["glob", "file", "brace-expansion"],
        source: r#"my @source = glob("*.{c,h,cpp,hpp}");
my @perl = glob("*.{pl,pm,pod,t}");
"#,
    },
    GlobExpressionCase {
        id: "glob.directory.wildcard",
        description: "Glob with wildcard in directory component.",
        tags: &["glob", "file", "directory", "wildcard"],
        source: r#"my @files = glob("lib/*/Config.pm");
my @all = glob("*/t/*.t");
"#,
    },
    GlobExpressionCase {
        id: "glob.range.expansion",
        description: "Glob with numeric range in brace expansion.",
        tags: &["glob", "file", "brace-expansion"],
        source: r#"my @files = glob("file{1..10}.txt");
"#,
    },
    GlobExpressionCase {
        id: "glob.assignment.scalar",
        description: "Glob assignment to scalar variable.",
        tags: &["glob", "file", "scalar-context"],
        source: r#"my $first = glob("*.txt");
print "$first\n" if defined $first;
"#,
    },
    GlobExpressionCase {
        id: "glob.sort.results",
        description: "Sorting glob results.",
        tags: &["glob", "file", "sort"],
        source: r#"my @files = sort glob("*.pl");
my @reversed = reverse sort glob("*.pm");
"#,
    },
    GlobExpressionCase {
        id: "glob.filetest.filter",
        description: "Glob with filetest operator filtering.",
        tags: &["glob", "file", "filetest"],
        source: r#"my @readable = grep { -r $_ } glob("*.txt");
my @writable = grep { -w $_ } glob("*.log");
my @executable = grep { -x $_ } glob("*.pl");
"#,
    },
    GlobExpressionCase {
        id: "glob.quote.meta",
        description: "Glob with quotemeta for literal special chars.",
        tags: &["glob", "file", "escape"],
        source: r#"my $literal = "file[1].txt";
my @files = glob(quotemeta($literal));
"#,
    },
    GlobExpressionCase {
        id: "glob.error.handling",
        description: "Glob with error handling and File::Glob constants.",
        tags: &["glob", "file", "error", "module"],
        source: r#"use File::Glob ':globally';
my @files = glob("*.txt");
if (GLOB_ERROR) {
    warn "Glob error occurred\n";
}
"#,
    },
];

/// Return the static glob expression fixtures.
pub fn glob_expression_cases() -> &'static [GlobExpressionCase] {
    GLOB_EXPRESSION_CASES
}

/// Find a glob expression fixture by ID.
pub fn find_glob_case(id: &str) -> Option<&'static GlobExpressionCase> {
    glob_expression_cases().iter().find(|case| case.id == id)
}

/// Convenience helper for working with glob expression fixtures.
pub struct GlobExpressionGenerator;

impl GlobExpressionGenerator {
    /// Return all available glob expression cases.
    pub fn all_cases() -> &'static [GlobExpressionCase] {
        glob_expression_cases()
    }

    /// Return glob expression cases with a matching tag.
    pub fn by_tag(tag: &str) -> Vec<&'static GlobExpressionCase> {
        glob_expression_cases().iter().filter(|case| case.tags.contains(&tag)).collect()
    }

    /// Return glob expression cases that match any of the provided tags.
    pub fn by_tags_any(tags: &[&str]) -> Vec<&'static GlobExpressionCase> {
        if tags.is_empty() {
            return glob_expression_cases().iter().collect();
        }

        glob_expression_cases()
            .iter()
            .filter(|case| case.tags.iter().any(|tag| tags.contains(tag)))
            .collect()
    }

    /// Return glob expression cases that match all of the provided tags.
    pub fn by_tags_all(tags: &[&str]) -> Vec<&'static GlobExpressionCase> {
        if tags.is_empty() {
            return glob_expression_cases().iter().collect();
        }

        glob_expression_cases()
            .iter()
            .filter(|case| tags.iter().all(|tag| case.tags.iter().any(|t| t == tag)))
            .collect()
    }

    /// Find a single glob expression case by ID.
    pub fn find(id: &str) -> Option<&'static GlobExpressionCase> {
        find_glob_case(id)
    }

    /// Sample a deterministic glob expression case by seed.
    pub fn sample(seed: u64) -> Option<&'static GlobExpressionCase> {
        let cases = glob_expression_cases();
        if cases.is_empty() {
            return None;
        }
        let idx = (seed % cases.len() as u64) as usize;
        cases.get(idx)
    }

    /// Sample a deterministic glob expression case by tag.
    pub fn sample_by_tag(tag: &str, seed: u64) -> Option<&'static GlobExpressionCase> {
        let matches = Self::by_tag(tag);
        if matches.is_empty() {
            return None;
        }
        let idx = (seed % matches.len() as u64) as usize;
        matches.get(idx).copied()
    }

    /// Return sorted unique glob expression tags.
    pub fn tags() -> Vec<&'static str> {
        let mut tags: Vec<&'static str> =
            glob_expression_cases().iter().flat_map(|case| case.tags.iter().copied()).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn glob_cases_have_ids() {
        assert!(glob_expression_cases().iter().all(|case| !case.id.is_empty()));
    }

    #[test]
    fn glob_cases_have_descriptions() {
        assert!(glob_expression_cases().iter().all(|case| !case.description.is_empty()));
    }

    #[test]
    fn glob_cases_have_source() {
        assert!(glob_expression_cases().iter().all(|case| !case.source.is_empty()));
    }

    #[test]
    fn glob_case_ids_are_unique() {
        let mut seen = HashSet::new();
        for case in glob_expression_cases() {
            assert!(seen.insert(case.id), "Duplicate glob case id: {}", case.id);
        }
    }

    #[test]
    fn glob_cases_can_filter_by_tag() {
        let diamond = GlobExpressionGenerator::by_tag("diamond");
        assert!(!diamond.is_empty());
        assert!(diamond.iter().all(|case| case.tags.contains(&"diamond")));
    }

    #[test]
    fn glob_cases_can_filter_by_any_tag() {
        let matches = GlobExpressionGenerator::by_tags_any(&["wildcard", "brace-expansion"]);
        assert!(!matches.is_empty());
    }

    #[test]
    fn glob_cases_can_filter_by_all_tags() {
        let matches = GlobExpressionGenerator::by_tags_all(&["glob", "file"]);
        assert!(!matches.is_empty());
        assert!(
            matches
                .iter()
                .all(|case| { case.tags.contains(&"glob") && case.tags.contains(&"file") })
        );
    }

    #[test]
    fn glob_case_lookup_by_id() {
        let case = find_glob_case("glob.function.basic");
        must_some(case);
        assert_eq!(must_some(case).id, "glob.function.basic");
    }

    #[test]
    fn glob_case_sample_is_stable() {
        let first = GlobExpressionGenerator::sample(42);
        let second = GlobExpressionGenerator::sample(42);
        assert_eq!(must_some(first).id, must_some(second).id);
    }

    #[test]
    fn glob_case_sample_by_tag_matches() {
        let case = GlobExpressionGenerator::sample_by_tag("diamond", 7);
        must_some(case);
        assert!(must_some(case).tags.contains(&"diamond"));
    }

    #[test]
    fn glob_case_tags_are_unique() {
        let tags = GlobExpressionGenerator::tags();
        let mut deduped = tags.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(tags, deduped);
    }

    #[test]
    fn glob_cases_cover_basic_patterns() {
        assert!(find_glob_case("glob.function.basic").is_some());
        assert!(find_glob_case("glob.diamond.basic").is_some());
        assert!(find_glob_case("glob.wildcard.question").is_some());
        assert!(find_glob_case("glob.wildcard.brackets").is_some());
        assert!(find_glob_case("glob.brace.expansion").is_some());
    }

    #[test]
    fn glob_cases_cover_advanced_patterns() {
        assert!(find_glob_case("glob.directory.pattern").is_some());
        assert!(find_glob_case("glob.variable.interpolation").is_some());
        assert!(find_glob_case("glob.list.context").is_some());
        assert!(find_glob_case("glob.scalar.context").is_some());
    }

    #[test]
    fn glob_generator_all_cases_returns_all() {
        assert_eq!(GlobExpressionGenerator::all_cases().len(), glob_expression_cases().len());
    }

    #[test]
    fn glob_generator_find_works() {
        let case = GlobExpressionGenerator::find("glob.diamond.variable");
        must_some(case);
        assert_eq!(must_some(case).id, "glob.diamond.variable");
    }
}
