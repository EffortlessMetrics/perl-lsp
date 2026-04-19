use perl_lsp_config::{WorkspaceConfig, detect_native_build_hints};
use std::fs;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_script(dir: &tempfile::TempDir, name: &str, content: &str) -> TestResult {
    fs::write(dir.path().join(name), content)?;
    Ok(())
}

#[test]
fn makefile_pl_literal_inc_extracts_include_dirs() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    INC => '-Iinclude -I. -Ilocal/lib/perl5',\n);\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    assert_eq!(hints.include_dirs, vec!["include", ".", "local/lib/perl5"]);
    Ok(())
}

#[test]
fn build_pl_literal_arrays_extract_include_dirs_and_compiler_flags() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Build.PL",
        "Module::Build->new(\n    include_dirs => ['include', '.'],\n    extra_compiler_flags => ['-Ilocal/include -I.'],\n);\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    assert_eq!(hints.include_dirs, vec!["include", ".", "local/include"]);
    Ok(())
}

#[test]
fn dynamic_native_build_hints_are_ignored() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "my $extra = '-Iignored';\nWriteMakefile( INC => join(' ', '-Iinclude', $extra) );\n",
    )?;
    write_script(
        &dir,
        "Build.PL",
        "Module::Build->new(\n    include_dirs => [map { $_ } qw(include .)],\n);\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    assert!(hints.include_dirs.is_empty());
    Ok(())
}

#[test]
fn commented_or_quoted_native_build_hints_are_ignored() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "# INC => '-Iignored-comment'\nmy $doc = \"INC => '-Iignored-string'\";\nWriteMakefile( INC => '-Iinclude' );\n",
    )?;
    write_script(
        &dir,
        "Build.PL",
        "my $doc = \"include_dirs => ['ignored-string']\";\n# include_dirs => ['ignored-comment']\nModule::Build->new( include_dirs => ['module-include'] );\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    assert_eq!(hints.include_dirs, vec!["include", "module-include"]);
    Ok(())
}

#[test]
fn workspace_config_refresh_native_build_hints_leaves_include_paths_untouched() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile( INC => '-Iinclude' );\n",
    )?;

    let mut cfg = WorkspaceConfig::default();
    let include_paths_before = cfg.include_paths.clone();
    cfg.refresh_native_build_hints(dir.path());

    assert_eq!(cfg.include_paths, include_paths_before);
    assert_eq!(cfg.native_build_hints.include_dirs, vec!["include"]);
    Ok(())
}
