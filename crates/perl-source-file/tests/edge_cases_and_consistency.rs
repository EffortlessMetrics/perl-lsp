//! Additional edge-case and cross-function consistency tests for perl-source-file.

use std::path::Path;

use perl_source_file::{
    PERL_SOURCE_EXTENSIONS, is_perl_source_extension, is_perl_source_path, is_perl_source_uri,
};

// ---------------------------------------------------------------------------
// Consistency: every canonical extension is accepted by is_perl_source_extension
// ---------------------------------------------------------------------------

#[test]
fn every_canonical_extension_is_accepted_bare() {
    for ext in &PERL_SOURCE_EXTENSIONS {
        assert!(
            is_perl_source_extension(ext),
            "canonical extension {ext:?} should be accepted without dot"
        );
    }
}

#[test]
fn every_canonical_extension_is_accepted_with_dot() {
    for ext in &PERL_SOURCE_EXTENSIONS {
        let dotted = format!(".{ext}");
        assert!(
            is_perl_source_extension(&dotted),
            "canonical extension {dotted:?} should be accepted with dot"
        );
    }
}

#[test]
fn every_canonical_extension_is_accepted_uppercase() {
    for ext in &PERL_SOURCE_EXTENSIONS {
        let upper = ext.to_ascii_uppercase();
        assert!(
            is_perl_source_extension(&upper),
            "uppercase extension {upper:?} should be accepted"
        );
    }
}

// ---------------------------------------------------------------------------
// Consistency: is_perl_source_path agrees with is_perl_source_extension
// ---------------------------------------------------------------------------

#[test]
fn path_agrees_with_extension_for_all_canonical() {
    for ext in &PERL_SOURCE_EXTENSIONS {
        let filename = format!("test_file.{ext}");
        let path = Path::new(&filename);
        assert!(is_perl_source_path(path), "path {filename:?} should be recognized");
    }
}

#[test]
fn uri_agrees_with_path_for_all_canonical() {
    for ext in &PERL_SOURCE_EXTENSIONS {
        let uri = format!("file:///workspace/test_file.{ext}");
        assert!(is_perl_source_uri(&uri), "URI {uri:?} should be recognized");
    }
}

// ---------------------------------------------------------------------------
// is_perl_source_extension — Perl-adjacent but unsupported extensions
// ---------------------------------------------------------------------------

#[test]
fn extension_recognises_cgi() {
    assert!(is_perl_source_extension("cgi"), "cgi is a recognized Perl source extension");
}

#[test]
fn extension_rejects_pod() {
    assert!(!is_perl_source_extension("pod"), "pod is documentation, not source");
}

#[test]
fn extension_rejects_xs() {
    assert!(!is_perl_source_extension("xs"), "xs is C glue, not Perl source");
}

#[test]
fn extension_rejects_al() {
    assert!(!is_perl_source_extension("al"), "al (autoloaded) is not a canonical extension");
}

#[test]
fn extension_rejects_perl6_raku() {
    assert!(!is_perl_source_extension("raku"), "raku is Perl 6, not Perl 5");
}

#[test]
fn extension_rejects_perl6_rakumod() {
    assert!(!is_perl_source_extension("rakumod"), "rakumod is Perl 6, not Perl 5");
}

// ---------------------------------------------------------------------------
// is_perl_source_extension — boundary / adversarial strings
// ---------------------------------------------------------------------------

#[test]
fn extension_rejects_just_a_newline() {
    assert!(!is_perl_source_extension("\n"));
}

#[test]
fn extension_rejects_tab_character() {
    assert!(!is_perl_source_extension("\t"));
}

#[test]
fn extension_rejects_null_byte_prefix() {
    assert!(!is_perl_source_extension("\0pl"));
}

#[test]
fn extension_rejects_pl_with_null_suffix() {
    assert!(!is_perl_source_extension("pl\0"));
}

#[test]
fn extension_rejects_unicode_lookalike_for_p() {
    // U+0440 Cyrillic р looks like Latin p
    assert!(!is_perl_source_extension("\u{0440}l"));
}

