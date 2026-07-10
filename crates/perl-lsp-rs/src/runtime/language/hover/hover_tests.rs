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
fn pod_hover_cache_refreshes_after_external_file_edit() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("ExternalEdit.pm");

    std::fs::write(
        &path,
        "package ExternalEdit;\n\n=head1 NAME\n\nExternalEdit\n\n=head1 DESCRIPTION\n\nOriginal POD.\n\n=cut\n\n1;\n",
    )?;

    let first_hover = server.format_pod_for_hover(&path);
    assert!(
        first_hover.contains("Original POD"),
        "initial POD hover should be cached: {first_hover}"
    );
    let cached_hover = server.format_pod_for_hover(&path);
    assert_eq!(cached_hover, first_hover, "unchanged POD hover should use the cached document");

    write_after_mtime_tick(
        &path,
        "package ExternalEdit;\n\n=head1 NAME\n\nExternalEdit\n\n=head1 DESCRIPTION\n\nUpdated POD.\n\n=cut\n\n1;\n",
    )?;

    let updated_hover = server.format_pod_for_hover(&path);
    assert!(
        updated_hover.contains("Updated POD"),
        "POD hover should refresh after file mtime changes: {updated_hover}"
    );
    assert!(
        !updated_hover.contains("Original POD"),
        "stale cached POD should not remain after external file edit: {updated_hover}"
    );

    Ok(())
}

fn write_after_mtime_tick(
    path: &std::path::Path,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let before_modified = std::fs::metadata(path)?.modified()?;

    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(path, contents)?;
        if std::fs::metadata(path)?.modified()? != before_modified {
            return Ok(());
        }
    }

    Err("file mtime did not change after rewrite".into())
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
fn missing_module_hover_mentions_declared_unindexed_dependency() {
    use perl_lsp_rs_core::config::{DeclaredDependency, DeclaredDependencySource, WorkspaceConfig};

    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    let mut config = WorkspaceConfig::default();
    config.use_perl5lib = false;
    config.use_system_inc = false;
    config.declared_dependencies = vec![DeclaredDependency::new(
        "JSON::PP",
        Some("4.16"),
        "requires",
        DeclaredDependencySource::Cpanfile,
    )];
    *server.workspace_folders.lock() = vec![
        crate::runtime::workspace_folder::WorkspaceFolderState::new(
            "file:///workspace".to_string(),
        )
        .with_effective_workspace_config(config),
    ];

    let hover = server.build_module_hover(
        "JSON::PP",
        "use JSON::PP;\n",
        "file:///workspace/main.pl",
        Some(4),
    );
    let value = must_some(hover["contents"]["value"].as_str());

    assert!(
        value.contains("declared in cpanfile"),
        "missing module hover should explain metadata declaration source: {value}"
    );
    assert!(
        value.contains("not currently indexed"),
        "missing module hover should distinguish declared-but-unindexed modules: {value}"
    );
    assert!(
        value.contains("requires 4.16"),
        "missing module hover should include declared dependency kind and version: {value}"
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

// -- Phase-block hover: strong-oracle unit tests ----------------------------------
//
// These tests assert exact content for every match arm in `phase_block_description`
// and exact Option discriminants for every code path in `find_phase_block_at_offset`,
// so that ripr static analysis can confirm each seam has an oracle-killing test.
// The existing integration tests in hover_provider_coverage.rs use broad `contains`
// checks; these unit tests provide the precise assertions the gap gate requires.

#[test]
fn phase_block_hover_begin_returns_compile_time_timing() {
    let hover = must_some(super::hover_cards::phase_block_hover("BEGIN"));
    let value = must_some(hover["contents"]["value"].as_str());
    assert_eq!(
        hover["contents"]["kind"].as_str(),
        Some("markdown"),
        "phase block hover must be markdown kind"
    );
    assert!(
        value.starts_with("**Phase Block: `BEGIN`**"),
        "BEGIN hover must open with the phase block header: {value}"
    );
    assert!(
        value.contains("_Compile-time execution_"),
        "BEGIN hover must contain exact timing label: {value}"
    );
    assert!(
        value.contains("as soon as the block is fully parsed"),
        "BEGIN hover must mention compile-time parse order: {value}"
    );
    assert!(value.contains("FIFO"), "BEGIN hover must mention FIFO ordering: {value}");
    assert!(value.contains("perlmod"), "BEGIN hover must link to perlmod: {value}");
}

#[test]
fn phase_block_hover_end_returns_program_exit_timing() {
    let hover = must_some(super::hover_cards::phase_block_hover("END"));
    let value = must_some(hover["contents"]["value"].as_str());
    assert!(
        value.starts_with("**Phase Block: `END`**"),
        "END hover must open with the phase block header: {value}"
    );
    assert!(
        value.contains("_Program-exit cleanup_"),
        "END hover must contain exact timing label: {value}"
    );
    assert!(value.contains("program exit"), "END hover must mention program exit: {value}");
    assert!(value.contains("LIFO"), "END hover must mention LIFO ordering: {value}");
}

#[test]
fn phase_block_hover_init_returns_post_compile_timing() {
    let hover = must_some(super::hover_cards::phase_block_hover("INIT"));
    let value = must_some(hover["contents"]["value"].as_str());
    assert!(
        value.starts_with("**Phase Block: `INIT`**"),
        "INIT hover must open with the phase block header: {value}"
    );
    assert!(
        value.contains("_Post-compile, pre-runtime startup_"),
        "INIT hover must contain exact timing label: {value}"
    );
    assert!(
        value.contains("start of runtime"),
        "INIT hover must mention start of runtime: {value}"
    );
    assert!(value.contains("FIFO"), "INIT hover must mention FIFO ordering: {value}");
}

#[test]
fn phase_block_hover_check_returns_end_of_compilation_timing() {
    let hover = must_some(super::hover_cards::phase_block_hover("CHECK"));
    let value = must_some(hover["contents"]["value"].as_str());
    assert!(
        value.starts_with("**Phase Block: `CHECK`**"),
        "CHECK hover must open with the phase block header: {value}"
    );
    assert!(
        value.contains("_End-of-compilation hook_"),
        "CHECK hover must contain exact timing label: {value}"
    );
    assert!(
        value.contains("end of compilation"),
        "CHECK hover must mention end of compilation: {value}"
    );
    assert!(value.contains("LIFO"), "CHECK hover must mention LIFO ordering: {value}");
}

#[test]
fn phase_block_hover_unitcheck_returns_compilation_unit_timing() {
    let hover = must_some(super::hover_cards::phase_block_hover("UNITCHECK"));
    let value = must_some(hover["contents"]["value"].as_str());
    assert!(
        value.starts_with("**Phase Block: `UNITCHECK`**"),
        "UNITCHECK hover must open with the phase block header: {value}"
    );
    assert!(
        value.contains("_End-of-compilation-unit hook_"),
        "UNITCHECK hover must contain exact timing label: {value}"
    );
    assert!(
        value.contains("compilation unit"),
        "UNITCHECK hover must mention compilation unit: {value}"
    );
    assert!(value.contains("LIFO"), "UNITCHECK hover must mention LIFO ordering: {value}");
}

#[test]
fn phase_block_hover_unknown_returns_none() {
    assert_eq!(
        super::hover_cards::phase_block_hover("UNKNOWN"),
        None,
        "unrecognised phase name must return None"
    );
    assert_eq!(super::hover_cards::phase_block_hover(""), None, "empty string must return None");
    assert_eq!(
        super::hover_cards::phase_block_hover("begin"),
        None,
        "lowercase phase name must return None (case-sensitive match)"
    );
}

#[test]
fn find_phase_block_at_offset_returns_none_when_offset_out_of_node_range() {
    use perl_parser::{Node, NodeKind, SourceLocation};

    // A PhaseBlock node spanning [10, 30].
    let block =
        Node::new(NodeKind::Block { statements: vec![] }, SourceLocation { start: 11, end: 29 });
    let node = Node::new(
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: None,
            block: Box::new(block),
        },
        SourceLocation { start: 10, end: 30 },
    );

    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 9),
        None,
        "offset before node span must return None"
    );
    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 31),
        None,
        "offset after node span must return None"
    );
}

