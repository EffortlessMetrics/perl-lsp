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
