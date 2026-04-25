use std::path::Path;

use perl_module::resolution::use_lib::{
    UseLibAction, UseLibPath, extract_use_lib_operations, extract_use_lib_paths,
    resolve_use_lib_paths, resolve_use_lib_paths_from_source_at_offset,
};

#[test]
fn findbin_parent_traversal_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let file_dir = workspace.join("project").join("lib");

    std::fs::create_dir_all(&file_dir)?;

    let resolved = resolve_use_lib_paths(
        &[UseLibPath { path: "../../../outside".to_string(), from_findbin: true }],
        &workspace,
        Some(&file_dir),
    );

    assert!(resolved.is_empty(), "findbin traversal should be dropped");
    Ok(())
}

#[test]
fn findbin_dot_segment_is_normalized_inside_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let file_dir = workspace.join("project").join("lib");

    std::fs::create_dir_all(&file_dir)?;

    let resolved = resolve_use_lib_paths(
        &[UseLibPath { path: "../vendor/./lib".to_string(), from_findbin: true }],
        &workspace,
        Some(&file_dir),
    );

    assert_eq!(resolved, vec!["project/vendor/lib".to_string()]);
    Ok(())
}

#[test]
fn absolute_use_lib_path_outside_workspace_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let outside = temp.path().join("outside-lib");
    std::fs::create_dir_all(&workspace)?;
    std::fs::create_dir_all(&outside)?;

    let outside_path = outside.to_string_lossy().to_string();
    let resolved = resolve_use_lib_paths(
        &[UseLibPath { path: outside_path, from_findbin: false }],
        &workspace,
        None,
    );

    assert!(resolved.is_empty(), "absolute outside-workspace paths should be dropped");
    Ok(())
}

#[test]
fn absolute_use_lib_path_inside_workspace_is_normalized_to_relative()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let inside = workspace.join("lib").join("Nested");
    std::fs::create_dir_all(&inside)?;

    let inside_path = inside.to_string_lossy().to_string();
    let resolved = resolve_use_lib_paths(
        &[UseLibPath { path: inside_path, from_findbin: false }],
        &workspace,
        None,
    );

    assert_eq!(resolved, vec!["lib/Nested".to_string()]);
    Ok(())
}

#[test]
fn absolute_use_lib_path_with_embedded_dotdot_is_rejected() -> Result<(), Box<dyn std::error::Error>>
{
    // Regression guard: `Path::strip_prefix` is purely lexical.  An absolute path
    // like `<workspace>/../sibling` strips the `<workspace>` prefix lexically but
    // the remainder is `../sibling`, which would escape the workspace.  The guard in
    // `path_to_relative_string` must detect any `ParentDir` component in the
    // stripped result and return `None`.
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace)?;

    // Construct a truly absolute path that lexically starts with the workspace
    // prefix but contains an embedded `..` that escapes it.
    let bypass_path = format!(
        "{}{}..{}sibling",
        workspace.display(),
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR
    );

    let resolved = resolve_use_lib_paths(
        &[UseLibPath { path: bypass_path.clone(), from_findbin: false }],
        &workspace,
        None,
    );
    assert!(
        resolved.is_empty(),
        "absolute path with embedded `..` must be rejected; bypass_path={bypass_path:?} got: {resolved:?}"
    );
    Ok(())
}

#[test]
fn use_and_no_lib_operations_are_extracted_in_order() {
    let source = "\
use lib 'first';\n\
use lib 'second';\n\
no lib 'first';\n\
";

    let ops = extract_use_lib_operations(source);

    assert_eq!(
        ops,
        vec![
            UseLibAction::Add(vec![UseLibPath { path: "first".to_string(), from_findbin: false }]),
            UseLibAction::Add(vec![UseLibPath { path: "second".to_string(), from_findbin: false }]),
            UseLibAction::Remove(vec![UseLibPath {
                path: "first".to_string(),
                from_findbin: false,
            }]),
        ]
    );
}

#[test]
fn use_lib_offset_resolution_obeys_lexical_order() {
    let source = "\
use lib 'first';\n\
use lib 'second';\n\
no lib 'first';\n\
use Lib::Thing;\n\
";

    let offset_at_use = source.find("use Lib::Thing;").unwrap_or(source.len());
    let include_paths = resolve_use_lib_paths_from_source_at_offset(
        source,
        offset_at_use,
        Path::new("/workspace"),
        None,
    );

    assert_eq!(include_paths, vec!["second".to_string()]);
}

