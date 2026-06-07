use super::LspServer;
use perl_tdd_support::must_some;
use serde_json::json;

#[test]
fn test_internal_pl_sv_yes_hover_from_sigiled_token() {
    let text = "print $PL_sv_yes;\n";
    let offset = must_some(text.find('$'));

    assert_eq!(LspServer::extract_special_variable(text, offset).as_deref(), Some("$PL_sv_yes"));

    let hover = must_some(LspServer::get_special_variable_hover("$PL_sv_yes"));
    let value = must_some(hover["contents"]["value"].as_str());
    assert!(value.contains("true scalar"), "hover should describe the shared true scalar: {value}");
}

#[test]
fn pragma_hover_links_external_and_virtual_perldoc() {
    let hover = must_some(LspServer::build_pragma_hover("strict"));
    let value = must_some(hover["contents"]["value"].as_str());

    let expected = "**Pragma: `strict`**\n\n\
        _Enable strict variable/subroutine/reference checking_\n\n\
        Restricts unsafe Perl constructs. Enables compile-time errors for undeclared variables \
        (`vars`), bareword subroutine names (`subs`), and symbolic references (`refs`). Use \
        `use strict;` to enable all three categories at once, or `use strict 'vars'` for \
        individual categories.\n\n\
        **Common usage**: Always include `use strict;` at the top of every Perl file.\n\n\
        [perldoc strict](https://perldoc.perl.org/strict) | \
        [Open virtual perldoc](perldoc://strict)";
    assert_eq!(value, expected);
}

#[test]
fn pod_hover_cache_prunes_at_cap_and_evicts_active_document_path()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    let dir = tempfile::tempdir()?;

    for i in 0..1025 {
        let path = dir.path().join(format!("Cached{i}.pm"));
        std::fs::write(
            &path,
            format!(
                "package Cached{i};\n\n=head1 NAME\n\nCached{i}\n\n=head1 DESCRIPTION\n\nCached POD {i}.\n\n=cut\n\n1;\n"
            ),
        )?;

        let hover = server.format_pod_for_hover(&path);
        assert!(hover.contains("Cached POD"), "POD hover should parse {path:?}");
    }

    let after_prune = server.memory_state_snapshot();
    assert!(
        after_prune.pod_cache_entries <= 513,
        "1025 unique POD hovers should prune to target plus current insert, got {}",
        after_prune.pod_cache_entries
    );

    let active_path = dir.path().join("Active.pm");
    let active_text = "package Active;\n\n=head1 NAME\n\nActive\n\n=head1 DESCRIPTION\n\nActive POD.\n\n=cut\n\n1;\n";
    std::fs::write(&active_path, active_text)?;
    let active_uri =
        url::Url::from_file_path(&active_path).map_err(|_| "invalid active file path")?;
    let active_uri = active_uri.to_string();

    server.did_open(json!({
        "textDocument": {
            "uri": active_uri,
            "languageId": "perl",
            "version": 1,
            "text": active_text
        }
    }))?;

    let active_hover = server.format_pod_for_hover(&active_path);
    assert!(active_hover.contains("Active POD"), "active document POD should be cached");
    let with_active = server.memory_state_snapshot();
    assert_eq!(
        with_active.pod_cache_entries,
        after_prune.pod_cache_entries + 1,
        "active POD path should add exactly one cache entry"
    );

    server.handle_did_close(Some(json!({"textDocument": {"uri": active_uri}})))?;
    std::fs::remove_file(&active_path)?;
    server.handle_did_change_watched_files(Some(json!({
        "changes": [
            { "uri": active_uri, "type": 3 }
        ]
    })))?;

    let after_delete = server.memory_state_snapshot();
    assert_eq!(after_delete.documents, 0);
    assert_eq!(after_delete.open_text_bytes, 0);
    assert_eq!(
        after_delete.pod_cache_entries, after_prune.pod_cache_entries,
        "close/delete should evict the active document POD path entry"
    );

    Ok(())
}

