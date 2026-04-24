use perl_workspace::workspace::workspace_index::{SymKind, SymbolKey, WorkspaceIndex};
use std::sync::Arc;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

#[test]
fn symbol_key_queries_resolve_cross_file_definition_and_references()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    index.index_file(
        file_url("/workspace/lib/My/App.pm")?,
        "package My::App;\nsub run { return 1; }\n".to_string(),
    )?;

    index.index_file(file_url("/workspace/bin/tool.pl")?, "My::App::run();\n".to_string())?;

    index.index_file(file_url("/workspace/t/run.t")?, "run();\n".to_string())?;

    let key = SymbolKey {
        pkg: Arc::from("My::App"),
        name: Arc::from("run"),
        sigil: None,
        kind: SymKind::Sub,
    };

    let definition = index.find_def(&key);
    assert!(definition.is_some(), "expected definition for My::App::run");

    let refs = index.find_refs(&key);
    assert_eq!(refs.len(), 2, "expected two cross-file callsite references");

    let files: Vec<&str> = refs.iter().map(|location| location.uri.as_str()).collect();
    assert_eq!(
        files,
        vec!["file:///workspace/bin/tool.pl", "file:///workspace/t/run.t"],
        "reference ordering should be deterministic by URI",
    );

    assert_eq!(key.stable_id(), "sub:My::App::run");
    Ok(())
}

#[test]
fn symbol_key_queries_return_clean_not_found_results() {
    let index = WorkspaceIndex::new();
    let key = SymbolKey {
        pkg: Arc::from("Missing::Pkg"),
        name: Arc::from("missing_sub"),
        sigil: None,
        kind: SymKind::Sub,
    };

    assert!(index.find_def(&key).is_none(), "missing symbol should have no definition");
    assert!(index.find_refs(&key).is_empty(), "missing symbol should have no references");
}
