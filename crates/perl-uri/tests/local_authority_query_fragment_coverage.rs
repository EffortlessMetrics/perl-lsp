use perl_uri::{is_special_scheme, normalize_uri, uri_extension, uri_key};
use url::Url;

#[cfg(windows)]
use perl_uri::uri_to_fs_path;

#[test]
fn local_file_authority_predicate_inputs_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse("file://127.0.0.1/tmp/module.pm?rev=42#L10")?;
    assert_eq!(url.scheme(), "file");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(normalize_uri(url.as_str()), "file:///tmp/module.pm?rev=42#L10");

    let url = Url::parse("file://example.com/tmp/module.pm?rev=42#L10")?;
    assert_eq!(url.scheme(), "file");
    assert_eq!(url.host_str(), Some("example.com"));
    assert_eq!(normalize_uri(url.as_str()), url.as_str());

    let url = Url::parse("https://localhost/tmp/module.pm?rev=42#L10")?;
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("localhost"));
    assert_eq!(normalize_uri(url.as_str()), url.as_str());

    Ok(())
}

#[test]
fn localhost_file_authority_preserves_query_and_fragment() -> Result<(), Box<dyn std::error::Error>>
{
    assert_eq!(
        uri_key("file://localhost/tmp/module.pm?rev=42#L10"),
        "file:///tmp/module.pm?rev=42#L10"
    );
    Ok(())
}

#[test]
fn loopback_file_authorities_preserve_query_and_fragment() -> Result<(), Box<dyn std::error::Error>>
{
    assert_eq!(
        uri_key("file://127.0.0.1/tmp/script.pl?debug=1#main"),
        "file:///tmp/script.pl?debug=1#main"
    );
    assert_eq!(
        uri_key("file://[::1]/tmp/script.pl?debug=1#main"),
        "file:///tmp/script.pl?debug=1#main"
    );
    Ok(())
}

#[test]
fn normalize_uri_loopback_authority_preserves_query_and_fragment()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_uri("file://127.0.0.1/tmp/module.pm?rev=42#L10"),
        "file:///tmp/module.pm?rev=42#L10"
    );
    assert_eq!(
        normalize_uri("file://[::1]/tmp/module.pm?rev=42#L10"),
        "file:///tmp/module.pm?rev=42#L10"
    );
    Ok(())
}

#[test]
fn normalize_uri_local_file_authority_predicate_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_uri("file://localhost/tmp/module.pm?rev=42#L10"),
        "file:///tmp/module.pm?rev=42#L10"
    );
    assert_eq!(
        normalize_uri("file://example.com/tmp/module.pm?rev=42#L10"),
        "file://example.com/tmp/module.pm?rev=42#L10"
    );
    assert_eq!(
        normalize_uri("https://localhost/tmp/module.pm?rev=42#L10"),
        "https://localhost/tmp/module.pm?rev=42#L10"
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn uri_to_fs_path_accepts_bare_windows_drive_path() -> Result<(), Box<dyn std::error::Error>> {
    let path = uri_to_fs_path(r"C:\Users\dev\module.pm").ok_or("expected Some")?;
    assert!(path.to_string_lossy().ends_with("module.pm"));
    Ok(())
}

#[test]
fn canonical_drive_uri_with_pipe_separator_preserves_suffix()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        uri_key("file:///Z|/Work/Module.pm?rev=42#L10"),
        "file:///z:/Work/Module.pm?rev=42#L10"
    );
    Ok(())
}

#[test]
fn invalid_special_scheme_prefixes_must_match_full_prefix() -> Result<(), Box<dyn std::error::Error>>
{
    assert!(!is_special_scheme("untitle invalid uri"));
    assert!(!is_special_scheme("vscode-notebook-cel invalid uri"));
    assert!(!is_special_scheme("vscode-vf invalid uri"));
    Ok(())
}

#[test]
fn extension_stops_at_earliest_query_or_fragment_separator()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(uri_extension("file:///tmp/module.pm#L10?ignored=.pl"), Some("pm"));
    assert_eq!(uri_extension("file:///tmp/module.pm?rev=42#ignored=.pl"), Some("pm"));
    Ok(())
}