#[test]
fn missing_module_hover_gives_actionable_next_steps() {
    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    {
        let mut config = server.workspace_config.lock();
        config.include_paths = vec!["lib".to_string(), "t/lib".to_string()];
        config.use_perl5lib = false;
        config.use_system_inc = false;
    }

    let hover = server.build_module_hover(
        "Definitely::Missing::Module",
        "use Definitely::Missing::Module;\n",
        "file:///tmp/missing.pl",
        Some(4),
    );
    let value = must_some(hover["contents"]["value"].as_str());

    assert!(
        value.contains("Not found in workspace or configured include paths"),
        "missing module hover should explain the failure scope: {value}"
    );
    let test_lib_display = std::path::Path::new("t").join("lib").display().to_string();
    let test_lib_line = format!("- `{test_lib_display}`");
    assert!(value.contains("- `lib`"), "missing module hover should list lib: {value}");
    assert!(value.contains(&test_lib_line), "missing module hover should list t/lib: {value}");
    assert!(
        value.contains("cpanm Definitely::Missing::Module"),
        "missing module hover should suggest an install command: {value}"
    );
    assert!(
        value.contains(".perl-lsp.toml` `include_paths`"),
        "missing module hover should point to include_paths configuration: {value}"
    );
    assert!(
        value.contains("https://metacpan.org/pod/Definitely::Missing::Module"),
        "missing module hover should keep the MetaCPAN link: {value}"
    );
    assert!(
        value.contains("perldoc://Definitely::Missing::Module"),
        "missing module hover should expose the virtual perldoc document: {value}"
    );
}

#[test]
fn resolved_module_hover_links_virtual_perldoc() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("workspace");
    let lib = root.join("lib").join("Local");
    std::fs::create_dir_all(&lib)?;
    std::fs::write(
        lib.join("Doc.pm"),
        "package Local::Doc;\n\n=head1 NAME\n\nLocal::Doc\n\n=head1 DESCRIPTION\n\nLocal POD.\n\n=cut\n\n1;\n",
    )?;

    let workspace_uri =
        url::Url::from_directory_path(&root).map_err(|_| "failed to create workspace URI")?;
    *server.workspace_folders.lock() = vec![
        crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri.to_string())
            .with_path(root.clone()),
    ];
    {
        let mut config = server.workspace_config.lock();
        config.include_paths = vec!["lib".to_string()];
        config.use_perl5lib = false;
        config.use_system_inc = false;
    }

    let script_uri = url::Url::from_file_path(root.join("main.pl"))
        .map_err(|_| "failed to create script URI")?;
    let doc_text = "use Local::Doc;\n";
    let hover = server.build_module_hover("Local::Doc", doc_text, script_uri.as_str(), Some(5));
    let value = must_some(hover["contents"]["value"].as_str());

    assert!(
        value.contains("[Go to module]("),
        "resolved hover should keep file navigation: {value}"
    );
    assert!(
        value.contains("https://metacpan.org/pod/Local::Doc"),
        "resolved hover should keep the MetaCPAN link: {value}"
    );
    assert!(
        value.contains("perldoc://Local::Doc"),
        "resolved hover should expose the virtual perldoc document: {value}"
    );
    assert!(value.contains("Local POD"), "resolved hover should keep local POD content: {value}");

    Ok(())
}

#[test]
fn require_module_hover_links_virtual_perldoc() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("require_hover.pl");
    let uri = url::Url::from_file_path(&script).map_err(|_| "failed to create script URI")?;
    let uri = uri.to_string();
    let text = "require Local::Doc;\n";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": text
        }
    }))?;

    let hover = must_some(server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 10 }
    })))?);
    let value = must_some(hover["contents"]["value"].as_str());

    assert!(
        value.contains("perldoc://Local::Doc"),
        "static require hover should expose the virtual perldoc document: {value}"
    );
    assert!(
        value.contains("https://metacpan.org/pod/Local::Doc"),
        "static require hover should keep the MetaCPAN link: {value}"
    );

    Ok(())
}

#[test]
fn require_module_scan_respects_static_module_token_boundaries() {
    let text = "require Local::Doc;\n";
    let token_start = must_some(text.find("Local"));
    let token_end = must_some(text.find(';'));
    let head = must_some(perl_module::import::parse_module_import_head(text));
    let span = must_some(perl_module::token_parser::parse_module_token(text, head.token_start));

    assert_eq!(head.kind, perl_module::import::ModuleImportKind::Require);
    assert_eq!(head.require_form(), Some(perl_module::import::RequireForm::ModuleName));
    assert_eq!(head.token_start, token_start);
    assert_eq!(head.token_end, token_end);
    assert_eq!(span.end, head.token_end);

    assert_eq!(LspServer::find_require_module_at_offset(text, token_start.saturating_sub(1)), None);
    assert_eq!(
        LspServer::find_require_module_at_offset(text, token_start).as_deref(),
        Some("Local::Doc")
    );
    assert_eq!(
        LspServer::find_require_module_at_offset(text, token_start + 2).as_deref(),
        Some("Local::Doc")
    );
    assert_eq!(
        LspServer::find_require_module_at_offset(text, token_end).as_deref(),
        Some("Local::Doc")
    );
    assert_eq!(LspServer::find_require_module_at_offset(text, token_end + 1), None);
}

