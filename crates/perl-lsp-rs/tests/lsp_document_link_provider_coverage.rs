use lsp_types::Position;
use perl_lsp::features::lsp_document_link::collect_document_links;
use url::Url;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn document_link_ranges_remain_correct_after_crlf_prefix() -> TestResult {
    let uri = Url::parse("file:///workspace/main.pl")?;
    let text = "# before\r\nuse Foo::Bar;\r\nrequire Baz::Qux;\r\n";

    let links = collect_document_links(text, &uri)?;
    let foo = links
        .iter()
        .find(|link| link.tooltip.as_deref() == Some("Open Foo::Bar on MetaCPAN"))
        .ok_or("missing Foo::Bar document link")?;
    let baz = links
        .iter()
        .find(|link| link.tooltip.as_deref() == Some("Open Baz::Qux on MetaCPAN"))
        .ok_or("missing Baz::Qux document link")?;

    assert_eq!(foo.range.start, Position::new(1, 4));
    assert_eq!(foo.range.end, Position::new(1, 12));
    assert_eq!(baz.range.start, Position::new(2, 8));
    assert_eq!(baz.range.end, Position::new(2, 16));
    Ok(())
}

#[test]
fn quoted_file_document_link_resolves_relative_to_current_file() -> TestResult {
    let uri = Url::from_file_path(
        std::env::temp_dir().join("perl-lsp-document-link").join("bin").join("app.pl"),
    )
    .map_err(|()| "failed to build platform file URI")?;
    let text = "use Foo::Bar;\nrequire 'lib/Local.pm';\ndo \"script.pl\";\n";

    let links = collect_document_links(text, &uri)?;
    let require_link = links
        .iter()
        .find(|link| link.range.start == Position::new(1, 9))
        .ok_or("missing require file link")?;
    let do_link = links
        .iter()
        .find(|link| link.range.start == Position::new(2, 4))
        .ok_or("missing do file link")?;

    let require_target = require_link.target.as_ref().ok_or("require link missing target")?;
    let do_target = do_link.target.as_ref().ok_or("do link missing target")?;

    assert!(
        require_target.as_str().ends_with("/bin/lib/Local.pm"),
        "unexpected require target: {require_target:?}"
    );
    assert!(do_target.as_str().ends_with("/bin/script.pl"), "unexpected do target: {do_target:?}");
    assert_eq!(require_link.range.end, Position::new(1, 21));
    assert_eq!(do_link.range.end, Position::new(2, 13));
    Ok(())
}
