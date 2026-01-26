//! Tie/untie interface corpus - comprehensive test fixtures for Perl's tie mechanism.

/// A tie interface test case with metadata.
#[derive(Debug, Clone, Copy)]
pub struct TieInterfaceCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Tags for filtering and grouping.
    pub tags: &'static [&'static str],
    /// Perl source for the test case.
    pub source: &'static str,
}

static TIE_INTERFACE_CASES: &[TieInterfaceCase] = &[
    // Basic tie operations for each variable type
    TieInterfaceCase {
        id: "tie.scalar.basic",
        description: "Basic scalar tie with class name.",
        tags: &["tie", "scalar", "basic"],
        source: r#"tie $var, 'Tie::Scalar';
"#,
    },
    TieInterfaceCase {
        id: "tie.scalar.my",
        description: "Tie scalar with my declaration.",
        tags: &["tie", "scalar", "declaration"],
        source: r#"tie my $var, 'Tie::Scalar';
"#,
    },
    TieInterfaceCase {
        id: "tie.array.basic",
        description: "Basic array tie with class name.",
        tags: &["tie", "array", "basic"],
        source: r#"tie @arr, 'Tie::Array';
"#,
    },
    TieInterfaceCase {
        id: "tie.array.my",
        description: "Tie array with my declaration.",
        tags: &["tie", "array", "declaration"],
        source: r#"tie my @arr, 'Tie::Array';
"#,
    },
    TieInterfaceCase {
        id: "tie.hash.basic",
        description: "Basic hash tie with class name.",
        tags: &["tie", "hash", "basic"],
        source: r#"tie %hash, 'Tie::Hash';
"#,
    },
    TieInterfaceCase {
        id: "tie.hash.my",
        description: "Tie hash with my declaration.",
        tags: &["tie", "hash", "declaration"],
        source: r#"tie my %hash, 'Tie::Hash';
"#,
    },
    TieInterfaceCase {
        id: "tie.filehandle.basic",
        description: "Basic filehandle tie with class name.",
        tags: &["tie", "filehandle", "basic"],
        source: r#"tie *FH, 'Tie::Handle';
"#,
    },
    TieInterfaceCase {
        id: "tie.filehandle.my",
        description: "Tie filehandle with my declaration.",
        tags: &["tie", "filehandle", "declaration"],
        source: r#"tie my *FH, 'Tie::Handle';
"#,
    },
    // Tie with arguments
    TieInterfaceCase {
        id: "tie.scalar.args",
        description: "Tie scalar with constructor arguments.",
        tags: &["tie", "scalar", "arguments"],
        source: r#"tie my $counter, 'Tie::Counter', initial => 0, step => 1;
"#,
    },
    TieInterfaceCase {
        id: "tie.array.args",
        description: "Tie array with constructor arguments.",
        tags: &["tie", "array", "arguments"],
        source: r#"tie my @queue, 'Tie::Array', max_size => 100;
"#,
    },
    TieInterfaceCase {
        id: "tie.hash.args.dbfile",
        description: "Tie hash with DB_File and file arguments.",
        tags: &["tie", "hash", "arguments", "db-file"],
        source: r#"tie my %cache, 'DB_File', 'cache.db', O_RDWR|O_CREAT, 0644;
"#,
    },
    TieInterfaceCase {
        id: "tie.hash.args.multiple",
        description: "Tie hash with multiple named arguments.",
        tags: &["tie", "hash", "arguments"],
        source: r#"tie my %config, 'Tie::IxHash', key1 => 'val1', key2 => 'val2';
"#,
    },
    TieInterfaceCase {
        id: "tie.filehandle.args",
        description: "Tie filehandle with file path argument.",
        tags: &["tie", "filehandle", "arguments"],
        source: r#"tie *LOG, 'Tie::FileHandle', file => '/var/log/app.log';
"#,
    },
    // Untie operations
    TieInterfaceCase {
        id: "untie.scalar",
        description: "Untie a scalar variable.",
        tags: &["untie", "scalar"],
        source: r#"tie my $var, 'Tie::Scalar';
untie $var;
"#,
    },
    TieInterfaceCase {
        id: "untie.array",
        description: "Untie an array variable.",
        tags: &["untie", "array"],
        source: r#"tie my @arr, 'Tie::Array';
untie @arr;
"#,
    },
    TieInterfaceCase {
        id: "untie.hash",
        description: "Untie a hash variable.",
        tags: &["untie", "hash"],
        source: r#"tie my %hash, 'Tie::Hash';
untie %hash;
"#,
    },
    TieInterfaceCase {
        id: "untie.filehandle",
        description: "Untie a filehandle.",
        tags: &["untie", "filehandle"],
        source: r#"tie *FH, 'Tie::Handle';
untie *FH;
"#,
    },
    // tied() function
    TieInterfaceCase {
        id: "tied.scalar.check",
        description: "Check if scalar is tied using tied().",
        tags: &["tied", "scalar", "check"],
        source: r#"tie my $var, 'Tie::Scalar';
my $obj = tied $var;
"#,
    },
    TieInterfaceCase {
        id: "tied.array.check",
        description: "Check if array is tied using tied().",
        tags: &["tied", "array", "check"],
        source: r#"tie my @arr, 'Tie::Array';
my $obj = tied @arr;
"#,
    },
    TieInterfaceCase {
        id: "tied.hash.check",
        description: "Check if hash is tied using tied().",
        tags: &["tied", "hash", "check"],
        source: r#"tie my %hash, 'Tie::Hash';
my $obj = tied %hash;
"#,
    },
    TieInterfaceCase {
        id: "tied.filehandle.check",
        description: "Check if filehandle is tied using tied().",
        tags: &["tied", "filehandle", "check"],
        source: r#"tie *FH, 'Tie::Handle';
my $obj = tied *FH;
"#,
    },
    TieInterfaceCase {
        id: "tied.conditional",
        description: "Use tied() in conditional to check tie status.",
        tags: &["tied", "conditional"],
        source: r#"tie my %cache, 'Tie::StdHash';
if (tied %cache) {
    print "Hash is tied\n";
}
"#,
    },
    // Tie with return value
    TieInterfaceCase {
        id: "tie.return.scalar",
        description: "Capture tie return value for scalar.",
        tags: &["tie", "scalar", "return-value"],
        source: r#"my $obj = tie my $var, 'Tie::Scalar';
"#,
    },
    TieInterfaceCase {
        id: "tie.return.array",
        description: "Capture tie return value for array.",
        tags: &["tie", "array", "return-value"],
        source: r#"my $obj = tie my @arr, 'Tie::Array';
"#,
    },
    TieInterfaceCase {
        id: "tie.return.hash",
        description: "Capture tie return value for hash.",
        tags: &["tie", "hash", "return-value"],
        source: r#"my $obj = tie my %hash, 'Tie::StdHash';
$hash{key} = 'value';
"#,
    },
    TieInterfaceCase {
        id: "tie.return.filehandle",
        description: "Capture tie return value for filehandle.",
        tags: &["tie", "filehandle", "return-value"],
        source: r#"my $obj = tie *FH, 'Tie::Handle';
"#,
    },
    // Complex tie scenarios
    TieInterfaceCase {
        id: "tie.hash.usage",
        description: "Tie hash with usage pattern.",
        tags: &["tie", "hash", "usage"],
        source: r#"tie my %cache, 'Tie::StdHash';
$cache{foo} = 'bar';
my $value = $cache{foo};
delete $cache{foo};
untie %cache;
"#,
    },
    TieInterfaceCase {
        id: "tie.array.usage",
        description: "Tie array with usage pattern.",
        tags: &["tie", "array", "usage"],
        source: r#"tie my @queue, 'Tie::Array';
push @queue, 'item1';
push @queue, 'item2';
my $first = shift @queue;
untie @queue;
"#,
    },
    TieInterfaceCase {
        id: "tie.scalar.usage",
        description: "Tie scalar with usage pattern.",
        tags: &["tie", "scalar", "usage"],
        source: r#"tie my $counter, 'Tie::Counter';
$counter++;
my $value = $counter;
untie $counter;
"#,
    },
    TieInterfaceCase {
        id: "tie.multiple.vars",
        description: "Tie multiple variables with different classes.",
        tags: &["tie", "multiple", "complex"],
        source: r#"tie my $scalar, 'Tie::Scalar';
tie my @array, 'Tie::Array';
tie my %hash, 'Tie::StdHash';
$scalar = 1;
push @array, 'item';
$hash{key} = 'value';
"#,
    },
    TieInterfaceCase {
        id: "tie.nested.access",
        description: "Tie with nested data structure access.",
        tags: &["tie", "hash", "nested"],
        source: r#"tie my %data, 'Tie::StdHash';
$data{users} = [];
push @{$data{users}}, { id => 1, name => 'Alice' };
"#,
    },
    TieInterfaceCase {
        id: "tie.method.call",
        description: "Call methods on tied object via tied().",
        tags: &["tie", "tied", "method"],
        source: r#"tie my %cache, 'Cache::Tie', size => 100;
my $obj = tied %cache;
$obj->clear();
"#,
    },
    TieInterfaceCase {
        id: "tie.stdmodules.stdhash",
        description: "Use Tie::StdHash standard module.",
        tags: &["tie", "hash", "std-module"],
        source: r#"use Tie::Hash;
tie my %hash, 'Tie::StdHash';
$hash{a} = 1;
$hash{b} = 2;
my @keys = keys %hash;
untie %hash;
"#,
    },
    TieInterfaceCase {
        id: "tie.stdmodules.stdarray",
        description: "Use Tie::StdArray standard module.",
        tags: &["tie", "array", "std-module"],
        source: r#"use Tie::Array;
tie my @array, 'Tie::StdArray';
$array[0] = 'first';
$array[1] = 'second';
my $len = scalar @array;
untie @array;
"#,
    },
    TieInterfaceCase {
        id: "tie.stdmodules.stdscalar",
        description: "Use Tie::StdScalar standard module.",
        tags: &["tie", "scalar", "std-module"],
        source: r#"use Tie::Scalar;
tie my $var, 'Tie::StdScalar';
$var = 42;
my $value = $var;
untie $var;
"#,
    },
    TieInterfaceCase {
        id: "tie.stdmodules.stdhandle",
        description: "Use Tie::StdHandle standard module.",
        tags: &["tie", "filehandle", "std-module"],
        source: r#"use Tie::Handle;
tie *FH, 'Tie::StdHandle';
print FH "output\n";
untie *FH;
"#,
    },
    TieInterfaceCase {
        id: "tie.dbfile.complete",
        description: "Complete DB_File tie example with operations.",
        tags: &["tie", "hash", "db-file", "complete"],
        source: r#"use DB_File;
tie my %db, 'DB_File', 'data.db', O_RDWR|O_CREAT, 0644
    or die "Cannot open data.db: $!";
$db{key1} = 'value1';
$db{key2} = 'value2';
my $val = $db{key1};
delete $db{key2};
untie %db;
"#,
    },
    TieInterfaceCase {
        id: "tie.ixhash.ordered",
        description: "Use Tie::IxHash for ordered hash.",
        tags: &["tie", "hash", "ordered", "ixhash"],
        source: r#"use Tie::IxHash;
tie my %ordered, 'Tie::IxHash';
$ordered{first} = 1;
$ordered{second} = 2;
$ordered{third} = 3;
my @keys = keys %ordered;
"#,
    },
    TieInterfaceCase {
        id: "tie.file.complete",
        description: "Use Tie::File for line-based file access.",
        tags: &["tie", "array", "file-access"],
        source: r#"use Tie::File;
tie my @lines, 'Tie::File', 'file.txt'
    or die "Cannot tie file.txt: $!";
$lines[0] = 'First line';
$lines[1] = 'Second line';
my $count = scalar @lines;
untie @lines;
"#,
    },
    TieInterfaceCase {
        id: "tie.memoize",
        description: "Use Tie::Memoize for function result caching.",
        tags: &["tie", "scalar", "memoize"],
        source: r#"use Tie::Memoize;
tie my $result, 'Tie::Memoize', \&expensive_function;
my $value = $result;
"#,
    },
    TieInterfaceCase {
        id: "tie.refhash",
        description: "Use Tie::RefHash for reference-keyed hash.",
        tags: &["tie", "hash", "refhash"],
        source: r#"use Tie::RefHash;
tie my %refhash, 'Tie::RefHash';
my $obj = bless {}, 'Object';
$refhash{$obj} = 'value';
my $val = $refhash{$obj};
"#,
    },
    TieInterfaceCase {
        id: "tie.eval.block",
        description: "Tie operation within eval block for error handling.",
        tags: &["tie", "eval", "error-handling"],
        source: r#"eval {
    tie my %db, 'DB_File', 'data.db', O_RDWR|O_CREAT, 0644;
    $db{key} = 'value';
    untie %db;
};
warn "Tie failed: $@" if $@;
"#,
    },
    TieInterfaceCase {
        id: "tie.retie.sequence",
        description: "Untie and retie sequence.",
        tags: &["tie", "untie", "sequence"],
        source: r#"tie my %cache, 'Tie::StdHash';
$cache{a} = 1;
untie %cache;
tie %cache, 'Tie::StdHash';
$cache{b} = 2;
untie %cache;
"#,
    },
    TieInterfaceCase {
        id: "tie.package.qualified",
        description: "Tie with package-qualified class name.",
        tags: &["tie", "package-qualified"],
        source: r#"tie my %hash, 'My::Custom::Tie::Hash', option => 'value';
"#,
    },
    TieInterfaceCase {
        id: "tie.bareword.filehandle",
        description: "Tie bareword filehandle.",
        tags: &["tie", "filehandle", "bareword"],
        source: r#"tie *LOG, 'Tie::Handle';
print LOG "message\n";
untie *LOG;
"#,
    },
    TieInterfaceCase {
        id: "tie.conditional.untie",
        description: "Conditional untie based on tied status.",
        tags: &["tie", "untie", "conditional"],
        source: r#"tie my %cache, 'Tie::StdHash';
$cache{foo} = 'bar';
untie %cache if tied %cache;
"#,
    },
    TieInterfaceCase {
        id: "tie.loop.iteration",
        description: "Iterate over tied hash.",
        tags: &["tie", "hash", "iteration"],
        source: r#"tie my %config, 'Tie::StdHash';
$config{key1} = 'val1';
$config{key2} = 'val2';
while (my ($key, $value) = each %config) {
    print "$key => $value\n";
}
untie %config;
"#,
    },
    TieInterfaceCase {
        id: "tie.exists.delete",
        description: "Use exists and delete with tied hash.",
        tags: &["tie", "hash", "exists", "delete"],
        source: r#"tie my %data, 'Tie::StdHash';
$data{key} = 'value';
if (exists $data{key}) {
    delete $data{key};
}
untie %data;
"#,
    },
    TieInterfaceCase {
        id: "tie.array.slice",
        description: "Array slice operations on tied array.",
        tags: &["tie", "array", "slice"],
        source: r#"tie my @array, 'Tie::StdArray';
@array = (1, 2, 3, 4, 5);
my @slice = @array[1, 3];
untie @array;
"#,
    },
    TieInterfaceCase {
        id: "tie.hash.slice",
        description: "Hash slice operations on tied hash.",
        tags: &["tie", "hash", "slice"],
        source: r#"tie my %hash, 'Tie::StdHash';
%hash = (a => 1, b => 2, c => 3);
my @values = @hash{qw(a c)};
untie %hash;
"#,
    },
    TieInterfaceCase {
        id: "tie.scope.local",
        description: "Tie with local scope and automatic cleanup.",
        tags: &["tie", "scope", "local"],
        source: r#"{
    tie my %cache, 'Tie::StdHash';
    $cache{temp} = 'value';
}
"#,
    },
    TieInterfaceCase {
        id: "tie.our.variable",
        description: "Tie our-scoped package variable.",
        tags: &["tie", "our", "package"],
        source: r#"our %global;
tie %global, 'Tie::StdHash';
$global{key} = 'value';
"#,
    },
];

