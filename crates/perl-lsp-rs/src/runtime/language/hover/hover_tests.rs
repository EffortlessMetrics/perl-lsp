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
fn missing_module_search_paths_reports_empty_configuration() {
    let paths = LspServer::format_missing_module_search_paths(&[]);

    assert_eq!(paths, "- No include paths configured");
}
