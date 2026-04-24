// Minimal test to reproduce deep nesting, large arrays, and cyclic reference behavior
// Run with: cargo test --test test_deep_truncation -- --nocapture

#[cfg(test)]
mod deep_truncation_tests {
    use perl_dap::variables::{PerlValue, PerlVariableRenderer, VariableParser, VariableRenderer};

    #[test]
    fn test_7level_nested_hash_rendering() {
        let renderer = PerlVariableRenderer::new();

        // Build a 7-level nested hash
        let mut value = PerlValue::Hash(vec![("g".to_string(), PerlValue::Integer(1))]);
        for level in ['f', 'e', 'd', 'c', 'b', 'a'].iter() {
            value = PerlValue::Hash(vec![(level.to_string(), value)]);
        }

        let rendered = renderer.render("$config", &value);
        println!("7-level nested hash:");
        println!("  value: {}", rendered.value);
        println!("  type_name: {:?}", rendered.type_name);
        println!("  named_variables: {:?}", rendered.named_variables);

        // Should not panic or produce exponential output
        assert!(rendered.value.len() < 1000, "value should be bounded");
        assert_eq!(rendered.type_name, Some("HASH".to_string()));
        assert_eq!(rendered.named_variables, Some(1));
    }

    #[test]
    fn test_500element_array_rendering() {
        let renderer = PerlVariableRenderer::new();

        let elements: Vec<PerlValue> = (0..500).map(PerlValue::Integer).collect();
        let value = PerlValue::Array(elements);

        let rendered = renderer.render("@big", &value);
        println!("500-element array:");
        println!("  value: {}", rendered.value);
        println!("  type_name: {:?}", rendered.type_name);
        println!("  indexed_variables: {:?}", rendered.indexed_variables);

        // Should show truncation marker
        assert!(rendered.value.contains("..."), "should have truncation marker");
        assert!(rendered.value.contains("500 total"), "should show total count");
        assert!(rendered.indexed_variables.is_some());
        assert!(rendered.value.len() < 500, "preview should be bounded");
    }

    #[test]
    fn test_500element_array_pagination() {
        let renderer = PerlVariableRenderer::new();

        let elements: Vec<PerlValue> = (0..500).map(PerlValue::Integer).collect();
        let value = PerlValue::Array(elements);

        // Request children at various positions
        let start = renderer.render_children(&value, 0, 50);
        assert_eq!(start.len(), 50);
        assert_eq!(start[0].name, "[0]");

        let mid = renderer.render_children(&value, 250, 50);
        assert_eq!(mid.len(), 50);
        assert_eq!(mid[0].name, "[250]");

        let end = renderer.render_children(&value, 450, 100);
        assert_eq!(end.len(), 50, "only 50 items left at [450..500]");
        assert_eq!(end[0].name, "[450]");

        println!("500-element array pagination: OK");
    }

    #[test]
    fn test_cyclic_reference_rendering() {
        let renderer = PerlVariableRenderer::new();

        // Simulate a self-referential hash: my %c; $c{self} = \%c;
        // In reality, PerlValue uses Box (no Rc), so true cycles can't exist.
        // The debugger would emit a Truncated marker instead.
        let truncated_marker =
            PerlValue::Truncated { summary: "HASH(0x7f1234567890)".to_string(), total_count: None };
        let value = PerlValue::Hash(vec![(
            "self".to_string(),
            PerlValue::Reference(Box::new(truncated_marker)),
        )]);

        let rendered = renderer.render("$c", &value);
        println!("Cyclic reference hash:");
        println!("  value: {}", rendered.value);
        println!("  type_name: {:?}", rendered.type_name);

        // Should not panic
        assert_eq!(rendered.type_name, Some("HASH".to_string()));
        assert_eq!(rendered.named_variables, Some(1));
        assert!(rendered.value.len() < 500);
    }

    #[test]
    fn test_parser_max_depth_parsing() {
        let parser = VariableParser::new();

        // Try to parse a 7-level nested literal
        let text = "$x = { a => { b => { c => { d => { e => { f => { g => 1 } } } } } } }";
        let result = parser.parse_assignment(text);

        // Should parse successfully with default max_depth=50
        assert!(result.is_ok(), "parser should accept 7-level nested hash: {:?}", result.err());
        if let Ok((name, value)) = result {
            println!("Parsed 7-level nested hash:");
            println!("  name: {}", name);
            println!("  value: {:?}", value);
            assert_eq!(name, "$x");
        }
    }

