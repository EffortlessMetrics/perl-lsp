use anyhow::{Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn semantic_inline_next_edit_cli_writes_scaffold_receipt() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-next-edit.json");
    let receipt_arg = receipt.to_str().ok_or_else(|| anyhow!("invalid next-edit receipt path"))?;

    cargo_bin_cmd!("xtask")
        .args(["semantic-inline-next-edit", "--receipt", receipt_arg])
        .assert()
        .success();

    let scaffold: Value = serde_json::from_str(&std::fs::read_to_string(&receipt)?)?;
    assert_eq!(
        scaffold.get("schema_version").and_then(Value::as_str),
        Some("semantic-inline-next-edit.v1")
    );
    assert_eq!(scaffold.get("provider").and_then(Value::as_str), Some("inline_completion"));
    assert_eq!(scaffold.get("provider_action").and_then(Value::as_str), Some("next_edit_scaffold"));
    assert_eq!(scaffold.get("enabled_by_default").and_then(Value::as_bool), Some(false));
    assert_eq!(scaffold.get("runtime_provider_registered").and_then(Value::as_bool), Some(false));
    assert_eq!(scaffold.get("ai_candidate_source_enabled").and_then(Value::as_bool), Some(false));
    assert_eq!(
        scaffold
            .pointer("/optional_ai_candidate_boundary/enabled_by_default")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        scaffold
            .pointer("/optional_ai_candidate_boundary/ai_candidate_source_enabled")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        scaffold
            .pointer("/optional_ai_candidate_boundary/rejects_ai_enabled_policy")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        scaffold
            .pointer("/optional_ai_candidate_boundary/rejects_missing_editor_safe_range")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        scaffold
            .pointer("/optional_ai_candidate_boundary/rejects_missing_parse_safety")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        scaffold
            .pointer(
                "/optional_ai_candidate_boundary/rejects_missing_selected_completion_compatibility"
            )
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        scaffold
            .pointer("/optional_ai_candidate_boundary/rejects_nondeterministic_sources")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        scaffold
            .pointer("/optional_ai_candidate_boundary/deterministic_sources_only")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        scaffold.pointer("/default_response/status").and_then(Value::as_str),
        Some("disabled")
    );
    assert_eq!(
        scaffold.pointer("/receipt_only_response/status").and_then(Value::as_str),
        Some("receipt_only")
    );
    assert_eq!(
        scaffold.pointer("/explicit_gate_response/status").and_then(Value::as_str),
        Some("runtime_provider_not_registered")
    );
    assert!(
        scaffold
            .pointer("/explicit_gate_response/suggestions")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    );

    let planned = scaffold
        .get("planned_candidate_families")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("planned_candidate_families missing"))?;
    for family in ["missing_import", "test_assertion_body", "call_site_update", "rename_occurrence"]
    {
        assert!(planned.iter().any(|entry| entry.as_str() == Some(family)));
    }

    let boundary = scaffold
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("claim_boundary missing"))?;
    assert!(boundary.contains("does not register an LSP method"));
    assert!(boundary.contains("emit editor-visible next-edit suggestions"));
    assert!(boundary.contains("enable AI behavior"));

    let missing_import = scaffold
        .get("missing_import_next_action")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("missing_import_next_action receipt missing"))?;
    assert_eq!(
        missing_import
            .get("reachable_candidate")
            .and_then(|proof| proof.get("status"))
            .and_then(Value::as_str),
        Some("receipt_only")
    );
    assert_eq!(
        missing_import
            .get("reachable_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("module"))
            .and_then(Value::as_str),
        Some("My::App")
    );
    assert_eq!(
        missing_import
            .get("reachable_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        missing_import
            .get("duplicate_import")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("duplicate_import")
    );
    assert_eq!(
        missing_import
            .get("unreachable_module")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("unreachable_module")
    );
    for field in ["comment_target", "pod_target", "data_target"] {
        assert_eq!(
            missing_import
                .get(field)
                .and_then(|proof| proof.get("rejectionReasons"))
                .and_then(Value::as_array)
                .and_then(|reasons| reasons.first())
                .and_then(Value::as_str),
            Some("unsafe_insertion_point"),
            "{field} must reject unsafe contexts"
        );
        assert!(
            missing_import.get(field).and_then(|proof| proof.get("candidate")).is_none(),
            "{field} must not emit a candidate"
        );
    }
    assert!(
        missing_import
            .get("accepted_document_text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("use My::App;\nmy $value"))
    );
    assert_eq!(missing_import.get("parse_stable").and_then(Value::as_bool), Some(true));
    assert_eq!(missing_import.get("line_endings_preserved").and_then(Value::as_bool), Some(true));
    assert!(
        missing_import
            .get("crlf_accepted_document_text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("use My::App;\r\nmy $value"))
    );
    let project_shape = missing_import
        .get("project_shape")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("missing_import project_shape receipt missing"))?;
    assert_eq!(
        project_shape
            .get("project_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("module"))
            .and_then(Value::as_str),
        Some("My::App")
    );
    assert_eq!(
        project_shape
            .get("project_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        project_shape
            .get("duplicate_project_import")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("duplicate_import")
    );
    assert_eq!(
        project_shape
            .get("root_only_module")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("unreachable_module")
    );
    assert_eq!(
        project_shape
            .get("cancelled_lib_module")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("unreachable_module")
    );
    assert_eq!(project_shape.get("parse_stable").and_then(Value::as_bool), Some(true));
    assert!(
        project_shape
            .get("accepted_document_text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("use lib 'lib';\nuse My::App;\nmy $app"))
    );
    let test_assertion = scaffold
        .get("test_assertion_next_action")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("test_assertion_next_action receipt missing"))?;
    assert_eq!(
        test_assertion
            .get("test_more_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("family"))
            .and_then(Value::as_str),
        Some("test_assertion_body")
    );
    assert_eq!(
        test_assertion
            .get("test_more_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        test_assertion
            .get("test2_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("framework"))
            .and_then(Value::as_str),
        Some("test2_v0")
    );
    assert_eq!(
        test_assertion
            .get("non_test_file")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("test_file_required")
    );
    assert_eq!(
        test_assertion
            .get("unsupported_framework")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("unsupported_test_framework")
    );
    assert_eq!(
        test_assertion
            .get("missing_variables")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("missing_assertion_variables")
    );
    assert_eq!(test_assertion.get("parse_stable").and_then(Value::as_bool), Some(true));
    assert!(
        test_assertion
            .get("accepted_document_text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("is($got, $expected"))
    );
    let call_site_update = scaffold
        .get("call_site_update_next_action")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("call_site_update_next_action receipt missing"))?;
    assert_eq!(
        call_site_update
            .get("next_call_site_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("family"))
            .and_then(Value::as_str),
        Some("call_site_update")
    );
    assert_eq!(
        call_site_update
            .get("next_call_site_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("calleeName"))
            .and_then(Value::as_str),
        Some("build_user")
    );
    assert_eq!(
        call_site_update
            .get("next_call_site_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("argument"))
            .and_then(Value::as_str),
        Some("$age")
    );
    assert_eq!(
        call_site_update
            .get("next_call_site_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        call_site_update
            .get("duplicate_argument")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("duplicate_call_argument")
    );
    assert_eq!(
        call_site_update
            .get("missing_call_site")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("missing_call_site")
    );
    assert_eq!(
        call_site_update
            .get("unsafe_call_site")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("unsafe_insertion_point")
    );
    assert_eq!(
        call_site_update
            .get("invalid_target")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("invalid_call_target")
    );
    assert_eq!(
        call_site_update
            .get("missing_argument")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("missing_call_argument")
    );
    assert_eq!(call_site_update.get("parse_stable").and_then(Value::as_bool), Some(true));
    assert!(
        call_site_update
            .get("accepted_document_text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("build_user($name, $age)"))
    );
    let rename_occurrence = scaffold
        .get("rename_occurrence_next_action")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("rename_occurrence_next_action receipt missing"))?;
    assert_eq!(
        rename_occurrence
            .get("next_occurrence_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("family"))
            .and_then(Value::as_str),
        Some("rename_occurrence")
    );
    assert_eq!(
        rename_occurrence
            .get("next_occurrence_candidate")
            .and_then(|proof| proof.get("candidate"))
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        rename_occurrence
            .get("unsafe_occurrence")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("unsafe_insertion_point")
    );
    assert_eq!(
        rename_occurrence
            .get("missing_occurrence")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("missing_rename_occurrence")
    );
    assert_eq!(
        rename_occurrence
            .get("invalid_symbol")
            .and_then(|proof| proof.get("rejectionReasons"))
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(Value::as_str),
        Some("invalid_rename_symbol")
    );
    assert_eq!(rename_occurrence.get("parse_stable").and_then(Value::as_bool), Some(true));
    assert!(
        rename_occurrence
            .get("accepted_document_text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("return $new + $old"))
    );

    Ok(())
}
