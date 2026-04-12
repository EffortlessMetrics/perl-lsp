//! Comprehensive unit tests for perl-source-file crate.

use std::path::Path;

use perl_source_file::{
    PERL_SOURCE_EXTENSIONS, is_perl_source_extension, is_perl_source_path, is_perl_source_uri,
};

// ---------------------------------------------------------------------------
// PERL_SOURCE_EXTENSIONS constant
// ---------------------------------------------------------------------------

#[test]
fn extensions_constant_has_exactly_nine_entries() -> Result<(), String> {
    if PERL_SOURCE_EXTENSIONS.len() != 9 {
        return Err(format!("expected 9 extensions, got {}", PERL_SOURCE_EXTENSIONS.len()));
    }
    Ok(())
}

#[test]
fn extensions_constant_contains_expected_values() -> Result<(), String> {
    let expected = ["pl", "pm", "t", "psgi", "cgi", "ep", "tt", "tt2", "mason"];
    if PERL_SOURCE_EXTENSIONS != expected {
        return Err(format!("expected {expected:?}, got {:?}", PERL_SOURCE_EXTENSIONS));
    }
    Ok(())
}

#[test]
fn extensions_are_all_lowercase() -> Result<(), String> {
    for ext in &PERL_SOURCE_EXTENSIONS {
        let lower = ext.to_ascii_lowercase();
        if *ext != lower {
            return Err(format!("extension {ext:?} is not lowercase"));
        }
    }
    Ok(())
}