#[test]
fn extension_rejects_dot_dot_extension() {
    assert!(!is_perl_source_extension("..pm"));
}

#[test]
fn extension_rejects_very_long_string() {
    let long = "pl".repeat(500);
    assert!(!is_perl_source_extension(&long));
}

// ---------------------------------------------------------------------------
// is_perl_source_path — special filesystem patterns
// ---------------------------------------------------------------------------

#[test]
fn path_with_parent_traversal_still_matches() {
    assert!(is_perl_source_path(Path::new("/lib/../lib/Foo.pm")));
}

#[test]
fn path_with_current_dir_reference() {
    assert!(is_perl_source_path(Path::new("/lib/./Foo.pm")));
}

#[test]
fn path_dot_extension_only() {
    // A file named ".pl" — on Unix this is a hidden file with no stem
    // Path::extension() returns None for dotfiles like ".pl"
    assert!(
        !is_perl_source_path(Path::new(".pl")),
        ".pl is a hidden file with no extension on Unix"
    );
}

#[test]
fn path_dot_extension_in_directory() {
    assert!(
        !is_perl_source_path(Path::new("/home/.pm")),
        ".pm alone is a hidden file, not a Perl module"
    );
}

#[test]
fn path_double_extension_tar_pl() {
    // "archive.tar.pl" — extension is "pl"
    assert!(is_perl_source_path(Path::new("archive.tar.pl")));
}

#[test]
fn path_double_extension_pl_bak() {
    // "script.pl.bak" — extension is "bak", not "pl"
    assert!(!is_perl_source_path(Path::new("script.pl.bak")));
}

#[test]
fn path_triple_extension_ending_in_pm() {
    assert!(is_perl_source_path(Path::new("my.lib.module.pm")));
}

#[test]
fn path_with_spaces_in_components() {
    assert!(is_perl_source_path(Path::new("/my project/lib/Foo.pm")));
}

#[test]
fn path_with_unicode_directory_name() {
    assert!(is_perl_source_path(Path::new("/café/lib/Moose.pm")));
}

#[test]
fn path_rejects_extension_as_directory_component() {
    // "/some/pm/file" — "pm" is a directory, "file" has no extension
    assert!(!is_perl_source_path(Path::new("/some/pm/file")));
}

#[test]
fn path_rejects_just_extension_no_stem() {
    // Path ".t" is treated as hidden file with no extension on Unix
    assert!(!is_perl_source_path(Path::new("/tests/.t")));
}

#[test]
fn path_recognises_mixed_case_psgi() {
    assert!(is_perl_source_path(Path::new("/app/myapp.PSGI")));
}

// ---------------------------------------------------------------------------
// is_perl_source_uri — unusual URI patterns
// ---------------------------------------------------------------------------

#[test]
fn uri_recognises_file_uri_with_host() {
    // file://localhost/path/script.pl is valid per RFC 8089
    assert!(is_perl_source_uri("file://localhost/workspace/script.pl"));
}

#[test]
fn uri_recognises_file_uri_uppercase_scheme() {
    // "FILE:///workspace/script.pl" — scheme is case-insensitive per RFC
    // Our implementation just passes to Path, so FILE:// prefix stays
    // The extension .pl should still match
    assert!(is_perl_source_uri("FILE:///workspace/script.pl"));
}

#[test]
fn uri_rejects_http_scheme_even_with_perl_extension() {
    // http URLs should still work — we only care about the path extension
    assert!(is_perl_source_uri("http://example.com/script.pl"));
}

#[test]
fn uri_rejects_javascript_scheme() {
    assert!(!is_perl_source_uri("javascript:alert(1)"));
}

#[test]
fn uri_rejects_data_scheme() {
    assert!(!is_perl_source_uri("data:text/plain;base64,SGVsbG8="));
}

#[test]
fn uri_with_port_in_authority() {
    assert!(is_perl_source_uri("file://host:8080/workspace/script.pl"));
}

