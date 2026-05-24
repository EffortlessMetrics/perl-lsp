#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_uri::{
    is_file_uri, is_special_scheme, normalize_uri, source_path_from_uri_or_path, uri_extension,
    uri_key, uri_to_fs_path,
};

const MAX_INPUT_BYTES: usize = 1024;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };
    String::from_utf8_lossy(capped)
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // None of the URI helpers may panic on arbitrary input. They each take a
    // raw &str and have historically tripped over embedded NULs, control
    // characters, and percent-encoded UTF-8.
    let _ = normalize_uri(&input);
    let _ = is_file_uri(&input);
    let _ = is_special_scheme(&input);
    let _ = uri_extension(&input);
    let _ = uri_to_fs_path(&input);
    let _ = source_path_from_uri_or_path(&input);

    // uri_key must be idempotent: lookups should converge after one pass so
    // that maps using it as a key remain consistent across re-normalizations.
    let key_once = uri_key(&input);
    let key_twice = uri_key(&key_once);
    assert_eq!(key_once, key_twice, "uri_key is not idempotent");

    // normalize_uri composed with itself should also stabilize. Even when
    // the input is garbage, normalize_uri returns *something*, and applying
    // the transformation again must not change it further.
    let norm_once = normalize_uri(&input);
    let norm_twice = normalize_uri(&norm_once);
    assert_eq!(norm_once, norm_twice, "normalize_uri is not idempotent");

    // Classification consistency: a value that classifies as a file URI must
    // not simultaneously classify as a non-file special scheme.
    if is_file_uri(&input) {
        assert!(
            !is_special_scheme(&input),
            "input was classified as both file:// and a special scheme"
        );
    }

    // Exercise common scheme prefixes so the fuzzer mutates around real shapes
    // even when starting from garbage.
    for prefix in ["file://", "untitled:", "git:", "vscode-notebook:", "vscode-vfs:"] {
        let synthetic = format!("{prefix}{input}");
        let _ = normalize_uri(&synthetic);
        let _ = uri_key(&synthetic);
        let _ = uri_extension(&synthetic);
        let _ = uri_to_fs_path(&synthetic);
    }
});
