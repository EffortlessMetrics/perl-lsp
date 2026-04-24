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
    fn test_500element_array_pagination_windows_do_not_overlap() {
        let renderer = PerlVariableRenderer::new();
        let elements: Vec<PerlValue> = (0..500).map(PerlValue::Integer).collect();
        let value = PerlValue::Array(elements);

        let first_window = renderer.render_children(&value, 0, 128);
        let second_window = renderer.render_children(&value, 128, 128);
        let near_end_window = renderer.render_children(&value, 384, 200);

        assert_eq!(first_window.len(), 128);
        assert_eq!(second_window.len(), 128);
        assert_eq!(near_end_window.len(), 116);

        assert_eq!(first_window.first().map(|child| child.name.as_str()), Some("[0]"));
        assert_eq!(first_window.last().map(|child| child.name.as_str()), Some("[127]"));
        assert_eq!(second_window.first().map(|child| child.name.as_str()), Some("[128]"));
        assert_eq!(second_window.last().map(|child| child.name.as_str()), Some("[255]"));
        assert_eq!(near_end_window.first().map(|child| child.name.as_str()), Some("[384]"));
        assert_eq!(near_end_window.last().map(|child| child.name.as_str()), Some("[499]"));
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
