use std::path::Path;

use perl_module::resolution::use_lib::{
    UseLibAction, UseLibPath, extract_use_lib_operations, resolve_use_lib_paths,
    resolve_use_lib_paths_from_source_at_offset,
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
    'alpha',
    'beta'
);
";

    let ops = extract_use_lib_operations(source);

    assert_eq!(
        ops,
        vec![UseLibAction::Add(vec![
            UseLibPath { path: "alpha".to_string(), from_findbin: false },
            UseLibPath { path: "beta".to_string(), from_findbin: false },
        ])]
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

    let include_paths = resolve_use_lib_paths_from_source_at_offset(
        source,
        source.len(),
        Path::new("/workspace"),
        None,
    );

    assert_eq!(include_paths, vec!["second".to_string()]);
}

#[test]
fn quoted_semicolon_does_not_split_statement() {
    let source = "use lib 'path;still_path';\n";

    let ops = extract_use_lib_operations(source);

    assert_eq!(
        ops,
        vec![UseLibAction::Add(vec![UseLibPath {
            path: "path;still_path".to_string(),
            from_findbin: false,
        }])]
    );
}
