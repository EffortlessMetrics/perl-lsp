use perl_semantic_analyzer::scope_analyzer::{ScopeAnalyzer, IssueKind};
use perl_parser_core::{Node, NodeKind, SourceLocation};
use std::time::Instant;

// Minimal AST construction helpers
fn create_var_decl(sigil: &str, name: &str, start: usize) -> Node {
    Node {
        kind: NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(Node {
                kind: NodeKind::Variable {
                    sigil: sigil.to_string(),
                    name: name.to_string(),
                },
                location: SourceLocation { start, end: start + name.len() + 1 },
            }),
            attributes: vec![],
            initializer: None,
        },
        location: SourceLocation { start, end: start + name.len() + 4 }, // my $var
    }
}

fn main() {
    let mut declarations = Vec::new();
    let count = 100_000;

    // Create a large AST with many variable declarations
    for i in 0..count {
        declarations.push(create_var_decl("$", &format!("var_{}", i), i * 10));
    }

    let root = Node {
        kind: NodeKind::Block { statements: declarations },
        location: SourceLocation { start: 0, end: count * 10 },
    };

    let code = "my $var_0; ".repeat(count);
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![];

    println!("Starting analysis of {} declarations...", count);
    let start = Instant::now();
    let issues = analyzer.analyze(&root, &code, &pragma_map);
    let duration = start.elapsed();

    println!("Analysis took: {:?}", duration);
    println!("Issues found: {}", issues.len());
}
