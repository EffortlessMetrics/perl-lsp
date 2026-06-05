use super::LspServer;
use crate::protocol::JsonRpcError;
use perl_tdd_support::must_some;
use serde_json::json;

fn expect_hover_shape_guidance(err: JsonRpcError) -> Result<(), String> {
    assert_eq!(err.code, crate::protocol::INVALID_PARAMS);
    for expected in [
        "Missing required parameters: textDocument.uri and position",
        "textDocument/hover",
        "params.textDocument.uri",
        "params.position.line",
        "params.position.character",
        "file:///workspace/lib/My/Module.pm",
    ] {
        if !err.message.contains(expected) {
            return Err(format!("expected error message to contain {expected:?}; got {err}"));
        }
    }
    Ok(())
}

#[test]
fn hover_missing_params_error_includes_shape_guidance() -> Result<(), String> {
    let server = LspServer::new();
    match server.handle_hover(None) {
        Err(err) => expect_hover_shape_guidance(err),
        Ok(result) => Err(format!("expected INVALID_PARAMS; got {result:?}")),
    }
}

#[test]
fn hover_missing_position_error_includes_shape_guidance() -> Result<(), String> {
    let server = LspServer::new();
    match server.handle_hover(Some(json!({
        "textDocument": { "uri": "file:///workspace/lib/My/Module.pm" }
    }))) {
        Err(err) => expect_hover_shape_guidance(err),
        Ok(result) => Err(format!("expected INVALID_PARAMS; got {result:?}")),
    }
}

#[test]
fn hover_missing_uri_error_includes_shape_guidance() -> Result<(), String> {
    let server = LspServer::new();
    match server.handle_hover(Some(json!({
        "position": { "line": 10, "character": 4 }
    }))) {
        Err(err) => expect_hover_shape_guidance(err),
        Ok(result) => Err(format!("expected INVALID_PARAMS; got {result:?}")),
    }
}

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
}

#[test]
fn missing_module_search_paths_reports_empty_configuration() {
    let paths = LspServer::format_missing_module_search_paths(&[]);

    assert_eq!(paths, "- No include paths configured");
}