#[test]
fn extensions_contain_no_leading_dot() -> Result<(), String> {
    for ext in &PERL_SOURCE_EXTENSIONS {
        if ext.starts_with('.') {
            return Err(format!("extension {ext:?} has a leading dot"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_extension — positive cases
// ---------------------------------------------------------------------------

#[test]
fn extension_recognises_pl() -> Result<(), String> {
    if !is_perl_source_extension("pl") {
        return Err("pl should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_recognises_pm() -> Result<(), String> {
    if !is_perl_source_extension("pm") {
        return Err("pm should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_recognises_t() -> Result<(), String> {
    if !is_perl_source_extension("t") {
        return Err("t should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_recognises_psgi() -> Result<(), String> {
    if !is_perl_source_extension("psgi") {
        return Err("psgi should be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_extension — leading dot handling
// ---------------------------------------------------------------------------

#[test]
fn extension_with_leading_dot_pl() -> Result<(), String> {
    if !is_perl_source_extension(".pl") {
        return Err(".pl should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_with_leading_dot_pm() -> Result<(), String> {
    if !is_perl_source_extension(".pm") {
        return Err(".pm should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_with_leading_dot_t() -> Result<(), String> {
    if !is_perl_source_extension(".t") {
        return Err(".t should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_with_leading_dot_psgi() -> Result<(), String> {
    if !is_perl_source_extension(".psgi") {
        return Err(".psgi should be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_extension — case-insensitive matching
// ---------------------------------------------------------------------------

#[test]
fn extension_uppercase_pl() -> Result<(), String> {
    if !is_perl_source_extension("PL") {
        return Err("PL should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_uppercase_pm() -> Result<(), String> {
    if !is_perl_source_extension("PM") {
        return Err("PM should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_uppercase_t() -> Result<(), String> {
    if !is_perl_source_extension("T") {
        return Err("T should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_uppercase_psgi() -> Result<(), String> {
    if !is_perl_source_extension("PSGI") {
        return Err("PSGI should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_mixed_case_psgi() -> Result<(), String> {
    if !is_perl_source_extension("PsGi") {
        return Err("PsGi should be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_mixed_case_with_dot() -> Result<(), String> {
    if !is_perl_source_extension(".Pm") {
        return Err(".Pm should be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_extension — negative cases
// ---------------------------------------------------------------------------

#[test]
fn extension_rejects_empty_string() -> Result<(), String> {
    if is_perl_source_extension("") {
        return Err("empty string should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_dot_only() -> Result<(), String> {
    if is_perl_source_extension(".") {
        return Err("bare dot should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_txt() -> Result<(), String> {
    if is_perl_source_extension("txt") {
        return Err("txt should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_rs() -> Result<(), String> {
    if is_perl_source_extension("rs") {
        return Err("rs should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_py() -> Result<(), String> {
    if is_perl_source_extension("py") {
        return Err("py should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_md() -> Result<(), String> {
    if is_perl_source_extension("md") {
        return Err("md should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_perl_substring_p() -> Result<(), String> {
    if is_perl_source_extension("p") {
        return Err("p should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_perl_superstring_plx() -> Result<(), String> {
    if is_perl_source_extension("plx") {
        return Err("plx should not be recognized".into());
    }
    Ok(())
}

#[test]
fn extension_rejects_double_dot_prefix() -> Result<(), String> {
    if is_perl_source_extension("..pl") {
        return Err("..pl should not be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_path — Unix paths
// ---------------------------------------------------------------------------

#[test]
fn path_recognises_pl_script() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/home/user/script.pl")) {
        return Err("script.pl should be recognized".into());
    }
    Ok(())
}

#[test]
fn path_recognises_pm_module() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/lib/Foo/Bar.pm")) {
        return Err("Bar.pm should be recognized".into());
    }
    Ok(())
}

#[test]
fn path_recognises_test_file() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/t/00-basic.t")) {
        return Err("00-basic.t should be recognized".into());
    }
    Ok(())
}

#[test]
fn path_recognises_psgi_app() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/opt/app.psgi")) {
        return Err("app.psgi should be recognized".into());
    }
    Ok(())
}

#[test]
fn path_recognises_relative_path() -> Result<(), String> {
    if !is_perl_source_path(Path::new("lib/Foo.pm")) {
        return Err("relative lib/Foo.pm should be recognized".into());
    }
    Ok(())
}

#[test]
fn path_recognises_bare_filename() -> Result<(), String> {
    if !is_perl_source_path(Path::new("script.pl")) {
        return Err("bare script.pl should be recognized".into());
    }
    Ok(())
}

#[test]
fn path_recognises_uppercase_extension() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/lib/Module.PM")) {
        return Err("Module.PM should be recognized".into());
    }
    Ok(())
}

#[test]
fn path_recognises_deeply_nested() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/a/b/c/d/e/f/g.pm")) {
        return Err("deeply nested .pm should be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_path — negative cases
// ---------------------------------------------------------------------------

#[test]
fn path_rejects_no_extension() -> Result<(), String> {
    if is_perl_source_path(Path::new("/usr/bin/perl")) {
        return Err("file without extension should not be recognized".into());
    }
    Ok(())
}

#[test]
fn path_rejects_markdown() -> Result<(), String> {
    if is_perl_source_path(Path::new("/docs/README.md")) {
        return Err("README.md should not be recognized".into());
    }
    Ok(())
}

#[test]
fn path_rejects_rust_file() -> Result<(), String> {
    if is_perl_source_path(Path::new("/src/main.rs")) {
        return Err("main.rs should not be recognized".into());
    }
    Ok(())
}

#[test]
fn path_rejects_python_file() -> Result<(), String> {
    if is_perl_source_path(Path::new("/app/views.py")) {
        return Err("views.py should not be recognized".into());
    }
    Ok(())
}

#[test]
fn path_rejects_directory_looking_like_perl() -> Result<(), String> {
    // A path ending with a component that has no extension
    if is_perl_source_path(Path::new("/lib/Foo/")) {
        return Err("directory path should not be recognized".into());
    }
    Ok(())
}

#[test]
fn path_rejects_hidden_file_without_extension() -> Result<(), String> {
    if is_perl_source_path(Path::new("/home/.perlrc")) {
        return Err(".perlrc should not be recognized".into());
    }
    Ok(())
}

#[test]
fn path_rejects_empty_path() -> Result<(), String> {
    if is_perl_source_path(Path::new("")) {
        return Err("empty path should not be recognized".into());
    }
    Ok(())
}

#[test]
fn path_rejects_dotfile_with_non_perl_ext() -> Result<(), String> {
    if is_perl_source_path(Path::new("/home/.config.yml")) {
        return Err(".config.yml should not be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_uri — file:// URIs
// ---------------------------------------------------------------------------

#[test]
fn uri_recognises_file_uri_pl() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/script.pl") {
        return Err("file URI for .pl should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_recognises_file_uri_pm() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/lib/Foo.pm") {
        return Err("file URI for .pm should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_recognises_file_uri_t() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/t/01-basic.t") {
        return Err("file URI for .t should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_recognises_file_uri_psgi() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/app.psgi") {
        return Err("file URI for .psgi should be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_uri — query and fragment stripping
// ---------------------------------------------------------------------------

#[test]
fn uri_strips_query_string() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/script.pl?version=2") {
        return Err("URI with query should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_strips_fragment() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/script.pl#line=42") {
        return Err("URI with fragment should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_strips_query_and_fragment() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/app.psgi?v=1#section") {
        return Err("URI with query+fragment should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_strips_empty_query() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/script.pl?") {
        return Err("URI with empty query should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_strips_empty_fragment() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/script.pl#") {
        return Err("URI with empty fragment should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_strips_multiple_query_params() -> Result<(), String> {
    if !is_perl_source_uri("file:///workspace/script.pl?a=1&b=2&c=3") {
        return Err("URI with multiple query params should be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_uri — plain paths (non-URI)
// ---------------------------------------------------------------------------

#[test]
fn uri_recognises_plain_unix_path() -> Result<(), String> {
    if !is_perl_source_uri("/home/user/script.pl") {
        return Err("plain unix path should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_recognises_plain_relative_path() -> Result<(), String> {
    if !is_perl_source_uri("lib/Foo.pm") {
        return Err("plain relative path should be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_uri — Windows-style paths
// ---------------------------------------------------------------------------

#[test]
fn uri_recognises_windows_path() -> Result<(), String> {
    if !is_perl_source_uri(r"C:\Users\dev\script.pl") {
        return Err("Windows path should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_recognises_windows_file_uri() -> Result<(), String> {
    if !is_perl_source_uri("file:///C:/Users/dev/lib/Foo.pm") {
        return Err("Windows file URI should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_rejects_windows_non_perl_path() -> Result<(), String> {
    if is_perl_source_uri(r"C:\workspace\README.txt") {
        return Err("Windows .txt path should not be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// is_perl_source_uri — negative cases
// ---------------------------------------------------------------------------

#[test]
fn uri_rejects_non_perl_file_uri() -> Result<(), String> {
    if is_perl_source_uri("file:///workspace/README.md") {
        return Err("non-Perl file URI should not be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_rejects_empty_string() -> Result<(), String> {
    if is_perl_source_uri("") {
        return Err("empty string should not be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_rejects_bare_scheme() -> Result<(), String> {
    if is_perl_source_uri("file://") {
        return Err("bare file:// should not be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_rejects_non_perl_with_query() -> Result<(), String> {
    if is_perl_source_uri("file:///workspace/readme.txt?v=1") {
        return Err("non-Perl URI with query should not be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_rejects_no_extension_uri() -> Result<(), String> {
    if is_perl_source_uri("file:///usr/bin/perl") {
        return Err("URI without extension should not be recognized".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn extension_with_whitespace_is_rejected() -> Result<(), String> {
    if is_perl_source_extension(" pl") {
        return Err("extension with leading space should not match".into());
    }
    Ok(())
}

#[test]
fn extension_with_trailing_whitespace_is_rejected() -> Result<(), String> {
    if is_perl_source_extension("pl ") {
        return Err("extension with trailing space should not match".into());
    }
    Ok(())
}

#[test]
fn path_with_dots_in_directory_name() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/lib/Some.Module/script.pl")) {
        return Err("path with dots in directory should still match .pl".into());
    }
    Ok(())
}

#[test]
fn path_with_multiple_dots_in_filename() -> Result<(), String> {
    if !is_perl_source_path(Path::new("/lib/my.module.pm")) {
        return Err("multi-dot filename ending in .pm should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_with_encoded_spaces() -> Result<(), String> {
    if !is_perl_source_uri("file:///my%20workspace/script.pl") {
        return Err("URI with %20 encoding should be recognized".into());
    }
    Ok(())
}

#[test]
fn uri_fragment_before_query_still_strips_correctly() -> Result<(), String> {
    // Fragment before query is unusual but the implementation strips # first
    // then ?, so "script.pl#frag?q=1" -> "script.pl" after # strip -> recognized
    if !is_perl_source_uri("file:///workspace/script.pl#frag?q=1") {
        return Err("unusual fragment-before-query ordering should still work".into());
    }
    Ok(())
}
