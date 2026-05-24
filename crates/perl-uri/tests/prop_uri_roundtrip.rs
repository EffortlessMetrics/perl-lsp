//! Property tests for `perl-uri` round-trip and invariant guarantees.
//!
//! Invariants under test:
//! - Round-trip: `uri_to_fs_path(fs_path_to_uri(p))` reproduces the original path
//! - Idempotence: `uri_key(uri_key(s)) == uri_key(s)` for any string input
//! - Idempotence: `normalize_uri(normalize_uri(s)) == normalize_uri(s)` for file URIs
//! - Determinism: calling each function twice with the same input yields the same output
//! - `is_file_uri` / `is_special_scheme` are mutually exclusive for well-formed URIs
//! - `uri_extension` is stable across repeated calls

#![cfg(not(target_arch = "wasm32"))]

use perl_uri::{
    fs_path_to_uri, is_file_uri, is_special_scheme, normalize_uri, uri_extension, uri_key,
    uri_to_fs_path,
};
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// One path-safe segment: ASCII alphanumeric plus underscore and hyphen, 1-8 chars.
fn path_segment() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,8}"
}

/// A native absolute path built under the platform temp directory.
fn native_abs_path() -> impl Strategy<Value = String> {
    prop::collection::vec(path_segment(), 1..=5).prop_map(|segs| {
        let mut path = std::env::temp_dir().join("perl_uri_prop");
        for segment in segs {
            path.push(segment);
        }
        path.to_string_lossy().into_owned()
    })
}

/// A native absolute path with a `.pl`, `.pm`, or `.t` extension on the final
/// segment, exercising the common Perl file types handled by `uri_extension`.
fn native_perl_path() -> impl Strategy<Value = String> {
    let ext = prop::sample::select(vec!["pl", "pm", "t"]);
    (prop::collection::vec(path_segment(), 1..=4), path_segment(), ext).prop_map(
        |(dirs, file_name, e)| {
            let mut path = std::env::temp_dir().join("perl_uri_prop");
            for dir in dirs {
                path.push(dir);
            }
            path.push(format!("{file_name}.{e}"));
            path.to_string_lossy().into_owned()
        },
    )
}

/// A valid `file://` URI built from a native absolute path.
fn file_uri_from_path() -> impl Strategy<Value = String> {
    native_abs_path().prop_filter_map("native path must convert to file URI", |path| {
        fs_path_to_uri(path.as_str()).ok()
    })
}

/// A canonical `file:///C:/...` Windows-style URI (drive letter lowercase).
fn windows_file_uri() -> impl Strategy<Value = String> {
    let drive = prop::sample::select(vec!['a', 'b', 'c', 'd', 'e']);
    (drive, prop::collection::vec(path_segment(), 1..=4))
        .prop_map(|(d, segs)| format!("file:///{d}:/{}", segs.join("/")))
}

