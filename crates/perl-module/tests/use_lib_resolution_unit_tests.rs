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
