//! Query-provider regression tests for post-edit workspace-index staleness.
//!
//! These tests use a deterministic helper instead of racing the scheduler: the
//! open document is advanced to the edited text while the workspace index is
//! intentionally left at the pre-edit text. The provider must not report a
//! stale workspace-index definition as a complete answer.

#![cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]

use perl_lsp::LspServer;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URI: &str = "file:///workspace/lib/Edit/Stale.pm";

const BEFORE_EDIT: &str = r#"package Edit::Stale;
use strict;
use warnings;

sub caller {
    target();
}

sub target {
    return 1;
}

1;
"#;

const AFTER_EDIT: &str = r#"package Edit::Stale;
use strict;
use warnings;

sub caller {
    target();
}

1;
"#;

const FRAMEWORK_URI: &str = "file:///workspace/lib/Edit/FrameworkStale.pm";

const FRAMEWORK_BEFORE_EDIT: &str = r#"package Edit::Consumer;
use Moo;
with 'Edit::Target';

package Edit::Target;
use Moo::Role;

1;
"#;

const FRAMEWORK_AFTER_EDIT: &str = r#"package Edit::Consumer;
use Moo;
with 'Edit::Target';

1;
"#;

fn position_of(text: &str, needle: &str) -> TestResult<(u32, u32)> {
    for (line_idx, line) in text.lines().enumerate() {
        if let Some(character) = line.find(needle) {
            return Ok((u32::try_from(line_idx)?, u32::try_from(character)?));
        }
    }
    Err(format!("needle `{needle}` not found").into())
}

fn contains_location_start(value: Option<&Value>, line: u32, character: u32) -> bool {
    let Some(Value::Array(items)) = value else {
        return false;
    };
    items.iter().any(|item| {
        item.pointer("/range/start/line").and_then(Value::as_u64) == Some(u64::from(line))
            && item.pointer("/range/start/character").and_then(Value::as_u64)
                == Some(u64::from(character))
    })
}

#[test]
fn definition_does_not_answer_from_stale_current_file_index() -> TestResult {
    let server = LspServer::new();

    server.test_apply_did_open(URI, BEFORE_EDIT, 1)?;
    server.test_replace_document_without_index(URI, AFTER_EDIT, 2)?;

    let (line, character) = position_of(AFTER_EDIT, "target();")?;
    let (old_target_line, old_target_character) = position_of(BEFORE_EDIT, "sub target")?;
    let result = server.test_handle_definition(Some(json!({
        "textDocument": {
            "uri": URI,
            "version": 2
        },
        "position": {
            "line": line,
            "character": character
        }
    })))?;

    assert!(
        !contains_location_start(result.as_ref(), old_target_line, old_target_character),
        "definition must not return the removed pre-edit target from a stale workspace index; got {result:?}"
    );

    Ok(())
}

#[test]
fn quoted_framework_definition_does_not_answer_from_stale_current_file_index() -> TestResult {
    let server = LspServer::new();

    server.test_apply_did_open(FRAMEWORK_URI, FRAMEWORK_BEFORE_EDIT, 1)?;
    server.test_replace_document_without_index(FRAMEWORK_URI, FRAMEWORK_AFTER_EDIT, 2)?;

    let (line, character) = position_of(FRAMEWORK_AFTER_EDIT, "Edit::Target")?;
    let (old_target_line, old_target_character) =
        position_of(FRAMEWORK_BEFORE_EDIT, "package Edit::Target")?;
    let result = server.test_handle_definition(Some(json!({
        "textDocument": {
            "uri": FRAMEWORK_URI,
            "version": 2
        },
        "position": {
            "line": line,
            "character": character
        }
    })))?;

    assert!(
        !contains_location_start(result.as_ref(), old_target_line, old_target_character),
        "quoted framework definition must not return the removed pre-edit package from a stale workspace index; got {result:?}"
    );

    Ok(())
}
