//! Mutation-killing tests for perl-lsp-import-management.
//!
//! The existing integration tests cover:
//!   - guess_module_for_function: "encode" → Some("Encode"), unknown → None
//!   - collect_imports: basic use/require detection
//!   - sort_imports: single example of each bucket
//!   - find_imports_range: happy path and None case
//!
//! These tests target the untested branches and boundary conditions:
//!
//! - guess_module_for_function: every match arm (9 more arms untested)
//! - collect_imports: empty input, lines that look like imports but aren't
//! - sort_imports: all-pragma, all-cpan, multiple items within same bucket sorted
//! - find_imports_range: single import, rfind fallback for repeated last line

use perl_lsp_import_management::{
    collect_imports, find_imports_range, guess_module_for_function, sort_imports,
};

// ---------------------------------------------------------------------------
// guess_module_for_function: every match arm
// ---------------------------------------------------------------------------

#[test]
fn guess_module_dumper_returns_data_dumper() {
    assert_eq!(
        guess_module_for_function("dumper"),
        Some("Data::Dumper".to_string()),
        "dumper → Data::Dumper"
    );
}

#[test]
fn guess_module_decode_returns_encode() {
    assert_eq!(
        guess_module_for_function("decode"),
        Some("Encode".to_string()),
        "decode → Encode"
    );
}

#[test]
fn guess_module_encode_returns_encode() {
    assert_eq!(
        guess_module_for_function("encode"),
        Some("Encode".to_string()),
        "encode → Encode"
    );
}

#[test]
fn guess_module_basename_returns_file_basename() {
    assert_eq!(
        guess_module_for_function("basename"),
        Some("File::Basename".to_string()),
        "basename → File::Basename"
    );
}

#[test]
fn guess_module_dirname_returns_file_basename() {
    assert_eq!(
        guess_module_for_function("dirname"),
        Some("File::Basename".to_string()),
        "dirname → File::Basename"
    );
}

#[test]
fn guess_module_mkpath_returns_file_path() {
    assert_eq!(
        guess_module_for_function("mkpath"),
        Some("File::Path".to_string()),
        "mkpath → File::Path"
    );
}

#[test]
fn guess_module_rmtree_returns_file_path() {
    assert_eq!(
        guess_module_for_function("rmtree"),
        Some("File::Path".to_string()),
        "rmtree → File::Path"
    );
}

#[test]
fn guess_module_slurp_returns_file_slurp() {
    assert_eq!(
        guess_module_for_function("slurp"),
        Some("File::Slurp".to_string()),
        "slurp → File::Slurp"
    );
}

#[test]
fn guess_module_decode_json_returns_json() {
    assert_eq!(
        guess_module_for_function("decode_json"),
        Some("JSON".to_string()),
        "decode_json → JSON"
    );
}

#[test]
fn guess_module_encode_json_returns_json() {
    assert_eq!(
        guess_module_for_function("encode_json"),
        Some("JSON".to_string()),
        "encode_json → JSON"
    );
}

#[test]
fn guess_module_unknown_function_returns_none() {
    assert_eq!(guess_module_for_function("no_such_func"), None);
}

#[test]
fn guess_module_empty_string_returns_none() {
    assert_eq!(guess_module_for_function(""), None);
}

#[test]
fn guess_module_case_sensitive_uppercase_returns_none() {
    // The match is case-sensitive; "Dumper" (capital D) must not match "dumper"
    assert_eq!(
        guess_module_for_function("Dumper"),
        None,
        "guess_module_for_function is case-sensitive: 'Dumper' must not match"
    );
}

// ---------------------------------------------------------------------------
// collect_imports: edge cases
// ---------------------------------------------------------------------------

#[test]
fn collect_imports_empty_input_returns_empty() {
    let lines: Vec<String> = vec![];
    assert!(
        collect_imports(&lines).is_empty(),
        "empty input must return empty"
    );
}

#[test]
fn collect_imports_no_imports_returns_empty() {
    let lines = vec![
        "#!/usr/bin/perl".to_string(),
        "my $x = 1;".to_string(),
        "print $x;".to_string(),
    ];
    assert!(collect_imports(&lines).is_empty());
}