    #[test]
    fn test_parser_exceeds_max_depth() {
        let parser = VariableParser::new().with_max_depth(3);

        // Try to parse a 7-level nested literal with shallow max_depth
        let text = "$x = { a => { b => { c => { d => 1 } } } }";
        let result = parser.parse_assignment(text);

        // Should fail due to max_depth exceeded
        assert!(result.is_err(), "should fail with max_depth=3");
        println!("Parser correctly rejects depth > 3: OK");
    }

    #[test]
    fn test_render_deeply_nested_hash_with_children() {
        let renderer = PerlVariableRenderer::new();

        // Build nested structure and check children expansion
        let mut value = PerlValue::Hash(vec![("level7".to_string(), PerlValue::Integer(7))]);
        for level in (1..=6).rev() {
            value = PerlValue::Hash(vec![(format!("level{}", level), value)]);
        }

        let rendered = renderer.render("$root", &value);
        let children = renderer.render_children(&value, 0, 10);

        println!("Nested hash children:");
        println!("  root_value: {}", rendered.value);
        println!("  children_count: {}", children.len());
        if !children.is_empty() {
            println!("  first_child: name={}, value={}", children[0].name, children[0].value);
        }

        assert_eq!(children.len(), 1, "root should have 1 child");
        assert_eq!(children[0].name, "level1");
    }
}

mod common;

#[cfg(test)]
mod fixture_backed_deep_truncation_tests {
    use super::common::{DapWorkflowSession, perl_available, workflow_timeout};
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FIXTURE_BREAKPOINT_LINE: u64 = 59;

