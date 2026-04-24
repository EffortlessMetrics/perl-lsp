// Minimal test to reproduce deep nesting, large arrays, and cyclic reference behavior
// Run with: cargo test --test test_deep_truncation -- --nocapture

#[cfg(test)]
mod deep_truncation_tests {
    use perl_dap::variables::{PerlValue, PerlVariableRenderer, VariableParser, VariableRenderer};
    use serde::Deserialize;
    use std::fs;
    use std::path::PathBuf;

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

    #[derive(Debug, Deserialize)]
    struct FixtureBank {
        variable_cases: Vec<VariableCase>,
        scope_visibility: ScopeVisibility,
        truncation_case: TruncationCase,
    }

    #[derive(Debug, Deserialize)]
    struct VariableCase {
        id: String,
        assignment: String,
        expected_type: String,
        expected_indexed_variables: Option<i64>,
        expected_named_variables: Option<i64>,
        preview_contains: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct ScopeVisibility {
        lexical: Vec<String>,
        package: Vec<String>,
        global: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct TruncationCase {
        id: String,
        summary: String,
        total_count: Option<usize>,
        expected_preview_contains: Vec<String>,
    }

    fn load_fixture_bank() -> Result<FixtureBank, Box<dyn std::error::Error>> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mocks/dap_correctness_fixture_bank.json");
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    #[test]
    fn fixture_bank_large_arrays_render_with_bounded_previews()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = load_fixture_bank()?;
        let parser = VariableParser::new();
        let renderer = PerlVariableRenderer::new();

        for case_id in ["array_200_real_session", "array_500_real_session"] {
            let case = fixture
                .variable_cases
                .iter()
                .find(|entry| entry.id == case_id)
                .ok_or_else(|| format!("missing fixture case: {case_id}"))?;
            let (_, parsed_value) = parser.parse_assignment(&case.assignment)?;
            let rendered = renderer.render("@fixture", &parsed_value);

            assert_eq!(rendered.type_name.as_deref(), Some(case.expected_type.as_str()));
            assert_eq!(rendered.indexed_variables, case.expected_indexed_variables);
            assert!(rendered.value.len() < 600, "array preview should remain bounded");
            for snippet in &case.preview_contains {
                assert!(
                    rendered.value.contains(snippet),
                    "expected preview for {} to contain {:?}, got {:?}",
                    case.id,
                    snippet,
                    rendered.value
                );
            }
        }
        Ok(())
    }

    #[test]
    fn fixture_bank_nested_hash_and_unicode_values_render() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = load_fixture_bank()?;
        let parser = VariableParser::new();
        let renderer = PerlVariableRenderer::new();

        for case_id in ["deep_nested_hash_real_session", "unicode_hash_real_session"] {
            let case = fixture
                .variable_cases
                .iter()
                .find(|entry| entry.id == case_id)
                .ok_or_else(|| format!("missing fixture case: {case_id}"))?;
            let (_, parsed_value) = parser.parse_assignment(&case.assignment)?;
            let rendered = renderer.render("%fixture", &parsed_value);

            assert_eq!(rendered.type_name.as_deref(), Some(case.expected_type.as_str()));
            assert_eq!(rendered.named_variables, case.expected_named_variables);
            for snippet in &case.preview_contains {
                assert!(
                    rendered.value.contains(snippet),
                    "expected preview for {} to contain {:?}, got {:?}",
                    case.id,
                    snippet,
                    rendered.value
                );
            }
        }
        Ok(())
    }

    #[test]
    fn fixture_bank_coderef_and_blessed_object_previews_render()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = load_fixture_bank()?;
        let parser = VariableParser::new();
        let renderer = PerlVariableRenderer::new();

        for case_id in ["coderef_preview_real_session", "blessed_object_preview_real_session"] {
            let case = fixture
                .variable_cases
                .iter()
                .find(|entry| entry.id == case_id)
                .ok_or_else(|| format!("missing fixture case: {case_id}"))?;
            let (_, parsed_value) = parser.parse_assignment(&case.assignment)?;
            let rendered = renderer.render("$fixture", &parsed_value);

            assert_eq!(rendered.type_name.as_deref(), Some(case.expected_type.as_str()));
            for snippet in &case.preview_contains {
                assert!(
                    rendered.value.contains(snippet),
                    "expected preview for {} to contain {:?}, got {:?}",
                    case.id,
                    snippet,
                    rendered.value
                );
            }
        }
        Ok(())
    }

    #[test]
    fn fixture_bank_scope_visibility_sets_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = load_fixture_bank()?;
        let scopes = fixture.scope_visibility;

        for lexical in &scopes.lexical {
            assert!(!scopes.package.contains(lexical));
            assert!(!scopes.global.contains(lexical));
        }
        for package in &scopes.package {
            assert!(!scopes.global.contains(package));
        }
        assert!(scopes.lexical.iter().all(|name| name.starts_with('$')));
        assert!(
            scopes.package.iter().any(|name| name.starts_with("%main::")),
            "package scope should include package-qualified hashes"
        );
        assert!(scopes.global.iter().any(|name| name == "$ENV"));
        Ok(())
    }

    #[test]
    fn fixture_bank_truncated_deep_structure_preview_is_stable()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = load_fixture_bank()?;
        let renderer = PerlVariableRenderer::new();
        let trunc = fixture.truncation_case;
        let value = PerlValue::Truncated { summary: trunc.summary, total_count: trunc.total_count };
        let rendered = renderer.render("$deep", &value);

        assert!(
            rendered.indexed_variables.is_none(),
            "truncated summary values should not expose child pagination metadata"
        );
        assert!(
            rendered.value.len() < 300,
            "truncated preview should be concise: {}",
            rendered.value
        );
        for snippet in &trunc.expected_preview_contains {
            assert!(rendered.value.contains(snippet));
        }
        assert!(trunc.id.contains("real_session"));
        Ok(())
    }
}