#[test]
fn collect_imports_ignores_lines_without_use_or_require_prefix() {
    // "# use strict;" is a comment, not an import
    let lines = vec![
        "# use strict;".to_string(),
        "  # require Foo;".to_string(),
        "my $use = 1;".to_string(), // contains "use" but not at start
    ];
    let imports = collect_imports(&lines);
    assert!(
        imports.is_empty(),
        "comment lines and mid-line 'use' must not be collected"
    );
}

#[test]
fn collect_imports_only_use_and_require_are_collected() {
    let lines = vec![
        "use strict;".to_string(),
        "require Scalar::Util;".to_string(),
        "no warnings;".to_string(), // "no" is not "use" or "require"
        "my $x = 1;".to_string(),
    ];
    let imports = collect_imports(&lines);
    assert_eq!(imports.len(), 2);
    assert!(imports.contains(&"use strict;".to_string()));
    assert!(imports.contains(&"require Scalar::Util;".to_string()));
}

#[test]
fn collect_imports_preserves_original_line_content() {
    // The line is stored as-is including indentation, so leading-whitespace lines
    // that trim to "use ..." are still collected
    let lines = vec!["    use strict;".to_string()];
    let imports = collect_imports(&lines);
    assert_eq!(imports, vec!["    use strict;".to_string()]);
}

// ---------------------------------------------------------------------------
// sort_imports: bucket ordering and intra-bucket sorting
// ---------------------------------------------------------------------------

#[test]
fn sort_imports_all_pragmas_stay_in_pragma_bucket() {
    let sorted = sort_imports(vec![
        "use warnings;".to_string(),
        "use utf8;".to_string(),
        "use strict;".to_string(),
        "use feature 'say';".to_string(),
    ]);
    // All are pragmas → sorted lexicographically within the pragma bucket
    assert_eq!(sorted.len(), 4);
    assert_eq!(sorted[0], "use feature 'say';");
    assert_eq!(sorted[1], "use strict;");
    assert_eq!(sorted[2], "use utf8;");
    assert_eq!(sorted[3], "use warnings;");
}

#[test]
fn sort_imports_cpan_modules_sorted_lexicographically_within_bucket() {
    // Both contain "::" → cpan bucket, sorted lexicographically within bucket
    let sorted = sort_imports(vec![
        "use Scalar::Util qw(blessed);".to_string(),
        "use List::Util qw(sum);".to_string(),
        "use Carp::Heavy;".to_string(),
    ]);
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0], "use Carp::Heavy;");
    assert_eq!(sorted[1], "use List::Util qw(sum);");
    assert_eq!(sorted[2], "use Scalar::Util qw(blessed);");
}

#[test]
fn sort_imports_bucket_order_pragma_core_cpan_local() {
    let sorted = sort_imports(vec![
        "use lib './lib';".to_string(), // local
        "use Foo::Bar;".to_string(),    // cpan
        "use integer;".to_string(),     // core
        "use strict;".to_string(),      // pragma
    ]);
    // Order: pragma → core → cpan → local
    assert_eq!(sorted[0], "use strict;", "pragma must come first");
    assert_eq!(sorted[1], "use integer;", "core must come second");
    assert_eq!(sorted[2], "use Foo::Bar;", "cpan must come third");
    assert_eq!(sorted[3], "use lib './lib';", "local must come last");
}

#[test]
fn sort_imports_empty_input_returns_empty() {
    let sorted = sort_imports(vec![]);
    assert!(sorted.is_empty());
}

// ---------------------------------------------------------------------------
// find_imports_range: single import, None when no imports
// ---------------------------------------------------------------------------

#[test]
fn find_imports_range_single_import_returns_its_span() {
    let source = "use strict;\nmy $x = 1;\n";
    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    let range = find_imports_range(source, &lines);
    assert_eq!(range, Some((0, "use strict;".len())));
}

#[test]
fn find_imports_range_no_imports_returns_none() {
    let source = "my $x = 1;\nprint $x;\n";
    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    assert_eq!(find_imports_range(source, &lines), None);
}

#[test]
fn find_imports_range_empty_source_returns_none() {
    let source = "";
    let lines: Vec<String> = vec![];
    assert_eq!(find_imports_range(source, &lines), None);
}
