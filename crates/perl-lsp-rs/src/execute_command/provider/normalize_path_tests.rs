use super::normalize_path_for_external_command;
use std::path::{Path, PathBuf};

/// On Windows the `\\?\` extended-length prefix must be stripped so that
/// external commands (perl.exe, prove, yath) can accept the path.
#[test]
#[cfg(target_os = "windows")]
fn strips_extended_length_prefix_on_windows() {
    let prefixed = Path::new(r"\\?\C:\Users\test\file.pl");
    let result = normalize_path_for_external_command(prefixed);
    assert_eq!(
        result,
        PathBuf::from(r"C:\Users\test\file.pl"),
        "Extended-length prefix should be stripped: got {:?}",
        result
    );
}

/// On Windows, paths without the prefix are returned unchanged.
#[test]
#[cfg(target_os = "windows")]
fn passthrough_plain_windows_path() {
    let plain = Path::new(r"C:\Users\test\file.pl");
    let result = normalize_path_for_external_command(plain);
    assert_eq!(result, PathBuf::from(r"C:\Users\test\file.pl"));
}

/// On non-Windows, the helper is a pass-through identity — paths are
/// returned exactly as given regardless of content.
#[test]
#[cfg(not(target_os = "windows"))]
fn passthrough_on_non_windows() {
    let path = Path::new("/tmp/test_valid.pl");
    let result = normalize_path_for_external_command(path);
    assert_eq!(result, PathBuf::from("/tmp/test_valid.pl"));
}

/// Verify the helper handles a synthetic Windows extended-length prefix
/// as a string: even on non-Windows the conditional compilation means the
/// prefix is left untouched (since there is no `\\?\` on Unix paths).
/// This test documents the cross-platform contract.
#[test]
#[cfg(not(target_os = "windows"))]
fn no_stripping_on_non_windows_even_for_unc_like_string() {
    // On Linux/macOS this is just a literal path string — no stripping.
    let path = Path::new(r"\\?\C:\foo\bar");
    let result = normalize_path_for_external_command(path);
    assert_eq!(result, PathBuf::from(r"\\?\C:\foo\bar"));
}

/// On Windows, the UNC extended-length form `\\?\UNC\server\share\...` must
/// become `\\server\share\...` — NOT `UNC\server\share\...`.
///
/// `Path::canonicalize` on Windows returns `\\?\UNC\...` for network paths.
/// Simply stripping `\\?\` would leave `UNC\server\share\...` which perl.exe
/// cannot resolve.  The correct result is a plain UNC path `\\server\share\...`.
#[test]
#[cfg(target_os = "windows")]
fn strips_extended_length_unc_prefix_on_windows() {
    let prefixed = Path::new(r"\\?\UNC\fileserver\share\project\test.pl");
    let result = normalize_path_for_external_command(prefixed);
    assert_eq!(
        result,
        PathBuf::from(r"\\fileserver\share\project\test.pl"),
        "UNC extended-length prefix should become plain UNC path: got {:?}",
        result
    );
}

/// On non-Windows, a UNC extended-length string is also left untouched —
/// the conditional compilation means the entire stripping block is absent.
#[test]
#[cfg(not(target_os = "windows"))]
fn no_stripping_on_non_windows_even_for_unc_extended_string() {
    let path = Path::new(r"\\?\UNC\fileserver\share\project\test.pl");
    let result = normalize_path_for_external_command(path);
    assert_eq!(result, PathBuf::from(r"\\?\UNC\fileserver\share\project\test.pl"));
}
