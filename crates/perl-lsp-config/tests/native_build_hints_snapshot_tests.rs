//! Snapshot tests for detect_native_build_hints output.
//!
//! These tests serialize NativeBuildHints to a stable text representation
//! and compare against stored baselines to detect any output changes immediately.

use perl_lsp_config::{NativeBuildHints, detect_native_build_hints};
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A stable, sorted representation of NativeBuildHints for snapshot comparison.
/// This format ensures consistent ordering of vector elements.
#[derive(Debug)]
struct SortedHints {
    include_dirs: Vec<String>,
    libs_flags: Vec<String>,
    define_flags: Vec<String>,
    object_files: Vec<String>,
    myextlib_files: Vec<String>,
}

impl From<&NativeBuildHints> for SortedHints {
    fn from(hints: &NativeBuildHints) -> Self {
        let mut include_dirs = hints.include_dirs.clone();
        include_dirs.sort();
        let mut libs_flags = hints.libs_flags.clone();
        libs_flags.sort();
        let mut define_flags = hints.define_flags.clone();
        define_flags.sort();
        let mut object_files = hints.object_files.clone();
        object_files.sort();
        let mut myextlib_files = hints.myextlib_files.clone();
        myextlib_files.sort();
        SortedHints { include_dirs, libs_flags, define_flags, object_files, myextlib_files }
    }
}

impl SortedHints {
    fn to_string(&self) -> String {
        format!(
            "include_dirs: {:?}\nlibs_flags: {:?}\ndefine_flags: {:?}\nobject_files: {:?}\nmyextlib_files: {:?}\n",
            self.include_dirs,
            self.libs_flags,
            self.define_flags,
            self.object_files,
            self.myextlib_files
        )
    }
}

/// Compare detected hints against a stored text snapshot.
fn assert_snapshot(hints: &NativeBuildHints, snapshot_path: &Path) -> TestResult {
    let sorted = SortedHints::from(hints);
    let content = sorted.to_string();

    if snapshot_path.exists() {
        let expected = fs::read_to_string(snapshot_path)?;
        let expected = expected.trim();
        let actual = content.trim();
        if expected != actual {
            return Err(format!(
                "Snapshot mismatch at {:?}\n\nExpected:\n{}\n\nGot:\n{}\n",
                snapshot_path, expected, actual
            )
            .into());
        }
    } else {
        // Snapshot doesn't exist yet - create it
        let dir = snapshot_path.parent().unwrap();
        fs::create_dir_all(dir)?;
        fs::write(snapshot_path, &content)?;
        println!("Created snapshot at {:?}", snapshot_path);
    }
    Ok(())
}

fn write_script(dir: &tempfile::TempDir, name: &str, content: &str) -> TestResult {
    fs::write(dir.path().join(name), content)?;
    Ok(())
}

fn snapshot_path(name: &str) -> std::path::PathBuf {
    Path::new("/home/hermes/.hermes/state/conveyor/work-8972b8ca/snapshots").join(name)
}

// =============================================================================
// Snapshot tests for detect_native_build_hints
// =============================================================================

#[test]
fn snapshot_default_hints() -> TestResult {
    let dir = tempfile::tempdir()?;
    // No Makefile.PL or Build.PL - should get empty hints
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("default_hints.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_all_four_params() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => '-L/usr/local/lib -lssl -lcrypto',\n    DEFINE => '-DFOO=1 -DBAR=2',\n    OBJECT => 'foo.o bar.o',\n    MYEXTLIB => 'someext.a anotherext.a',\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_all_four_params.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_libs_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => '-L/usr/local/lib -lssl -lcrypto',\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_libs_only.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_define_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(&dir, "Makefile.PL", "WriteMakefile(\n    DEFINE => '-DFOO=1 -DBAR=2',\n);\n")?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_define_only.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_object_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(&dir, "Makefile.PL", "WriteMakefile(\n    OBJECT => 'foo.o bar.o baz.o',\n);\n")?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_object_only.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_myextlib_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    MYEXTLIB => 'someext.a anotherext.a',\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_myextlib_only.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(&dir, "Makefile.PL", "WriteMakefile();\n")?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_empty.txt"))?;
    Ok(())
}

#[test]
fn snapshot_build_pl_no_new_params() -> TestResult {
    let dir = tempfile::tempdir()?;
    // Build.PL doesn't use LIBS/DEFINE/OBJECT/MYEXTLIB
    write_script(
        &dir,
        "Build.PL",
        "Module::Build->new(\n    module_name => 'Foo',\n    include_dirs => ['include', 'lib'],\n    extra_compiler_flags => ['-Ilib/perl5'],\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("build_pl_no_new_params.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_with_inc() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    INC => '-Iinclude -I. -Ilocal/lib/perl5',\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_with_inc.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_double_quoted_values() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => \"-L/usr/lib -lcurl\",\n    DEFINE => \"-DVERSION=1.0\",\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_double_quoted.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_array_syntax() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => ['-L/lib1 -lfoo', '-L/lib2 -lbar'],\n    OBJECT => ['foo.o', 'bar.o'],\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_array_syntax.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_commented_skipped() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "# LIBS => '-lfoo'\nWriteMakefile(\n    LIBS => '-L/lib -lbar',\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_commented_skipped.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_dynamic_skipped() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "my $libs = '-lfoo';\nWriteMakefile(\n    LIBS => $libs,\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_dynamic_skipped.txt"))?;
    Ok(())
}

#[test]
fn snapshot_makefile_key_boundary() -> TestResult {
    let dir = tempfile::tempdir()?;
    // LIBSWORD should NOT match LIBS
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBSWORD => '-lssl',\n    OBJECT => 'foo.o',\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("makefile_key_boundary.txt"))?;
    Ok(())
}

#[test]
fn snapshot_both_makefile_and_buildpl() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => '-L/lib -lssl',\n    DEFINE => '-DFOO=1',\n);\n",
    )?;
    write_script(
        &dir,
        "Build.PL",
        "Module::Build->new(\n    module_name => 'Foo',\n    include_dirs => ['include'],\n);\n",
    )?;
    let hints = detect_native_build_hints(dir.path());
    assert_snapshot(&hints, &snapshot_path("both_makefile_and_buildpl.txt"))?;
    Ok(())
}