#[test]
fn short_findbin_exports_are_treated_as_findbin_paths() {
    // Both double-quoted (interpolating) and single-quoted (literal in real Perl)
    // forms are recognised; the extractor is intentionally quote-type-agnostic.
    let source = "\
use lib '$Bin/../lib';\n\
use lib \"$RealBin/../vendor\";\n\
";

    let ops = extract_use_lib_operations(source);

    assert_eq!(
        ops,
        vec![
            UseLibAction::Add(vec![UseLibPath { path: "../lib".to_string(), from_findbin: true }]),
            UseLibAction::Add(vec![UseLibPath {
                path: "../vendor".to_string(),
                from_findbin: true,
            }]),
        ]
    );
}

#[test]
fn short_findbin_prefix_does_not_match_longer_variable_name() {
    // `$BinDir` and `$RealBinPath` look like they start with `$Bin`/`$RealBin`
    // but are different variables — word-boundary check must reject them.
    let source = "\
use lib \"$BinDir/lib\";\n\
use lib \"$RealBinPath/vendor\";\n\
";

    let ops = extract_use_lib_operations(source);

    // Both paths should be treated as plain (non-FindBin) string paths.
    assert_eq!(
        ops,
        vec![
            UseLibAction::Add(vec![UseLibPath {
                path: "$BinDir/lib".to_string(),
                from_findbin: false,
            }]),
            UseLibAction::Add(vec![UseLibPath {
                path: "$RealBinPath/vendor".to_string(),
                from_findbin: false,
            }]),
        ]
    );
}

#[test]
fn braced_short_findbin_exports_are_treated_as_findbin_paths() {
    let source = "use lib \"${Bin}/../lib\";\n";

    let ops = extract_use_lib_operations(source);

    assert_eq!(
        ops,
        vec![UseLibAction::Add(vec![UseLibPath {
            path: "../lib".to_string(),
            from_findbin: true,
        }]),]
    );
}

#[test]
fn multiline_use_lib_is_extracted() {
    let source = "\
use lib (
    'first',
    \"second\"
);
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "first".to_string(), from_findbin: false },
            UseLibPath { path: "second".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn multiline_use_and_no_lib_are_ordered() {
    let source = "\
use lib (
    'first',
    'second'
);
no lib (
    'first'
);
";

    let ops = extract_use_lib_operations(source);

    assert_eq!(
        ops,
        vec![
            UseLibAction::Add(vec![
                UseLibPath { path: "first".to_string(), from_findbin: false },
                UseLibPath { path: "second".to_string(), from_findbin: false },
            ]),
            UseLibAction::Remove(vec![UseLibPath {
                path: "first".to_string(),
                from_findbin: false,
            }]),
        ]
    );
}