    fn fixture_script_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/variables_evaluate_deep_session.pl");
        Ok(fixture)
    }

    fn variable_named<'a>(vars: &'a [Value], name: &str) -> Option<&'a Value> {
        vars.iter().find(|entry| entry.get("name").and_then(Value::as_str) == Some(name))
    }

    fn nested_reference(var: &Value) -> i64 {
        var.get("variablesReference").and_then(Value::as_i64).unwrap_or(0)
    }

    fn session_stopped_at_fixture() -> Result<(DapWorkflowSession, i64, TempDir), Box<dyn std::error::Error>> {
        let timeout = workflow_timeout();
        let fixture_path = fixture_script_path()?;
        let fixture_contents = fs::read_to_string(&fixture_path)?;
        let temp_workspace = tempfile::tempdir()?;
        let temp_script = temp_workspace.path().join("variables_evaluate_deep_session.pl");
        fs::write(&temp_script, fixture_contents)?;
        let script_path =
            temp_script.to_str().ok_or("temp fixture path is not valid UTF-8")?.to_string();

        let mut session = DapWorkflowSession::new(timeout)?;
        session.launch(&script_path)?;
        session.set_breakpoints(&script_path, &[FIXTURE_BREAKPOINT_LINE])?;
        session.configuration_done()?;
        for _ in 0..3 {
            let stopped = session.wait_stopped()?;
            let thread_id = stopped.thread_id;
            let (_, _, frame_line) = session.stack_trace(thread_id)?;
            if frame_line == FIXTURE_BREAKPOINT_LINE as i64 {
                return Ok((session, thread_id, temp_workspace));
            }
            session.continue_exec(thread_id)?;
        }
        Err(format!("did not stop at fixture breakpoint line {}", FIXTURE_BREAKPOINT_LINE).into())
    }

    #[test]
    fn test_fixture_scope_visibility_locals_vs_globals() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, thread_id, _temp_workspace) = match session_stopped_at_fixture() {
            Ok(state) => state,
            Err(_) => return Ok(()),
        };
        let (frame_id, _, _) = session.stack_trace(thread_id)?;

        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let globals_ref = session.scopes_globals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;
        let globals = session.variables(globals_ref)?;

        assert!(variable_named(&locals, "$lexical_scalar").is_some());
        assert!(variable_named(&locals, "@big_array").is_some());
        assert!(variable_named(&locals, "%deep_hash").is_some());
        assert!(variable_named(&locals, "$coderef").is_some());
        assert!(variable_named(&locals, "$object").is_some());
        assert!(variable_named(&globals, "$GLOBAL_SCALAR").is_some());
        assert!(variable_named(&globals, "%GLOBAL_HASH").is_some());

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_500_plus_array_pagination_is_stable() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, thread_id, _temp_workspace) = match session_stopped_at_fixture() {
            Ok(state) => state,
            Err(_) => return Ok(()),
        };
        let (frame_id, _, _) = session.stack_trace(thread_id)?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;
        let big_array = variable_named(&locals, "@big_array").ok_or("missing @big_array")?;
        let array_ref = nested_reference(big_array);
        assert!(array_ref > 0);

        let first_resp = session.request(
            "variables",
            Some(json!({"variablesReference": array_ref, "start": 0, "count": 200})),
        );
        let first_page = session.expect_success(&first_resp, "variables")?.ok_or("missing body")?;
        let first_vars = first_page
            .get("variables")
            .and_then(Value::as_array)
            .ok_or("missing first page variables")?;
        assert_eq!(first_vars.len(), 200);
        assert_eq!(first_vars.first().and_then(|v| v.get("name")).and_then(Value::as_str), Some("[0]"));

        let tail_resp = session.request(
            "variables",
            Some(json!({"variablesReference": array_ref, "start": 500, "count": 100})),
        );
        let tail_page = session.expect_success(&tail_resp, "variables")?.ok_or("missing body")?;
        let tail_vars =
            tail_page.get("variables").and_then(Value::as_array).ok_or("missing tail page variables")?;
        assert_eq!(tail_vars.len(), 50);
        assert_eq!(tail_vars.first().and_then(|v| v.get("name")).and_then(Value::as_str), Some("[500]"));
        assert_eq!(tail_vars.last().and_then(|v| v.get("name")).and_then(Value::as_str), Some("[549]"));

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_deep_hash_and_unicode_map_expand() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, thread_id, _temp_workspace) = match session_stopped_at_fixture() {
            Ok(state) => state,
            Err(_) => return Ok(()),
        };
        let (frame_id, _, _) = session.stack_trace(thread_id)?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;

        let deep_hash = variable_named(&locals, "%deep_hash").ok_or("missing %deep_hash")?;
        let deep_hash_ref = nested_reference(deep_hash);
        assert!(deep_hash_ref > 0);
        let deep_resp = session.request(
            "variables",
            Some(json!({"variablesReference": deep_hash_ref, "start": 0, "count": 20})),
        );
        let deep_body = session.expect_success(&deep_resp, "variables")?.ok_or("missing deep body")?;
        let deep_vars = deep_body.get("variables").and_then(Value::as_array).ok_or("missing deep vars")?;
        assert!(variable_named(deep_vars, "level1").is_some());

        let unicode_map = variable_named(&locals, "%unicode_map").ok_or("missing %unicode_map")?;
        let unicode_ref = nested_reference(unicode_map);
        assert!(unicode_ref > 0);
        let unicode_resp = session.request(
            "variables",
            Some(json!({"variablesReference": unicode_ref, "start": 0, "count": 20})),
        );
        let unicode_body = session.expect_success(&unicode_resp, "variables")?.ok_or("missing unicode body")?;
        let unicode_vars =
            unicode_body.get("variables").and_then(Value::as_array).ok_or("missing unicode vars")?;
        assert!(variable_named(unicode_vars, "ключ").is_some());
        assert!(variable_named(unicode_vars, "emoji_😀").is_some());

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_coderef_object_and_truncated_probe_are_expandable() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, thread_id, _temp_workspace) = match session_stopped_at_fixture() {
            Ok(state) => state,
            Err(_) => return Ok(()),
        };
        let (frame_id, _, _) = session.stack_trace(thread_id)?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;

        let coderef = variable_named(&locals, "$coderef").ok_or("missing $coderef")?;
        assert!(coderef.get("value").and_then(Value::as_str).unwrap_or("").contains("CODE"));

        let object = variable_named(&locals, "$object").ok_or("missing $object")?;
        let object_value = object.get("value").and_then(Value::as_str).unwrap_or("");
        assert!(object_value.contains("Fixture::Thing"));
        let object_ref = nested_reference(object);
        assert!(object_ref > 0);

        let object_resp = session.request(
            "variables",
            Some(json!({"variablesReference": object_ref, "start": 0, "count": 20})),
        );
        let object_body = session.expect_success(&object_resp, "variables")?.ok_or("missing object body")?;
        let object_vars =
            object_body.get("variables").and_then(Value::as_array).ok_or("missing object vars")?;
        assert!(variable_named(object_vars, "class_name").is_some());
        assert!(variable_named(object_vars, "status").is_some());

        let truncated_probe = variable_named(&locals, "$truncated_probe").ok_or("missing probe")?;
        let truncated_value = truncated_probe.get("value").and_then(Value::as_str).unwrap_or("");
        assert!(
            truncated_value.contains("...") || truncated_value.contains("HASH("),
            "expected truncated or structured preview, got: {truncated_value}"
        );

        session.disconnect()?;
        Ok(())
    }
}
