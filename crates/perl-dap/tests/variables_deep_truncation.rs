// Minimal test to reproduce deep nesting, large arrays, and cyclic reference behavior
// Run with: cargo test --test test_deep_truncation -- --nocapture
mod common;

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

#[cfg(test)]
mod variable_fixture_bank_tests {
    use perl_dap::variables::{PerlValue, PerlVariableRenderer, VariableRenderer};
    use std::fs;
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn fixture_text() -> Result<String, Box<dyn std::error::Error>> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/dap_real_session_data.pl");
        Ok(fs::read_to_string(path)?)
    }

    #[test]
    fn test_fixture_contains_large_array_and_deep_hash_markers() -> TestResult {
        let text = fixture_text()?;
        assert!(text.contains("@large_200"));
        assert!(text.contains("@large_500"));
        assert!(text.contains("level5"));
        Ok(())
    }

    #[test]
    fn test_fixture_contains_scope_and_preview_targets() -> TestResult {
        let text = fixture_text()?;
        assert!(text.contains("our $shared_symbol"));
        assert!(text.contains("my $shared_symbol"));
        assert!(text.contains("$coderef"));
        assert!(text.contains("$object"));
        Ok(())
    }

    #[test]
    fn test_fixture_unicode_tokens_render_via_renderer() -> TestResult {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Hash(vec![
            ("ключ".to_string(), PerlValue::Scalar("значение".to_string())),
            ("こんにちは".to_string(), PerlValue::Scalar("世界".to_string())),
        ]);
        let rendered = renderer.render("%unicode_hash", &value);
        assert!(rendered.value.contains("ключ") || rendered.value.contains("..."));
        Ok(())
    }

    #[test]
    fn test_fixture_sized_structures_truncate_in_preview() {
        let renderer = PerlVariableRenderer::new();
        let large_500 = PerlValue::Array((0..551).map(PerlValue::Integer).collect());
        let rendered = renderer.render("@large_500", &large_500);
        assert!(rendered.value.len() < 512);
        assert!(rendered.value.contains("..."));
    }
}

#[cfg(any())]
mod real_session_fixture_tests {
    use super::common::{DapWorkflowSession, perl_available, workflow_timeout};
    use serde_json::{Value, json};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FIXTURE_PATH: &str = "tests/fixtures/dap_real_session_data.pl";
    const FIXTURE_BREAKPOINT_LINE: u64 = 54;

