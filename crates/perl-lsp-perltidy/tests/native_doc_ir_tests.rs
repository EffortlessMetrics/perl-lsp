use perl_lsp_perltidy::{FormatConfig, FormatDoc};

#[test]
fn format_doc_renders_flat_group_when_it_fits() {
    let doc = FormatDoc::group(vec![
        FormatDoc::text("my"),
        FormatDoc::Space,
        FormatDoc::text("$x"),
        FormatDoc::SoftLine,
        FormatDoc::text("="),
        FormatDoc::Space,
        FormatDoc::text("1;"),
    ]);

    let rendered = doc.render(&FormatConfig::default());

    assert_eq!(rendered, "my $x = 1;");
}

#[test]
fn format_doc_breaks_group_when_width_is_exceeded() {
    let config = FormatConfig { line_width: 8, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::text("my"),
        FormatDoc::Space,
        FormatDoc::text("$long_name"),
        FormatDoc::SoftLine,
        FormatDoc::text("="),
        FormatDoc::Space,
        FormatDoc::text("1;"),
    ]);

    let rendered = doc.render(&config);

    assert_eq!(rendered, "my $long_name\n= 1;");
}

#[test]
fn format_doc_accumulates_column_before_later_group_fit_check() {
    let config = FormatConfig { line_width: 9, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::text("my"),
        FormatDoc::Space,
        FormatDoc::group(vec![FormatDoc::text("$long"), FormatDoc::SoftLine, FormatDoc::text("=")]),
    ]);

    let rendered = doc.render(&config);

    assert_eq!(rendered, "my $long\n=");
}

#[test]
fn format_doc_indents_broken_nested_group() {
    let config = FormatConfig { line_width: 12, indent_width: 2, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::text("sub demo {"),
        FormatDoc::Indent(vec![
            FormatDoc::SoftLine,
            FormatDoc::text("return"),
            FormatDoc::Space,
            FormatDoc::text("1;"),
        ]),
        FormatDoc::SoftLine,
        FormatDoc::text("}"),
    ]);

    let rendered = doc.render(&config);

    assert_eq!(rendered, "sub demo {\n  return 1;\n}");
}

#[test]
fn format_doc_uses_tabs_when_configured() {
    let config = FormatConfig { line_width: 8, use_tabs: true, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::text("if ($x) {"),
        FormatDoc::Indent(vec![FormatDoc::SoftLine, FormatDoc::text("print $x;")]),
        FormatDoc::SoftLine,
        FormatDoc::text("}"),
    ]);

    let rendered = doc.render(&config);

    assert_eq!(rendered, "if ($x) {\n\tprint $x;\n}");
}

#[test]
fn format_doc_if_break_selects_flat_or_broken_branch() {
    let flat = FormatDoc::group(vec![
        FormatDoc::text("("),
        FormatDoc::if_break(FormatDoc::text(","), FormatDoc::text("")),
        FormatDoc::SoftLine,
        FormatDoc::text(")"),
    ]);
    let broken_config = FormatConfig { line_width: 1, ..FormatConfig::default() };

    assert_eq!(flat.render(&FormatConfig::default()), "( )");
    assert_eq!(flat.render(&broken_config), "(,\n)");
}

#[test]
fn format_doc_literal_preserve_keeps_multiline_region() {
    let doc = FormatDoc::group(vec![
        FormatDoc::text("print "),
        FormatDoc::literal_preserve("<<'EOF';\nraw { text }\nEOF"),
    ]);

    let rendered = doc.render(&FormatConfig::default());

    assert_eq!(rendered, "print <<'EOF';\nraw { text }\nEOF");
}