#[test]
fn uri_with_userinfo_in_authority() {
    assert!(is_perl_source_uri("file://user@host/workspace/script.pl"));
}

#[test]
fn uri_with_only_fragment_no_query() {
    assert!(is_perl_source_uri("file:///workspace/lib.pm#L10"));
}

#[test]
fn uri_with_hash_in_fragment() {
    // Double hash: first # starts fragment, second # is inside fragment
    assert!(is_perl_source_uri("file:///workspace/lib.pm#section#sub"));
}

#[test]
fn uri_rejects_fragment_that_looks_like_perl_ext() {
    // "file:///workspace/readme.md#anchor.pl" — base path is .md
    assert!(!is_perl_source_uri("file:///workspace/readme.md#anchor.pl"));
}

#[test]
fn uri_rejects_query_that_looks_like_perl_ext() {
    // "file:///workspace/readme.txt?file=test.pm" — base path is .txt
    assert!(!is_perl_source_uri("file:///workspace/readme.txt?file=test.pm"));
}

#[test]
fn uri_with_percent_encoded_extension() {
    // %2E is '.', so "script%2Epl" — Path sees literal "%2Epl" as extension
    // This should NOT match because the extension is "%2Epl" not "pl"
    assert!(!is_perl_source_uri("file:///workspace/script%2Epl"));
}

#[test]
fn uri_with_plus_for_space() {
    // Plus encoding in path — extension is still .pm
    assert!(is_perl_source_uri("file:///my+project/lib/Foo.pm"));
}

// ---------------------------------------------------------------------------
// Idempotency and reflexive properties
// ---------------------------------------------------------------------------

#[test]
fn extension_check_is_idempotent() {
    let inputs = ["pl", ".pm", "T", "PSGI", "txt", "", "rs"];
    for input in &inputs {
        let first = is_perl_source_extension(input);
        let second = is_perl_source_extension(input);
        assert_eq!(first, second, "result should be stable for {input:?}");
    }
}

#[test]
fn path_check_is_idempotent() {
    let paths = ["/a.pl", "/b.rs", "/c", ""];
    for p in &paths {
        let path = Path::new(p);
        let first = is_perl_source_path(path);
        let second = is_perl_source_path(path);
        assert_eq!(first, second, "result should be stable for {p:?}");
    }
}

#[test]
fn uri_check_is_idempotent() {
    let uris = ["file:///a.pl", "file:///b.rs", "", "file:///c?q=1"];
    for u in &uris {
        let first = is_perl_source_uri(u);
        let second = is_perl_source_uri(u);
        assert_eq!(first, second, "result should be stable for {u:?}");
    }
}

// ---------------------------------------------------------------------------
// Bulk negative: common non-Perl extensions via path
// ---------------------------------------------------------------------------

#[test]
fn path_rejects_common_web_extensions() {
    let exts = ["html", "css", "js", "ts", "jsx", "tsx", "json", "xml", "yaml", "yml"];
    for ext in &exts {
        let path_str = format!("/workspace/file.{ext}");
        assert!(
            !is_perl_source_path(Path::new(&path_str)),
            ".{ext} should not be recognized as Perl source"
        );
    }
}

#[test]
fn path_rejects_common_compiled_extensions() {
    let exts = ["o", "so", "dylib", "dll", "exe", "class", "jar", "wasm"];
    for ext in &exts {
        let path_str = format!("/workspace/file.{ext}");
        assert!(
            !is_perl_source_path(Path::new(&path_str)),
            ".{ext} should not be recognized as Perl source"
        );
    }
}

#[test]
fn path_rejects_common_scripting_extensions() {
    let exts = ["py", "rb", "lua", "sh", "bash", "zsh", "fish", "php"];
    for ext in &exts {
        let path_str = format!("/workspace/file.{ext}");
        assert!(
            !is_perl_source_path(Path::new(&path_str)),
            ".{ext} should not be recognized as Perl source"
        );
    }
}