    fn fixture_script_path() -> Result<String, Box<dyn std::error::Error>> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let path = path.to_str().ok_or("fixture path is not valid UTF-8")?.to_string();
        Ok(path)
    }

    fn find_var<'a>(vars: &'a [Value], name: &str) -> Option<&'a Value> {
        vars.iter().find(|v| v.get("name").and_then(Value::as_str) == Some(name))
    }

    fn request_variables_page(
        session: &mut DapWorkflowSession,
        variables_reference: i64,
        start: i64,
        count: i64,
    ) -> Result<Vec<Value>, String> {
        let response = session.request(
            "variables",
            Some(json!({
                "variablesReference": variables_reference,
                "start": start,
                "count": count
            })),
        );
        let body = session.expect_success(&response, "variables")?;
        let body = body.ok_or("variables response missing body")?;
        let variables = body
            .get("variables")
            .and_then(Value::as_array)
            .ok_or("variables response missing `variables` array")?
            .clone();
        Ok(variables)
    }

    fn package_scope_ref(session: &mut DapWorkflowSession, frame_id: i64) -> Result<i64, String> {
        let response = session.request("scopes", Some(json!({"frameId": frame_id})));
        let body = session.expect_success(&response, "scopes")?;
        let body = body.ok_or("scopes response missing body")?;
        let scopes = body
            .get("scopes")
            .and_then(Value::as_array)
            .ok_or("scopes response missing `scopes` array")?;
        for scope in scopes {
            if scope.get("name").and_then(Value::as_str) == Some("Package")
                && let Some(reference) = scope.get("variablesReference").and_then(Value::as_i64)
            {
                return Ok(reference);
            }
        }
        Err("Package scope not found".to_string())
    }

    fn launch_and_stop() -> Result<(DapWorkflowSession, i64), Box<dyn std::error::Error>> {
        let script = fixture_script_path()?;
        let mut session = DapWorkflowSession::new(workflow_timeout())?;
        let launch = session.request(
            "launch",
            Some(json!({
                "program": script.clone(),
                "args": [],
                "stopOnEntry": true,
                "env": {
                    "PERL_PERTURB_KEYS": "0",
                    "PERL_HASH_SEED": "0",
                    "LC_ALL": "C",
                    "TZ": "UTC"
                }
            })),
        );
        session.expect_success(&launch, "launch")?;
        session.set_breakpoints(&script, &[FIXTURE_BREAKPOINT_LINE])?;
        session.configuration_done()?;
        let mut stopped = session.wait_stopped()?;
        let mut frame = session.stack_trace(stopped.thread_id)?;
        if frame.2 < FIXTURE_BREAKPOINT_LINE as i64 {
            session.continue_exec(stopped.thread_id)?;
            stopped = session.wait_stopped()?;
            frame = session.stack_trace(stopped.thread_id)?;
        }
        if frame.2 < FIXTURE_BREAKPOINT_LINE as i64 {
            return Err("did not reach fixture breakpoint".into());
        }
        Ok((session, frame.0))
    }

    #[test]
    fn test_fixture_scopes_show_lexical_package_global_visibility() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, frame_id) = launch_and_stop()?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let package_ref = package_scope_ref(&mut session, frame_id)?;
        let globals_ref = session.scopes_globals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;
        let package = session.variables(package_ref)?;
        let globals = session.variables(globals_ref)?;

        assert!(find_var(&locals, "$shared_symbol").is_some());
        assert!(find_var(&package, "$shared_symbol").is_some());
        assert!(find_var(&globals, "$GLOBAL_NAME").is_some());
        assert!(find_var(&locals, "$GLOBAL_NAME").is_none());

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_large_array_500_paginates_real_session() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, frame_id) = launch_and_stop()?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;
        let large_500 = find_var(&locals, "@large_500").ok_or("missing @large_500")?;
        let reference = large_500
            .get("variablesReference")
            .and_then(Value::as_i64)
            .ok_or("@large_500 missing variablesReference")?;

        let page = request_variables_page(&mut session, reference, 500, 60)?;
        assert_eq!(page.len(), 51);
        assert_eq!(page.first().and_then(|v| v.get("name")).and_then(Value::as_str), Some("[500]"));
        assert_eq!(page.last().and_then(|v| v.get("name")).and_then(Value::as_str), Some("[550]"));

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_large_array_200_preview_present_real_session() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, frame_id) = launch_and_stop()?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;
        let large_200 = find_var(&locals, "@large_200").ok_or("missing @large_200")?;
        let preview = large_200.get("value").and_then(Value::as_str).ok_or("missing value")?;
        assert!(preview.contains("..."), "array preview should be truncated: {preview}");

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_deep_hash_and_unicode_values_are_expandable() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, frame_id) = launch_and_stop()?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;

        let deep_hash = find_var(&locals, "%deep_hash").ok_or("missing %deep_hash")?;
        let deep_reference = deep_hash
            .get("variablesReference")
            .and_then(Value::as_i64)
            .ok_or("%deep_hash missing variablesReference")?;
        let deep_children = request_variables_page(&mut session, deep_reference, 0, 20)?;
        assert!(find_var(&deep_children, "level1").is_some());

        let unicode = find_var(&locals, "%unicode_hash").ok_or("missing %unicode_hash")?;
        let unicode_reference = unicode
            .get("variablesReference")
            .and_then(Value::as_i64)
            .ok_or("%unicode_hash missing variablesReference")?;
        let unicode_children = request_variables_page(&mut session, unicode_reference, 0, 20)?;
        assert!(find_var(&unicode_children, "ключ").is_some());
        assert!(find_var(&unicode_children, "こんにちは").is_some());

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_coderef_and_blessed_object_have_previews() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, frame_id) = launch_and_stop()?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;

        let coderef = find_var(&locals, "$coderef").ok_or("missing $coderef")?;
        let coderef_value =
            coderef.get("value").and_then(Value::as_str).ok_or("missing coderef preview")?;
        assert!(coderef_value.contains("CODE"));

        let object = find_var(&locals, "$object").ok_or("missing $object")?;
        let object_value =
            object.get("value").and_then(Value::as_str).ok_or("missing object preview")?;
        assert!(
            object_value.contains("Fixture::Widget") || object_value.contains("HASH"),
            "unexpected object preview: {object_value}"
        );

        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_fixture_deep_data_truncation_preview_is_bounded() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let (mut session, frame_id) = launch_and_stop()?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;
        let deep_hash = find_var(&locals, "%deep_hash").ok_or("missing %deep_hash")?;
        let preview = deep_hash.get("value").and_then(Value::as_str).ok_or("missing preview")?;
        assert!(preview.len() < 512);
        assert!(preview.contains("...") || preview.contains("HASH"));

        session.disconnect()?;
        Ok(())
    }
}