/// Return all tie interface test cases.
pub fn tie_interface_cases() -> &'static [TieInterfaceCase] {
    TIE_INTERFACE_CASES
}

/// Find a tie interface case by ID.
pub fn find_tie_case(id: &str) -> Option<&'static TieInterfaceCase> {
    tie_interface_cases().iter().find(|case| case.id == id)
}

/// Find tie interface cases by tag.
pub fn tie_cases_by_tag(tag: &str) -> Vec<&'static TieInterfaceCase> {
    tie_interface_cases().iter().filter(|case| case.tags.contains(&tag)).collect()
}

/// Find tie interface cases matching any of the provided tags.
pub fn tie_cases_by_tags_any(tags: &[&str]) -> Vec<&'static TieInterfaceCase> {
    if tags.is_empty() {
        return tie_interface_cases().iter().collect();
    }
    tie_interface_cases()
        .iter()
        .filter(|case| case.tags.iter().any(|tag| tags.contains(tag)))
        .collect()
}

/// Find tie interface cases matching all of the provided tags.
pub fn tie_cases_by_tags_all(tags: &[&str]) -> Vec<&'static TieInterfaceCase> {
    if tags.is_empty() {
        return tie_interface_cases().iter().collect();
    }
    tie_interface_cases()
        .iter()
        .filter(|case| tags.iter().all(|tag| case.tags.iter().any(|t| t == tag)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;
    use std::collections::HashSet;

    #[test]
    fn tie_cases_have_ids() {
        assert!(tie_interface_cases().iter().all(|case| !case.id.is_empty()));
    }

    #[test]
    fn tie_cases_have_descriptions() {
        assert!(tie_interface_cases().iter().all(|case| !case.description.is_empty()));
    }

    #[test]
    fn tie_cases_have_source() {
        assert!(tie_interface_cases().iter().all(|case| !case.source.is_empty()));
    }

    #[test]
    fn tie_case_ids_are_unique() {
        let mut seen = HashSet::new();
        for case in tie_interface_cases() {
            assert!(seen.insert(case.id), "Duplicate tie case id: {}", case.id);
        }
    }

    #[test]
    fn tie_cases_contain_tie_keyword() {
        let tie_cases = tie_interface_cases()
            .iter()
            .filter(|case| !case.tags.contains(&"untie") || case.tags.contains(&"tie"))
            .collect::<Vec<_>>();
        assert!(tie_cases.iter().all(|case| case.source.contains("tie")));
    }

    #[test]
    fn tie_case_find_by_id() {
        let case = find_tie_case("tie.scalar.basic");
        assert!(case.is_some());
        assert_eq!(must_some(case).id, "tie.scalar.basic");
    }

    #[test]
    fn tie_cases_filter_by_tag() {
        let scalar_cases = tie_cases_by_tag("scalar");
        assert!(!scalar_cases.is_empty());
        assert!(scalar_cases.iter().all(|case| case.tags.contains(&"scalar")));
    }

    #[test]
    fn tie_cases_filter_by_tags_any() {
        let cases = tie_cases_by_tags_any(&["scalar", "array"]);
        assert!(!cases.is_empty());
        assert!(
            cases.iter().all(|case| case.tags.contains(&"scalar") || case.tags.contains(&"array"))
        );
    }

    #[test]
    fn tie_cases_filter_by_tags_all() {
        let cases = tie_cases_by_tags_all(&["tie", "scalar"]);
        assert!(!cases.is_empty());
        assert!(
            cases.iter().all(|case| case.tags.contains(&"tie") && case.tags.contains(&"scalar"))
        );
    }

    #[test]
    fn tie_cases_cover_all_variable_types() {
        let scalar_cases = tie_cases_by_tag("scalar");
        let array_cases = tie_cases_by_tag("array");
        let hash_cases = tie_cases_by_tag("hash");
        let filehandle_cases = tie_cases_by_tag("filehandle");

        assert!(!scalar_cases.is_empty(), "Should have scalar tie cases");
        assert!(!array_cases.is_empty(), "Should have array tie cases");
        assert!(!hash_cases.is_empty(), "Should have hash tie cases");
        assert!(!filehandle_cases.is_empty(), "Should have filehandle tie cases");
    }

    #[test]
    fn tie_cases_cover_untie() {
        let untie_cases = tie_cases_by_tag("untie");
        assert!(!untie_cases.is_empty(), "Should have untie cases");
        assert!(untie_cases.iter().all(|case| case.source.contains("untie")));
    }

    #[test]
    fn tie_cases_cover_tied_function() {
        let tied_cases = tie_cases_by_tag("tied");
        assert!(!tied_cases.is_empty(), "Should have tied() function cases");
        assert!(tied_cases.iter().all(|case| case.source.contains("tied")));
    }

    #[test]
    fn tie_cases_cover_arguments() {
        let arg_cases = tie_cases_by_tag("arguments");
        assert!(!arg_cases.is_empty(), "Should have tie cases with arguments");
    }

    #[test]
    fn tie_cases_cover_std_modules() {
        let std_cases = tie_cases_by_tag("std-module");
        assert!(!std_cases.is_empty(), "Should have standard module tie cases");
    }

    #[test]
    fn tie_case_count_is_sufficient() {
        let total = tie_interface_cases().len();
        assert!(
            total >= 40,
            "Should have comprehensive coverage with at least 40 test cases, got {}",
            total
        );
    }
}
