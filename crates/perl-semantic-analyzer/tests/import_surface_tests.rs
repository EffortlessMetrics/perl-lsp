use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::import_surface::{ImportKind, ImportSurface};

fn build_surface(code: &str) -> Result<ImportSurface, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    Ok(ImportSurface::from_ast(&ast))
}

#[test]
fn import_surface_collects_qw_use_list() -> Result<(), Box<dyn std::error::Error>> {
    let surface = build_surface("use List::Util qw(sum min);\nsum();\n")?;
    let sum = surface
        .entries()
        .iter()
        .find(|entry| entry.bare_name == "sum")
        .ok_or("sum should be collected")?;
    assert_eq!(sum.source_package, "List::Util");
    assert!(matches!(sum.kind, ImportKind::UseList));
    assert!(sum.is_resolved);
    Ok(())
}

#[test]
fn import_surface_collects_parenthesized_and_single_quoted_use_list(
) -> Result<(), Box<dyn std::error::Error>> {
    let surface = build_surface("use Foo ('bar');\nuse Baz 'quux';\n")?;
    assert!(surface.contains_name("bar"), "bar should be visible from paren list");
    assert!(surface.contains_name("quux"), "quux should be visible from single-quoted form");
    Ok(())
}

#[test]
fn import_surface_expands_known_export_tags() -> Result<(), Box<dyn std::error::Error>> {
    let surface = build_surface("use POSIX ':sys_wait_h';\nWIFEXITED($status);\n")?;
    let wifexited = surface
        .entries()
        .iter()
        .find(|entry| entry.bare_name == "WIFEXITED")
        .ok_or("WIFEXITED should be expanded from :sys_wait_h")?;
    assert_eq!(wifexited.source_package, "POSIX");
    assert!(matches!(wifexited.kind, ImportKind::ExportTag { .. }));
    assert!(wifexited.is_resolved);
    Ok(())
}

#[test]
fn import_surface_collects_use_constant_forms() -> Result<(), Box<dyn std::error::Error>> {
    let surface = build_surface(
        "use constant PI => 3.14;\nuse constant { MIN => 1, MAX => 9 };\npackage App;\nuse constant APP_NAME => 'x';\n",
    )?;
    let pi = surface
        .entries()
        .iter()
        .find(|entry| entry.bare_name == "PI")
        .ok_or("PI should be collected")?;
    assert_eq!(pi.source_package, "main");
    assert!(matches!(pi.kind, ImportKind::UseConstant));
    assert!(surface.contains_name("MIN"));
    assert!(surface.contains_name("MAX"));
    let app_name = surface
        .entries()
        .iter()
        .find(|entry| entry.bare_name == "APP_NAME")
        .ok_or("APP_NAME should be collected")?;
    assert_eq!(app_name.source_package, "App");
    Ok(())
}
