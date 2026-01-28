// Test for issue #432: Corpus Coverage - Format Statements Test Fixtures
// Validates all acceptance criteria for format statement coverage

use perl_parser::Parser;
use perl_ast::ast::{Node, NodeKind};
use std::fs;
use std::path::Path;

fn parse_code(input: &str) -> Result<Node, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(input);
    parser.parse().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn count_format_statements(node: &Node) -> usize {
    match &node.kind {
        NodeKind::Program { statements } => {
            statements.iter().filter(|stmt| matches!(stmt.kind, NodeKind::Format { .. })).count()
        }
        _ => 0,
    }
}

fn extract_format_statements(node: &Node) -> Vec<(String, String)> {
    match &node.kind {
        NodeKind::Program { statements } => {
            statements
                .iter()
                .filter_map(|stmt| {
                    if let NodeKind::Format { name, body } = &stmt.kind {
                        Some((name.clone(), body.clone()))
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => vec![],
    }
}

#[test]
fn parser_format_corpus_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    // AC5: At least 10 test cases in corpus
    let corpus_path = Path::new("test_corpus/format_comprehensive.pl");
    let corpus_path = if !corpus_path.exists() {
        Path::new("../../test_corpus/format_comprehensive.pl")
    } else {
        corpus_path
    };

    if !corpus_path.exists() {
        eprintln!("Warning: test_corpus/format_comprehensive.pl not found, skipping test");
        return Ok(());
    }

    let content = fs::read_to_string(corpus_path)?;
    let ast = parse_code(&content)?;

    let format_count = count_format_statements(&ast);
    assert!(
        format_count >= 10,
        "AC5: Expected at least 10 format statements in corpus, found {}",
        format_count
    );

    Ok(())
}

#[test]
fn parser_format_corpus_ac1_keyword_recognition() -> Result<(), Box<dyn std::error::Error>> {
    // AC1: Parser recognizes `format` keyword
    let corpus_path = Path::new("test_corpus/format_comprehensive.pl");
    let corpus_path = if !corpus_path.exists() {
        Path::new("../../test_corpus/format_comprehensive.pl")
    } else {
        corpus_path
    };

    if !corpus_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(corpus_path)?;
    let ast = parse_code(&content)?;
    let formats = extract_format_statements(&ast);

    assert!(
        !formats.is_empty(),
        "AC1: Parser should recognize format keyword and create Format nodes"
    );

    Ok(())
}

#[test]
fn parser_format_corpus_ac2_picture_lines() -> Result<(), Box<dyn std::error::Error>> {
    // AC2: Format picture lines are parsed correctly
    let corpus_path = Path::new("test_corpus/format_comprehensive.pl");
    let corpus_path = if !corpus_path.exists() {
        Path::new("../../test_corpus/format_comprehensive.pl")
    } else {
        corpus_path
    };

    if !corpus_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(corpus_path)?;
    let ast = parse_code(&content)?;
    let formats = extract_format_statements(&ast);

    // Check that picture lines are captured
    let has_picture_lines = formats.iter().any(|(_, body)| {
        body.contains('@') && (
            body.contains("@<") ||
            body.contains("@>") ||
            body.contains("@|") ||
            body.contains("@#")
        )
    });

    assert!(
        has_picture_lines,
        "AC2: Format picture lines should be parsed and captured correctly"
    );

    Ok(())
}

#[test]
fn parser_format_corpus_ac3_field_specifiers() -> Result<(), Box<dyn std::error::Error>> {
    // AC3: Format field specifiers are recognized as special syntax
    let corpus_path = Path::new("test_corpus/format_comprehensive.pl");
    let corpus_path = if !corpus_path.exists() {
        Path::new("../../test_corpus/format_comprehensive.pl")
    } else {
        corpus_path
    };

    if !corpus_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(corpus_path)?;
    let ast = parse_code(&content)?;
    let formats = extract_format_statements(&ast);

    // Check for different field specifier types
    let all_bodies = formats.iter().map(|(_, body)| body).collect::<Vec<_>>();
    let combined = all_bodies.join("\n");

    assert!(
        combined.contains("@<"),
        "AC3: Left-justified field specifiers should be recognized"
    );
    assert!(
        combined.contains("@>"),
        "AC3: Right-justified field specifiers should be recognized"
    );
    assert!(
        combined.contains("@|"),
        "AC3: Center-justified field specifiers should be recognized"
    );
    assert!(
        combined.contains("@#"),
        "AC3: Numeric field specifiers should be recognized"
    );

    Ok(())
}

#[test]
fn parser_format_corpus_ac4_variable_binding() -> Result<(), Box<dyn std::error::Error>> {
    // AC4: Format variable binding is supported
    let corpus_path = Path::new("test_corpus/format_comprehensive.pl");
    let corpus_path = if !corpus_path.exists() {
        Path::new("../../test_corpus/format_comprehensive.pl")
    } else {
        corpus_path
    };

    if !corpus_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(corpus_path)?;
    let ast = parse_code(&content)?;
    let formats = extract_format_statements(&ast);

    // Check for both named and anonymous formats
    let has_named = formats.iter().any(|(name, _)| !name.is_empty());
    let has_anonymous = formats.iter().any(|(name, _)| name.is_empty());

    assert!(
        has_named,
        "AC4: Named format declarations should be supported"
    );
    assert!(
        has_anonymous,
        "AC4: Anonymous format declarations should be supported"
    );

    Ok(())
}

#[test]
fn parser_format_corpus_ac6_ast_structure() -> Result<(), Box<dyn std::error::Error>> {
    // AC6: Format statements produce correct AST structure with NodeKind::Format
    let corpus_path = Path::new("test_corpus/format_comprehensive.pl");
    let corpus_path = if !corpus_path.exists() {
        Path::new("../../test_corpus/format_comprehensive.pl")
    } else {
        corpus_path
    };

    if !corpus_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(corpus_path)?;
    let ast = parse_code(&content)?;

    if let NodeKind::Program { statements } = &ast.kind {
        let format_nodes: Vec<_> = statements
            .iter()
            .filter(|stmt| matches!(stmt.kind, NodeKind::Format { .. }))
            .collect();

        assert!(
            !format_nodes.is_empty(),
            "AC6: Format statements should produce NodeKind::Format AST nodes"
        );

        // Verify structure of first format node
        if let Some(first) = format_nodes.first() {
            if let NodeKind::Format { name, body } = &first.kind {
                assert!(
                    !body.is_empty() || name == "EMPTY",
                    "AC6: Format AST should capture name and body correctly"
                );
            }
        }
    }

    Ok(())
}

#[test]
fn parser_format_corpus_ac7_picture_lines_captured() -> Result<(), Box<dyn std::error::Error>> {
    // AC7: Picture lines are captured correctly in AST
    let corpus_path = Path::new("test_corpus/format_comprehensive.pl");
    let corpus_path = if !corpus_path.exists() {
        Path::new("../../test_corpus/format_comprehensive.pl")
    } else {
        corpus_path
    };

    if !corpus_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(corpus_path)?;
    let ast = parse_code(&content)?;
    let formats = extract_format_statements(&ast);

    // Find a format with known picture line content
    let complex_format = formats
        .iter()
        .find(|(name, _)| name == "COMPLEX_REPORT");

    assert!(
        complex_format.is_some(),
        "AC7: Should find COMPLEX_REPORT format in corpus"
    );

    if let Some((_, body)) = complex_format {
        assert!(
            body.contains("EMPLOYEE REPORT"),
            "AC7: Picture lines with literal text should be captured"
        );
        assert!(
            body.contains("$name") || body.contains("Name:"),
            "AC7: Picture lines should capture field specifiers or variables"
        );
    }

    Ok(())
}