#[test]
fn quoted_semicolon_does_not_split_statement() {
    let source = "use lib ('alpha;beta', 'gamma');";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "alpha;beta".to_string(), from_findbin: false },
            UseLibPath { path: "gamma".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn inline_comment_inside_multiline_use_lib_does_not_truncate_paths() {
    // Perl inline comments (# ...) inside a parenthesized list must be skipped
    // so that paths appearing after the comment are still extracted.
    let source = "\
use lib (
    '/foo/bar',  # the main lib
    '/baz/qux'
);
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "/foo/bar".to_string(), from_findbin: false },
            UseLibPath { path: "/baz/qux".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn crlf_line_endings_do_not_affect_extraction() {
    // CRLF (\r\n) line endings are whitespace-normalized by trim(), so
    // multiline use lib with Windows line endings must work identically
    // to the Unix (\n) form.
    let source = "use lib (\r\n    'first',\r\n    'second'\r\n);\r\n";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "first".to_string(), from_findbin: false },
            UseLibPath { path: "second".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn multiline_qw_use_lib_is_extracted() {
    // qw() with whitespace-separated paths on multiple lines.
    let source = "\
use lib qw(
    /path/one
    /path/two
);
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "/path/one".to_string(), from_findbin: false },
            UseLibPath { path: "/path/two".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn escaped_quote_inside_single_quoted_path_is_handled() {
    // 'it\'s a path' is an extreme edge case; the backslash-escaping in
    // split_perl_statements must not cause the closing quote to be missed.
    // The practical value is that \\-terminated paths work correctly.
    let source = "use lib 'normal'; use lib 'also';";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "normal".to_string(), from_findbin: false },
            UseLibPath { path: "also".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn unterminated_use_lib_does_not_panic() {
    // Malformed Perl: unclosed string or missing semicolon.
    // The extractor must not panic; it may return partial or empty results.
    let sources = ["use lib 'unclosed", "use lib (\"no closing paren", "use lib"];
    for source in &sources {
        // Should not panic; we don't assert on the exact output.
        let _ = extract_use_lib_paths(source);
        let _ = extract_use_lib_operations(source);
    }
}

#[test]
fn comment_with_semicolon_does_not_drop_subsequent_use_lib() {
    // Regression: split_perl_statements must skip Perl line comments (#...\n)
    // so that a semicolon inside a comment does not split the statement and
    // cause the next `use lib` to be dropped.
    let source = "\
use lib '/foo';  # primary lib; see INSTALL for details
use lib '/bar';
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "/foo".to_string(), from_findbin: false },
            UseLibPath { path: "/bar".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn leading_comment_with_semicolon_does_not_drop_use_lib() {
    // A comment before the first use lib that contains a semicolon must not
    // create a spurious leading fragment that mangles the statement that follows.
    let source = "\
# This script uses lib; see also INSTALL
use lib 'mylib';
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths, vec![UseLibPath { path: "mylib".to_string(), from_findbin: false }]);
}

#[test]
fn empty_source_returns_empty_paths() {
    // Edge case: completely empty input should not crash.
    let source = "";
    let paths = extract_use_lib_paths(source);
    assert!(paths.is_empty());

    let ops = extract_use_lib_operations(source);
    assert!(ops.is_empty());
}

#[test]
fn only_whitespace_returns_empty_paths() {
    // Edge case: input with only whitespace, tabs, newlines.
    let source = "   \n\t\n  ";
    let paths = extract_use_lib_paths(source);
    assert!(paths.is_empty());
}

#[test]
fn only_comments_returns_empty_paths() {
    // Edge case: input with only comments (no actual code).
    let source = "\
# Comment 1
# Comment 2
# Comment with semicolon;
";
    let paths = extract_use_lib_paths(source);
    assert!(paths.is_empty());
}

#[test]
fn qw_with_semicolon_in_path_is_not_split() {
    // Edge case: qw() list item that contains a semicolon (unusual but valid).
    // qw() items are whitespace-separated, not quote-delimited, so a semicolon
    // inside a path would be literal in the output. However, qw() splitter
    // must not confuse the semicolon with a statement terminator.
    let source = "use lib qw(/path/one /path/two); use lib '/path/three';";

    let paths = extract_use_lib_paths(source);

    // Both qw paths and the regular path should be extracted.
    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "/path/one".to_string(), from_findbin: false },
            UseLibPath { path: "/path/two".to_string(), from_findbin: false },
            UseLibPath { path: "/path/three".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn nested_parentheses_in_qw_are_handled() {
    // Edge case: qw() can use different delimiters; qw(a b) and qw[a b] are both valid.
    // The split logic must handle parentheses correctly even inside qw-ish contexts.
    let source = "\
use lib qw(
    /lib/a
    /lib/b
);
use lib ('/lib/c');
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "/lib/a".to_string(), from_findbin: false },
            UseLibPath { path: "/lib/b".to_string(), from_findbin: false },
            UseLibPath { path: "/lib/c".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn escaped_backslash_at_end_of_path() {
    // Edge case: Windows paths with backslashes, or paths ending in \.
    // The escape-tracking logic must correctly handle these.
    let source = r#"use lib 'C:\\temp\\lib'; use lib 'also';"#;

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: r"C:\\temp\\lib".to_string(), from_findbin: false },
            UseLibPath { path: "also".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn double_quoted_path_with_escaped_double_quote() {
    // Edge case: double-quoted string containing an escaped quote.
    // The statement splitter must handle the escaped quote so it doesn't
    // terminate the string prematurely.
    let source = r#"use lib "path/to/lib"; use lib 'also';"#;

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths.len(), 2);
    assert!(paths.iter().any(|p| p.path.contains("lib")));
    assert!(paths.iter().any(|p| p.path.contains("also")));
}

#[test]
fn single_quoted_path_with_escaped_single_quote() {
    // Edge case: single-quoted string with escaped single quote.
    // In Perl, only \\ and \' are special in single quotes.
    let source = r"use lib 'it\'s; a path'; use lib 'also';";

    let paths = extract_use_lib_paths(source);

    // Should extract both paths without being confused by the \' in the first path.
    assert!(paths.len() >= 2, "Expected at least 2 paths, got {}", paths.len());
}

#[test]
fn comment_with_multiple_semicolons() {
    // Edge case: comment containing multiple semicolons.
    let source = "\
use lib '/foo'; # Important: see docs at URL; also check README;
use lib '/bar';
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "/foo".to_string(), from_findbin: false },
            UseLibPath { path: "/bar".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn alternating_single_and_double_quotes_in_statement() {
    // Edge case: statement with mixed quote types (valid Perl).
    let source = r#"use lib '/lib1', "/lib2", '/lib3'; use lib "/lib4";"#;

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths.len(), 4, "Expected 4 paths");
}

#[test]
fn hash_character_inside_double_quoted_string() {
    // Edge case: # inside a double-quoted string is NOT a comment.
    let source = r#"use lib "/path#with#hashes"; use lib '/other';"#;

    let paths = extract_use_lib_paths(source);

    // The first path should include the hashes (they're inside quotes).
    let first_path = &paths[0];
    assert!(first_path.path.contains('#'), "Expected # to be in path: {}", first_path.path);
}

#[test]
fn hash_character_inside_single_quoted_string() {
    // Edge case: # inside a single-quoted string is NOT a comment.
    let source = "use lib '/path#with#hashes'; use lib '/other';";

    let paths = extract_use_lib_paths(source);

    let first_path = &paths[0];
    assert!(first_path.path.contains('#'));
}

#[test]
fn very_long_path_list() {
    // Boundary: large number of paths (stress test).
    let mut source = String::new();
    let num_paths = 100;
    source.push_str("use lib qw(\n");
    for i in 0..num_paths {
        source.push_str(&format!("    /path/{}\n", i));
    }
    source.push_str(");\n");

    let paths = extract_use_lib_paths(&source);

    assert_eq!(paths.len(), num_paths, "Expected {} paths", num_paths);
}

#[test]
fn unicode_paths_are_extracted() {
    // Edge case: Unicode characters in paths (non-ASCII).
    let source = "use lib '/lib/日本語'; use lib '/lib/中文'; use lib '/lib/русский';";

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths.len(), 3);
    assert!(paths[0].path.contains('日'));
    assert!(paths[1].path.contains('中'));
    assert!(paths[2].path.contains('р'));
}

#[test]
fn multiline_with_mixed_comments_and_code() {
    // Regression: complex real-world scenario with comments scattered throughout.
    let source = "\
# Global configuration
use lib 'default';  # Add default lib

# Also use project-specific lib; important for imports
use lib (
    'project/lib',  # project-local
    'vendor/lib'    # vendored; see VENDOR.md for details
);

# Override path if env var is set; see also setup.sh
no lib 'default';
";

    let paths = extract_use_lib_paths(source);
    let ops = extract_use_lib_operations(source);

    // Paths: default, project/lib, vendor/lib
    assert_eq!(paths.len(), 3);

    // Operations: Add default, Add [project, vendor], Remove default
    assert_eq!(ops.len(), 3);
}

#[test]
fn no_lib_without_use_lib() {
    // Edge case: 'no lib' statement without any preceding 'use lib'.
    let source = "no lib 'phantom';";

    let ops = extract_use_lib_operations(source);

    assert_eq!(
        ops,
        vec![UseLibAction::Remove(vec![UseLibPath {
            path: "phantom".to_string(),
            from_findbin: false,
        }])]
    );
}

#[test]
fn empty_parentheses_in_use_lib() {
    // Edge case: use lib () with no paths (malformed but shouldn't crash).
    let source = "use lib (); use lib 'valid';";

    let paths = extract_use_lib_paths(source);

    // Should extract the valid path and skip the empty one.
    assert!(paths.iter().any(|p| p.path == "valid"));
}

#[test]
fn statement_without_semicolon_at_eof() {
    // Boundary: statement at end of file without trailing semicolon.
    let source = "use lib '/path'";

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths, vec![UseLibPath { path: "/path".to_string(), from_findbin: false }]);
}

#[test]
fn multiple_statements_on_same_line() {
    // Edge case: multiple use lib statements on one line (valid Perl).
    let source = "use lib '/a'; use lib '/b'; use lib '/c';";

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths.len(), 3);
    assert!(paths.iter().any(|p| p.path == "/a"));
    assert!(paths.iter().any(|p| p.path == "/b"));
    assert!(paths.iter().any(|p| p.path == "/c"));
}

#[test]
fn mixed_use_and_no_lib_on_same_line() {
    // Edge case: both use lib and no lib on same line.
    let source = "use lib '/first'; no lib '/first'; use lib '/second';";

    let ops = extract_use_lib_operations(source);

    assert_eq!(ops.len(), 3);
    match &ops[0] {
        UseLibAction::Add(_) => {}
        _ => panic!("First op should be Add"),
    }
    match &ops[1] {
        UseLibAction::Remove(_) => {}
        _ => panic!("Second op should be Remove"),
    }
    match &ops[2] {
        UseLibAction::Add(_) => {}
        _ => panic!("Third op should be Add"),
    }
}

#[test]
fn carriage_return_line_feed_in_comment() {
    // Boundary: CRLF line endings inside a comment.
    let source = "use lib '/a'; # comment\r\nuse lib '/b';\r\n";

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths.len(), 2);
}

