use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{path}"))?)
}

#[test]
fn query_cross_file_references_returns_definition_and_cross_file_usages()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let def_uri = file_url("/workspace/lib/Utils.pm")?;
    let caller_a_uri = file_url("/workspace/app/main.pl")?;
    let caller_b_uri = file_url("/workspace/app/worker.pl")?;

    index
        .index_file(def_uri.clone(), "package Utils;\nsub process_data { 1 }\n1;\n".to_string())?;
    index.index_file(caller_a_uri, "package App::Main;\nUtils::process_data();\n".to_string())?;
    index.index_file(caller_b_uri, "package App::Worker;\nUtils::process_data();\n".to_string())?;

    let result =
        index.query_cross_file_references("Utils::process_data").ok_or("symbol not found")?;

    assert_eq!(result.symbol.stable_key, "Utils::process_data");
    assert_eq!(result.symbol.qualified_name.as_deref(), Some("Utils::process_data"));
    assert!(result.definition.uri.ends_with("/workspace/lib/Utils.pm"));
    assert_eq!(result.references.len(), 2);
    assert!(
        result.references[0].uri <= result.references[1].uri,
        "references should be deterministically ordered"
    );
    assert!(
        result.references.iter().all(|location| location.uri != def_uri.to_string()),
        "definition should not be returned as a usage reference"
    );

    Ok(())
}

#[test]
fn query_cross_file_references_avoids_bare_name_false_positives_and_handles_not_found()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let alpha_uri = file_url("/workspace/lib/Alpha.pm")?;
    let beta_uri = file_url("/workspace/lib/Beta.pm")?;
    let caller_uri = file_url("/workspace/app/caller.pl")?;

    index.index_file(alpha_uri, "package Alpha;\nsub helper { 1 }\n1;\n".to_string())?;
    index.index_file(beta_uri, "package Beta;\nsub helper { 1 }\n1;\n".to_string())?;
    index.index_file(caller_uri, "package Caller;\nAlpha::helper();\n".to_string())?;

    let result = index.query_cross_file_references("Alpha::helper").ok_or("symbol not found")?;

    assert_eq!(result.symbol.stable_key, "Alpha::helper");
    assert_eq!(result.references.len(), 1);
    assert!(result.references[0].uri.ends_with("/workspace/app/caller.pl"));

    assert!(index.query_cross_file_references("Does::Not::Exist").is_none());

    Ok(())
}
