use perl_workspace::workspace::workspace_index::{SafeDeletePreflight, WorkspaceIndex};
use url::Url;

#[test]
fn safe_delete_preflight_blocks_symbol_with_external_references()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let symbol_uri = Url::parse("file:///workspace/lib/DeleteTarget.pm")?;
    let caller_uri = Url::parse("file:///workspace/lib/Caller.pm")?;

    index.index_file(
        symbol_uri.clone(),
        r#"
package DeleteTarget;
sub to_delete { return 1; }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        caller_uri.clone(),
        r#"
package Caller;
DeleteTarget::to_delete();
1;
"#
        .to_string(),
    )?;

    let preflight =
        index.safe_delete_symbol_preflight("DeleteTarget::to_delete", symbol_uri.as_str());
    match preflight {
        SafeDeletePreflight::Blocked { external_references } => {
            assert!(
                external_references.iter().any(|location| location.uri == caller_uri.as_str()),
                "expected caller URI in blocking references, got: {external_references:?}"
            );
        }
        SafeDeletePreflight::SafeToDelete => {
            return Err("expected preflight to block external references".into());
        }
        _ => return Err("unexpected safe-delete preflight variant".into()),
    }

    Ok(())
}

#[test]
fn safe_delete_preflight_allows_symbol_with_no_external_references()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let symbol_uri = Url::parse("file:///workspace/lib/Solo.pm")?;

    index.index_file(
        symbol_uri.clone(),
        r#"
package Solo;
sub internal_only { return 1; }
internal_only();
1;
"#
        .to_string(),
    )?;

    let preflight = index.safe_delete_symbol_preflight("Solo::internal_only", symbol_uri.as_str());
    match preflight {
        SafeDeletePreflight::SafeToDelete => {}
        SafeDeletePreflight::Blocked { external_references } => {
            return Err(
                format!("expected safe delete, got blocked refs: {external_references:?}").into(),
            );
        }
        _ => return Err("unexpected safe-delete preflight variant".into()),
    }

    Ok(())
}