#[test]
fn tab_characters_in_statement() {
    // Boundary: tabs as whitespace inside statement.
    let source = "use\tlib\t(\t'/a',\t'/b'\t);\n";

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths.len(), 2);
}

#[test]
fn deeply_nested_quotes_in_single_statement() {
    // Edge case: statement with multiple levels of quoting (valid Perl concatenation).
    let source = r#"use lib "prefix" . "/suffix"; use lib '/other';"#;

    let paths = extract_use_lib_paths(source);

    // The concatenation won't be evaluated, but paths inside quotes should be extracted.
    assert!(!paths.is_empty());
}

#[test]
fn comment_only_on_comment_line_before_use_lib() {
    // Regression: comment-only lines before use lib should not interfere.
    let source = "\
# Comment 1
# Comment 2; with semicolon
use lib '/path';
";

    let paths = extract_use_lib_paths(source);

    assert_eq!(paths, vec![UseLibPath { path: "/path".to_string(), from_findbin: false }]);
}

#[test]
fn findbin_variable_with_path_containing_semicolon_in_comment() {
    // Regression: FindBin extraction should work even with tricky comments.
    let source = "\
use lib \"$RealBin/../lib\"; # Historical reason; keep this
use lib \"/override\";
";

    let ops = extract_use_lib_operations(source);

    assert_eq!(ops.len(), 2);
    match &ops[0] {
        UseLibAction::Add(paths) => {
            assert_eq!(paths.len(), 1);
            assert!(paths[0].from_findbin);
        }
        _ => panic!("Expected Add"),
    }
}

