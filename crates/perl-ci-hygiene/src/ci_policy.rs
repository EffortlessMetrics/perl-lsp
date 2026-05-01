use std::ffi::OsStr;
use std::path::Path;

const CI_REPORT_CRATES_EXCLUDE: [&str; 5] = [
    "tree-sitter-perl-c",
    "perl-parser-pest",
    "perl-tdd-support",
    "perl-test-must",
    "perl-ci-hygiene",
];

const CI_TEST_FILE_SUFFIXES: [&str; 3] = ["_test.rs", "_tests.rs", "tests.rs"];

pub(crate) fn is_excluded_test_path(path: &Path) -> bool {
    if path.components().any(|component| {
        let value = component.as_os_str();
        value == OsStr::new("tests")
            || value == OsStr::new("benches")
            || value == OsStr::new("examples")
            || value == OsStr::new("bin")
    }) {
        return true;
    }

    if let Some(file_name) = path.file_name().and_then(|name| name.to_str())
        && CI_TEST_FILE_SUFFIXES.iter().any(|suffix| file_name.ends_with(suffix))
    {
        return true;
    }

    path.components().any(|component| {
        CI_REPORT_CRATES_EXCLUDE.iter().any(|item| component.as_os_str() == OsStr::new(item))
    })
}