#[test]
fn find_phase_block_at_offset_returns_phase_name_when_offset_in_node_and_no_phase_span() {
    use perl_parser::{Node, NodeKind, SourceLocation};

    // No phase_span: any offset within [10, 30] must return the phase name.
    let block =
        Node::new(NodeKind::Block { statements: vec![] }, SourceLocation { start: 11, end: 29 });
    let node = Node::new(
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: None,
            block: Box::new(block),
        },
        SourceLocation { start: 10, end: 30 },
    );

    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 10).as_deref(),
        Some("BEGIN"),
        "offset at node start must return phase name when no phase_span"
    );
    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 20).as_deref(),
        Some("BEGIN"),
        "offset in node middle must return phase name when no phase_span"
    );
    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 30).as_deref(),
        Some("BEGIN"),
        "offset at node end must return phase name when no phase_span"
    );
}

#[test]
fn find_phase_block_at_offset_respects_phase_span_boundary() {
    use perl_parser::{Node, NodeKind, SourceLocation};

    // phase_span = [10, 14] (just "BEGIN"), whole node = [10, 30].
    let block =
        Node::new(NodeKind::Block { statements: vec![] }, SourceLocation { start: 16, end: 29 });
    let node = Node::new(
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: Some(SourceLocation { start: 10, end: 14 }),
            block: Box::new(block),
        },
        SourceLocation { start: 10, end: 30 },
    );

    // Inside phase_span: returns Some.
    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 10).as_deref(),
        Some("BEGIN"),
        "offset at phase_span start must return phase name"
    );
    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 14).as_deref(),
        Some("BEGIN"),
        "offset at phase_span end must return phase name"
    );
    // Outside phase_span but inside node: must return None (phase_span present, not matched).
    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 15),
        None,
        "offset after phase_span end (but inside node) must return None when phase_span present"
    );
    assert_eq!(
        LspServer::find_phase_block_at_offset(&node, 20),
        None,
        "offset in block area must return None when phase_span present and not matched"
    );
}

