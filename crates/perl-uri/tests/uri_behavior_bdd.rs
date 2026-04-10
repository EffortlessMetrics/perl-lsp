//! Behavior-driven tests for `perl-uri`.
//!
//! These scenarios document protocol-boundary behavior using
//! `given_..._when_..._then_...` style test names.

use perl_uri::{
    fs_path_to_uri, is_file_uri, is_special_scheme, normalize_uri, uri_extension, uri_key,
    uri_to_fs_path,
};

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn given_absolute_filesystem_path_when_normalizing_then_file_uri_is_returned() -> Result<(), String>
{
    let normalized = normalize_uri("/tmp/perl-uri-bdd-script.pl");

    if !normalized.starts_with("file:///") {
        return Err(format!("expected file URI, got: {normalized}"));
    }

    if uri_extension(&normalized) != Some("pl") {
        return Err(format!("expected .pl extension, got: {:?}", uri_extension(&normalized)));
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn given_file_uri_with_encoded_segments_when_converting_to_path_then_segments_are_decoded()
-> Result<(), String> {
    let input = "file:///tmp/path%20with%20spaces/example.pm";
    let path = uri_to_fs_path(input).ok_or("expected file URI to convert")?;
    let path_str = path.to_string_lossy();

    if !path_str.contains("path with spaces") {
        return Err(format!("expected decoded spaces, got: {path_str}"));
    }

    if !path_str.ends_with("example.pm") {
        return Err(format!("expected example.pm suffix, got: {path_str}"));
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn given_relative_path_when_converting_to_uri_then_result_is_absolute_file_uri()
-> Result<(), String> {
    let uri = fs_path_to_uri("tests/fixtures/sample.t")?;

    if !uri.starts_with("file:///") {
        return Err(format!("expected absolute file URI, got: {uri}"));
    }

    if !uri.contains("sample.t") {
        return Err(format!("expected converted URI to include filename, got: {uri}"));
    }

    Ok(())
}

#[test]
fn given_windows_drive_letter_file_uri_when_keying_then_drive_letter_is_lowercased() {
    let key = uri_key("file:///C:/Users/Dev/Project.pm");
    assert_eq!(key, "file:///c:/Users/Dev/Project.pm");
}

#[test]
fn given_non_file_scheme_when_classifying_then_scheme_is_special_not_file() {
    let uri = "https://example.com/project/lib/Foo.pm";
    assert!(!is_file_uri(uri));
    assert!(is_special_scheme(uri));
}

#[test]
fn given_file_uri_when_classifying_then_uri_is_file_and_not_special() {
    let uri = "file:///tmp/Foo/Bar.pm";
    assert!(is_file_uri(uri));
    assert!(!is_special_scheme(uri));
}

#[test]
fn given_uri_with_query_and_fragment_when_extracting_extension_then_extension_is_from_path() {
    let uri = "file:///tmp/lib/My/Module.pm?cache=1#L22";
    assert_eq!(uri_extension(uri), Some("pm"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn given_roundtrip_conversion_when_path_contains_unicode_then_original_path_is_preserved()
-> Result<(), String> {
    let original = "/tmp/perl-uri/日本語/モジュール.pl";

    let uri = fs_path_to_uri(original)?;
    let converted = uri_to_fs_path(&uri).ok_or("expected URI to convert back to path")?;

    if converted != std::path::Path::new(original) {
        return Err(format!("expected roundtrip path to match, got: {}", converted.display()));
    }

    Ok(())
}