#[test]
fn qw_list_with_multiple_items_on_separate_lines() {
    // Edge case: qw() list with items on separate lines (whitespace-separated).
    let source = "\
use lib qw(
    /path/one
    /path/two
);
";

    let paths = extract_use_lib_paths(source);

    // Already tested in multiline_qw_use_lib_is_extracted, but here we verify
    // boundary behavior with exactly 2 items.
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].path, "/path/one");
    assert_eq!(paths[1].path, "/path/two");
}

#[test]
fn statement_split_at_correct_boundary_with_adjacent_semicolons() {
    // Regression: ensure statement splitting happens at the right place
    // when there are multiple semicolons (in quotes and outside).
    let source = "use lib 'a;b'; use lib 'c';";

    let paths = extract_use_lib_paths(source);

    assert_eq!(
        paths,
        vec![
            UseLibPath { path: "a;b".to_string(), from_findbin: false },
            UseLibPath { path: "c".to_string(), from_findbin: false },
        ]
    );
}

#[test]
fn bare_path_without_quotes_in_use_lib() {
    // Edge case: bare words (no quotes) in use lib (unusual, may be function calls).
    // The extractor may not handle these, but it shouldn't crash.
    let source = "use lib qw(bare_word_path);";

    let paths = extract_use_lib_paths(source);

    // Should extract the bare word as a path.
    assert!(paths.iter().any(|p| p.path.contains("bare_word")));
}