#[test]
fn find_phase_block_at_offset_recurses_through_program_to_find_phase_block() {
    use perl_parser::{Node, NodeKind, SourceLocation};

    // Program { statements: [PhaseBlock { "END", [40,50] }] } spanning [0, 60].
    let block_inner =
        Node::new(NodeKind::Block { statements: vec![] }, SourceLocation { start: 45, end: 49 });
    let phase_node = Node::new(
        NodeKind::PhaseBlock {
            phase: "END".to_string(),
            phase_span: None,
            block: Box::new(block_inner),
        },
        SourceLocation { start: 40, end: 50 },
    );
    let program = Node::new(
        NodeKind::Program { statements: vec![phase_node] },
        SourceLocation { start: 0, end: 60 },
    );

    // Offset inside the nested phase block: must find it.
    assert_eq!(
        LspServer::find_phase_block_at_offset(&program, 45).as_deref(),
        Some("END"),
        "recursion through Program must find nested PhaseBlock"
    );
    // Offset outside the nested phase block but inside program: must return None.
    assert_eq!(
        LspServer::find_phase_block_at_offset(&program, 35),
        None,
        "offset not in any PhaseBlock must return None even when inside Program"
    );
}

#[test]
fn hover_documentation_with_markdown_chars_is_escaped() -> Result<(), Box<dyn std::error::Error>> {
    // Test that documentation containing markdown special characters is properly escaped
    // so they render as literal text, not as markdown formatting.
    let text = r#"
# This variable tracks *important* data [see docs]
my $var = 42;
"#;

    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    let uri = "file:///test.pl".to_string();
    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": text
        }
    }))?;

    // Get hover at $var position (line 2, character 4 = inside '$var')
    let hover = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 2, "character": 4 }
    })))?;

    let hover = must_some(hover);
    let value = must_some(hover["contents"]["value"].as_str());

    // The documentation should escape the asterisks and brackets
    assert!(value.contains(r"\*important\*"), "markdown asterisks should be escaped: {}", value);
    assert!(value.contains(r"\[see docs\]"), "markdown brackets should be escaped: {}", value);
    assert!(
        !value.contains("*important*"),
        "unescaped asterisks should not be present (they would make bold text): {}",
        value
    );
    assert!(
        !value.contains("[see docs]"),
        "unescaped brackets should not be present (they would be treated as links): {}",
        value
    );

    Ok(())
}

#[test]
fn method_modifier_hover_escapes_doc_markdown() {
    // Verify that method modifier hover cards escape markdown in the user-supplied
    // documentation string, while preserving intentional markdown in the hardcoded
    // kind_label (e.g. the "runs **before** the method" descriptions).
    let hover = super::hover_cards::method_modifier_hover(
        "before",
        "validate_input",
        "Checks that *all* args are [valid] before calling the real method",
    );
    let value = must_some(hover["contents"]["value"].as_str());

    // User-supplied doc should have markdown chars escaped
    assert!(value.contains(r"\*all\*"), "asterisks in user doc should be escaped: {value}");
    assert!(value.contains(r"\[valid\]"), "brackets in user doc should be escaped: {value}");
    // The hardcoded kind_label **before** formatting should remain as-is
    assert!(
        value.contains("**before**"),
        "hardcoded kind_label markdown should be preserved: {value}"
    );
    // Method name should appear in backtick span (not escaped — it's code)
    assert!(value.contains("`validate_input`"), "method name should appear in code span: {value}");
}

#[test]
fn hover_off_lock_analysis_emits_lock_hold_and_analyze_timing_spans()
-> Result<(), Box<dyn std::error::Error>> {
    // #3396 Phase 4: `handle_hover` grabs the parsed snapshot + text under a
    // brief documents-map lock, then drops the guard before analysis. Proves
    // this measurably: the `lock_hold` span (the brief guarded scope) must be
    // recorded before the `analyze` span (the off-lock work), for the same
    // request.
    let text = "my $var = 42;\n";
    let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
    let uri = "file:///timing-hover.pl".to_string();
    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": text
        }
    }))?;

    let _lock = crate::runtime::timing::capture::test_lock();
    crate::runtime::timing::capture::start();
    let _ = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 4 }
    })))?;
    let spans = crate::runtime::timing::capture::drain();

    let lock_hold_idx = spans.iter().position(|s| s.span == "provider.hover.lock_hold");
    let analyze_idx = spans.iter().position(|s| s.span == "provider.hover.analyze");
    assert!(lock_hold_idx.is_some(), "expected a provider.hover.lock_hold span, got: {spans:?}");
    assert!(analyze_idx.is_some(), "expected a provider.hover.analyze span, got: {spans:?}");
    assert!(
        lock_hold_idx < analyze_idx,
        "lock_hold span must be emitted before the analyze span (proves the documents-map guard \
         is dropped before analysis runs): {spans:?}"
    );

    Ok(())
}
