use std::ffi::OsStr;
use std::path::Path;

/// Returns `true` for paths that should be skipped by the print-in-lib check.
///
/// This is a superset of `is_excluded_test_path` with extra exclusions specific
/// to the print-macro policy:
///   - `build.rs` files: Cargo build scripts use `println!("cargo:...")` to communicate
///     with Cargo itself. This is the standard mechanism; it is not "library output".
///   - Files whose name starts with `test_` (e.g. `test_parser.rs`): these are test
///     driver / helper files that live alongside library source but are only invoked
///     during test runs.
///   - Test-support crates whose primary purpose is emitting diagnostic output during
///     test execution (e.g. `perl-lsp-ux-tests`).
pub(super) fn is_excluded_for_print_check(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Build scripts may use println!("cargo:...") — standard Cargo convention.
    if file_name == "build.rs" {
        return true;
    }

    // Test-driver files next to src/ but not inside tests/ directory.
    if file_name.starts_with("test_") && file_name.ends_with(".rs") {
        return true;
    }

    // Test-support and UX-test crates use print output intentionally.
    if path.components().any(|c| c.as_os_str() == OsStr::new("perl-lsp-ux-tests")) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_rs_is_excluded() {
        assert!(is_excluded_for_print_check(Path::new("crates/my-crate/build.rs")));
    }

    #[test]
    fn test_prefix_rs_file_is_excluded() {
        assert!(is_excluded_for_print_check(Path::new("crates/my-crate/src/test_helpers.rs")));
    }

    #[test]
    fn test_prefix_without_rs_extension_not_excluded() {
        // Only .rs files with test_ prefix are excluded.
        assert!(!is_excluded_for_print_check(Path::new("crates/my-crate/src/test_helpers.txt")));
    }

    #[test]
    fn ux_tests_crate_path_is_excluded() {
        let path: PathBuf = ["crates", "perl-lsp-ux-tests", "src", "lib.rs"].iter().collect();
        assert!(is_excluded_for_print_check(&path));
    }

    #[test]
    fn normal_lib_rs_is_not_excluded() {
        assert!(!is_excluded_for_print_check(Path::new("crates/perl-parser/src/lib.rs")));
    }

    #[test]
    fn file_starting_with_test_but_not_rs_extension_is_not_excluded() {
        // test_helpers.toml should NOT be excluded.
        assert!(!is_excluded_for_print_check(Path::new("crates/my-crate/src/test_helpers.toml")));
    }
}