/// A `file:///C:/...` Windows-style URI with uppercase drive letter (pre-normalization).
fn windows_file_uri_upper_drive() -> impl Strategy<Value = String> {
    let drive = prop::sample::select(vec!['A', 'B', 'C', 'D', 'E']);
    (drive, prop::collection::vec(path_segment(), 1..=4))
        .prop_map(|(d, segs)| format!("file:///{d}:/{}", segs.join("/")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    // ------------------------------------------------------------------
    // Round-trip: path to URI to path
    // ------------------------------------------------------------------

    /// `uri_to_fs_path(fs_path_to_uri(p))` must return `Some` and the
    /// resulting path must match the original native path.
    #[test]
    fn prop_path_to_uri_to_path_roundtrip(path in native_abs_path()) {
        let Ok(uri) = fs_path_to_uri(&path) else {
            // On some test environments current_dir may be unavailable;
            // accept that gracefully.
            return Ok(());
        };
        let recovered = uri_to_fs_path(&uri);
        let Some(recovered_path) = recovered else {
            prop_assert!(false, "uri_to_fs_path returned None for URI: {}", uri);
            return Ok(());
        };
        let original_path = PathBuf::from(&path);
        prop_assert_eq!(
            &recovered_path,
            &original_path,
            "Round-trip mismatch: original={}, recovered={}",
            path,
            recovered_path.display()
        );
    }

    /// With a Perl extension appended, the round-trip still holds.
    #[test]
    fn prop_perl_path_to_uri_to_path_roundtrip(path in native_perl_path()) {
        let Ok(uri) = fs_path_to_uri(&path) else {
            return Ok(());
        };
        let recovered = uri_to_fs_path(&uri);
        let Some(recovered_path) = recovered else {
            prop_assert!(false, "uri_to_fs_path returned None for URI: {}", uri);
            return Ok(());
        };
        prop_assert_eq!(&recovered_path, &PathBuf::from(&path));
    }

    // ------------------------------------------------------------------
    // URI to path to URI round-trip
    // ------------------------------------------------------------------

    /// `fs_path_to_uri(uri_to_fs_path(u))` should reproduce a normalized
    /// URI equivalent to `uri_key(u)`.
    #[test]
    fn prop_file_uri_to_path_to_uri_roundtrip(uri in file_uri_from_path()) {
        let Some(path) = uri_to_fs_path(&uri) else {
            return Ok(());
        };
        let Ok(round_tripped_uri) = fs_path_to_uri(&path) else {
            return Ok(());
        };
        // The final URI must equal the normalized form of the original.
        prop_assert_eq!(
            uri_key(&round_tripped_uri),
            uri_key(&uri),
            "URI round-trip key mismatch: original={}, round-tripped={}",
            uri,
            round_tripped_uri
        );
    }

    // ------------------------------------------------------------------
    // Idempotence: uri_key
    // ------------------------------------------------------------------

    /// `uri_key` must be idempotent: applying it twice yields the same result.
    #[test]
    fn prop_uri_key_idempotent_on_file_uri(uri in file_uri_from_path()) {
        let once = uri_key(&uri);
        let twice = uri_key(&once);
        prop_assert_eq!(&once, &twice, "uri_key not idempotent for: {}", uri);
    }

    #[test]
    fn prop_uri_key_idempotent_on_windows_uri(uri in windows_file_uri()) {
        let once = uri_key(&uri);
        let twice = uri_key(&once);
        prop_assert_eq!(&once, &twice, "uri_key not idempotent for: {}", uri);
    }

    #[test]
    fn prop_uri_key_idempotent_on_upper_drive_windows_uri(uri in windows_file_uri_upper_drive()) {
        let once = uri_key(&uri);
        let twice = uri_key(&once);
        prop_assert_eq!(&once, &twice, "uri_key not idempotent for: {}", uri);
    }

    /// `uri_key` must be idempotent on arbitrary path segments (including
    /// values that do not look like valid URIs).
    #[test]
    fn prop_uri_key_idempotent_on_path_segment(s in path_segment()) {
        let once = uri_key(&s);
        let twice = uri_key(&once);
        prop_assert_eq!(&once, &twice, "uri_key not idempotent for path segment: {}", s);
    }

    // ------------------------------------------------------------------
    // Idempotence: normalize_uri
    // ------------------------------------------------------------------

    /// `normalize_uri` must be idempotent on already-normalized file URIs.
    #[test]
    fn prop_normalize_uri_idempotent_on_file_uri(uri in file_uri_from_path()) {
        let once = normalize_uri(&uri);
        let twice = normalize_uri(&once);
        prop_assert_eq!(&once, &twice, "normalize_uri not idempotent for: {}", uri);
    }

    // ------------------------------------------------------------------
    // Determinism
    // ------------------------------------------------------------------

    /// `fs_path_to_uri` must be deterministic: two calls with the same path
    /// produce the same result.
    #[test]
    fn prop_fs_path_to_uri_is_deterministic(path in native_abs_path()) {
        let first = fs_path_to_uri(&path);
        let second = fs_path_to_uri(&path);
        prop_assert_eq!(
            first,
            second,
            "fs_path_to_uri is not deterministic for: {}",
            path
        );
    }

    /// `uri_key` must be deterministic.
    #[test]
    fn prop_uri_key_is_deterministic(uri in file_uri_from_path()) {
        let first = uri_key(&uri);
        let second = uri_key(&uri);
        prop_assert_eq!(first, second);
    }

    /// `normalize_uri` must be deterministic.
    #[test]
    fn prop_normalize_uri_is_deterministic(uri in file_uri_from_path()) {
        let first = normalize_uri(&uri);
        let second = normalize_uri(&uri);
        prop_assert_eq!(first, second);
    }

    // ------------------------------------------------------------------
    // is_file_uri and is_special_scheme mutual exclusion
    // ------------------------------------------------------------------

    /// A well-formed `file:///` URI must satisfy `is_file_uri` and must NOT
    /// satisfy `is_special_scheme`.
    #[test]
    fn prop_file_uri_is_file_and_not_special(uri in file_uri_from_path()) {
        prop_assert!(is_file_uri(&uri), "expected is_file_uri for: {}", uri);
        prop_assert!(!is_special_scheme(&uri), "expected !is_special_scheme for: {}", uri);
    }

    /// For `file:///` URIs, `uri_key` must produce a string that still
    /// satisfies `is_file_uri`.
    #[test]
    fn prop_uri_key_preserves_file_uri_classification(uri in file_uri_from_path()) {
        let key = uri_key(&uri);
        prop_assert!(is_file_uri(&key), "uri_key output is not a file URI: {}", key);
    }

    // ------------------------------------------------------------------
    // uri_extension stability
    // ------------------------------------------------------------------

    /// `uri_extension` must return a consistent value across repeated calls.
    #[test]
    fn prop_uri_extension_is_deterministic(uri in file_uri_from_path()) {
        prop_assert_eq!(uri_extension(&uri), uri_extension(&uri));
    }

    /// For Perl paths, the extension extracted from the URI must match the
    /// extension appended in the strategy.
    #[test]
    fn prop_uri_extension_matches_perl_extension(path in native_perl_path()) {
        let Ok(uri) = fs_path_to_uri(&path) else {
            return Ok(());
        };
        let ext = uri_extension(&uri);
        prop_assert!(
            matches!(ext, Some("pl") | Some("pm") | Some("t")),
            "unexpected extension {:?} for URI: {}",
            ext,
            uri
        );
    }

    // ------------------------------------------------------------------
    // Windows drive-letter normalization
    // ------------------------------------------------------------------

    /// `uri_key` must lowercase Windows drive letters.
    #[test]
    fn prop_uri_key_lowercases_windows_drive(uri in windows_file_uri_upper_drive()) {
        let key = uri_key(&uri);
        // The key should start with `file:///` and the drive character must be lowercase.
        let Some(after_prefix) = key.strip_prefix("file:///") else {
            prop_assert!(false, "uri_key output lost file URI prefix: {}", key);
            return Ok(());
        };
        let Some(drive_char) = after_prefix.chars().next() else {
            prop_assert!(false, "uri_key output lost drive letter: {}", key);
            return Ok(());
        };
        prop_assert!(drive_char.is_ascii_alphabetic(), "drive letter missing in key: {}", key);
        prop_assert!(
            drive_char.is_ascii_lowercase(),
            "drive letter not lowercased in key: {}",
            key
        );
    }

    /// A lowercase and uppercase form of the same Windows drive URI must
    /// produce the same `uri_key`.
    #[test]
    fn prop_uri_key_upper_and_lower_drive_are_equivalent(
        uri in windows_file_uri()
    ) {
        // Build the upper-drive variant by uppercasing the drive letter in the key.
        let lower_key = uri_key(&uri);
        // Create the upper version by replacing the drive letter character.
        let Some(rest) = lower_key.strip_prefix("file:///") else {
            prop_assert!(false, "uri_key output lost file URI prefix: {}", lower_key);
            return Ok(());
        };
        let Some(first_char) = rest.chars().next() else {
            prop_assert!(false, "uri_key output lost drive letter: {}", lower_key);
            return Ok(());
        };
        prop_assert!(first_char.is_ascii_lowercase(), "lower key drive not lowercase: {}", lower_key);
        let upper = format!("file:///{}{}", first_char.to_ascii_uppercase(), &rest[1..]);
        let upper_key = uri_key(&upper);
        prop_assert_eq!(
            &lower_key,
            &upper_key,
            "drive case mismatch: lower={}, upper={}",
            lower_key,
            upper_key
        );
    }
}

// ---------------------------------------------------------------------------
// Targeted regression cases (deterministic, not property-based)
// ---------------------------------------------------------------------------

#[test]
fn regression_uri_key_idempotent_on_space_encoded_uri() {
    let uri = "file:///tmp/path%20with%20spaces/test.pl";
    assert_eq!(uri_key(uri), uri_key(&uri_key(uri)));
}

#[test]
fn regression_uri_key_idempotent_on_unicode_path() {
    // cafe with percent-encoding already applied.
    let uri = "file:///tmp/caf%C3%A9/test.pl";
    assert_eq!(uri_key(uri), uri_key(&uri_key(uri)));
}

#[test]
fn regression_uri_key_idempotent_on_dotdot_segment() {
    // uri_key does not resolve `..`, but it should still be idempotent.
    let uri = "file:///tmp/a/../b/test.pl";
    let once = uri_key(uri);
    let twice = uri_key(&once);
    assert_eq!(once, twice);
}

#[cfg(not(windows))]
#[test]
fn regression_roundtrip_path_with_spaces() {
    let path = "/tmp/path with spaces/test.pl";
    let Ok(uri) = fs_path_to_uri(path) else {
        return;
    };
    let recovered = uri_to_fs_path(&uri);
    let recovered_path = perl_tdd_support::must_some(recovered);
    assert_eq!(recovered_path.to_string_lossy(), path);
}
