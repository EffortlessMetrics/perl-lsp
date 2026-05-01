use super::*;
use crate::parser::Parser;

#[test]
fn test_call_hierarchy_prepare() {
    let code = r#"
sub main {
    helper();
    process_data();
}

sub helper {
    print "Helper\n";
}

sub process_data {
    helper();
}
"#;

    let mut parser = Parser::new(code);
    if let Ok(ast) = parser.parse() {
        let provider = CallHierarchyProvider::new(code.to_string(), "file:///test.pl".to_string());

        // Find function at position (line 1, char 5) - "main"
        let items = provider.prepare(&ast, 1, 5);
        assert!(items.is_some());
        let items = items.ok_or("expected items").map_err(|e| e.to_string());
        if let Ok(items) = items {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].name, "main");
        }
    }
}

#[test]
fn test_incoming_calls() {
    let code = r#"
sub caller1 {
    target_func();
}

sub caller2 {
    target_func();
    target_func(); # called twice
}

sub target_func {
    print "Target\n";
}
"#;

    let mut parser = Parser::new(code);
    if let Ok(ast) = parser.parse() {
        let provider = CallHierarchyProvider::new(code.to_string(), "file:///test.pl".to_string());

        let target_item = CallHierarchyItem {
            name: "target_func".to_string(),
            kind: "function".to_string(),
            uri: "file:///test.pl".to_string(),
            range: Range {
                start: Position { line: 10, character: 0 },
                end: Position { line: 12, character: 1 },
            },
            selection_range: Range {
                start: Position { line: 10, character: 4 },
                end: Position { line: 10, character: 15 },
            },
            detail: None,
            package_name: None,
            qualified_name: None,
        };

        let incoming = provider.incoming_calls(&ast, &target_item);
        assert_eq!(incoming.len(), 2);

        // Check callers
        let caller_names: Vec<_> = incoming.iter().map(|c| &c.from.name).collect();
        assert!(caller_names.contains(&&"caller1".to_string()));
        assert!(caller_names.contains(&&"caller2".to_string()));

        // caller2 should have 2 ranges (called twice)
        let caller2_opt = incoming.iter().find(|c| c.from.name == "caller2");
        assert!(caller2_opt.is_some(), "caller2 not found in incoming calls");
        if let Some(caller2) = caller2_opt {
            assert_eq!(caller2.from_ranges.len(), 2);
        }
    }
}

#[test]
fn test_outgoing_calls() {
    let code = r#"
sub main {
    helper();
    process_data();
    $obj->method_call();
}

sub helper {
    print "Helper\n";
}
"#;

    let mut parser = Parser::new(code);
    if let Ok(ast) = parser.parse() {
        let provider = CallHierarchyProvider::new(code.to_string(), "file:///test.pl".to_string());

        let main_item = CallHierarchyItem {
            name: "main".to_string(),
            kind: "function".to_string(),
            uri: "file:///test.pl".to_string(),
            range: Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 5, character: 1 },
            },
            selection_range: Range {
                start: Position { line: 1, character: 4 },
                end: Position { line: 1, character: 8 },
            },
            detail: None,
            package_name: None,
            qualified_name: None,
        };

        let outgoing = provider.outgoing_calls(&ast, &main_item);
        assert_eq!(outgoing.len(), 3);

        // Check called functions
        let called_names: Vec<_> = outgoing.iter().map(|c| &c.to.name).collect();
        assert!(called_names.contains(&&"helper".to_string()));
        assert!(called_names.contains(&&"process_data".to_string()));
        assert!(called_names.contains(&&"method_call".to_string()));
    }
}