#[test]
fn require_module_scan_rejects_non_require_and_non_module_require_forms() {
    assert_eq!(LspServer::find_require_module_at_offset("use Local::Doc;\n", 5), None);
    assert_eq!(LspServer::find_require_module_at_offset("require $module;\n", 10), None);
    assert_eq!(LspServer::find_require_module_at_offset("require 'Local/Doc.pm';\n", 10), None);
}

#[test]
fn require_module_scan_rejects_non_module_suffixes() {
    let text = "require Local::Doc-extra;\n";
    let token_offset = must_some(text.find("Local")) + 2;

    assert_eq!(LspServer::find_require_module_at_offset(text, token_offset), None);
}

#[test]
fn require_module_scan_has_explicit_boundary_discriminators() {
    let text = "require Local::Doc;\n";
    let head = must_some(perl_module::import::parse_module_import_head(text));
    let span = must_some(perl_module::token_parser::parse_module_token(text, 8));

    assert_eq!(head.kind, perl_module::import::ModuleImportKind::Require);
    assert_eq!(head.require_form(), Some(perl_module::import::RequireForm::ModuleName));
    assert_eq!(span.end, 18);
    assert_eq!(head.token_start, 8);
    assert_eq!(head.token_end, 18);
    assert_eq!(LspServer::find_require_module_at_offset(text, 7), None);
    assert_eq!(LspServer::find_require_module_at_offset(text, 8).as_deref(), Some("Local::Doc"));
    assert_eq!(LspServer::find_require_module_at_offset(text, 18).as_deref(), Some("Local::Doc"));
    assert_eq!(LspServer::find_require_module_at_offset(text, 19), None);
}

#[test]
fn require_module_boundary_predicates_are_explicit() {
    assert!(LspServer::is_static_require_module(
        perl_module::import::ModuleImportKind::Require,
        Some(perl_module::import::RequireForm::ModuleName)
    ));
    assert!(!LspServer::is_static_require_module(perl_module::import::ModuleImportKind::Use, None));
    assert!(!LspServer::is_static_require_module(
        perl_module::import::ModuleImportKind::Require,
        Some(perl_module::import::RequireForm::FilePath)
    ));

    assert!(!LspServer::cursor_spans_module_token(7, 8, 18));
    assert!(LspServer::cursor_spans_module_token(8, 8, 18));
    assert!(LspServer::cursor_spans_module_token(18, 8, 18));
    assert!(!LspServer::cursor_spans_module_token(19, 8, 18));

    assert!(LspServer::module_token_span_matches_head(18, 18));
    assert!(!LspServer::module_token_span_matches_head(10, 18));
}

#[test]
fn require_module_scan_normalizes_utf8_offsets() {
    let text = "é\nrequire Local::Doc;\n";

    assert_eq!(LspServer::normalize_hover_text_offset(text, 1), 0);
}

#[test]
fn missing_module_search_paths_reports_empty_configuration() {
    let paths = LspServer::format_missing_module_search_paths(&[]);

    assert_eq!(paths, "- No include paths configured");
}

#[test]
fn hover_token_extraction_works_with_non_ascii_prefix() {
    // "# café\n" — 'é' (U+00E9) is 2 UTF-8 bytes; line is 9 bytes, 8 chars.
    // Byte offset of '$' on line 2: "# café\nmy $bar = 2;" -> find('$') = 12.
    // Bug: using byte offset 12 as char index into Vec<char> would yield the wrong character.
    let text = "# café\nmy $bar = 2;";
    let dollar_offset = must_some(text.find('$'));
    let token = LspServer::get_token_at_position_static(text, dollar_offset);
    assert_eq!(
        token, "$bar",
        "byte offset must not be used as char index in hover token extraction"
    );
}
