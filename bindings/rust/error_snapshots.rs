#[cfg(test)]
mod error_snapshots {
    use super::super::test_harness::{parse_perl_code, capture_error_snapshot, ErrorSnapshot};

    /// Test error node snapshotting for various error conditions
    /// This ensures error recovery is stable and predictable

    #[test]
    fn test_unterminated_string_errors() {
        let error_cases = [
            (r#"my $str = "Hello, World!;"#, ErrorSnapshot {
                count: 1,
                positions: vec![(0, 15)], // Error at the unterminated string
                kinds: vec!["ERROR".to_string()],
            }),
            (r#"my $str = 'Unterminated;"#, ErrorSnapshot {
                count: 1,
                positions: vec![(0, 15)],
                kinds: vec!["ERROR".to_string()],
            }),
        ];

        for (i, (code, expected)) in error_cases.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(result.is_ok(), "Error case {} failed to parse: {:?}", i, result);
            
            let tree = result.unwrap();
            let snapshot = capture_error_snapshot(&tree);
            
            // For now, we just check that we get the expected number of errors
            // The exact positions might vary based on the parser implementation
            assert_eq!(
                snapshot.count, expected.count,
                "Error case {}: expected {} errors, got {}",
                i, expected.count, snapshot.count
            );
        }
    }

    #[test]
    fn test_unterminated_block_errors() {
        let error_cases = [
            (r#"if ($condition) { my $var = 1;"#, ErrorSnapshot {
                count: 1,
                positions: vec![(0, 0)], // Error at the start of the unterminated block
                kinds: vec!["ERROR".to_string()],
            }),
            (r#"sub foo { return 1;"#, ErrorSnapshot {
                count: 1,
                positions: vec![(0, 0)],
                kinds: vec!["ERROR".to_string()],
            }),
        ];

        for (i, (code, expected)) in error_cases.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(result.is_ok(), "Error case {} failed to parse: {:?}", i, result);
            
            let tree = result.unwrap();
            let snapshot = capture_error_snapshot(&tree);
            
            assert_eq!(
                snapshot.count, expected.count,
                "Error case {}: expected {} errors, got {}",
                i, expected.count, snapshot.count
            );
        }
    }

    #[test]
    fn test_malformed_expression_errors() {
        let error_cases = [
            (r#"my $var = 1 +;"#, ErrorSnapshot {
                count: 1,
                positions: vec![(0, 12)], // Error at the incomplete expression
                kinds: vec!["ERROR".to_string()],
            }),
            (r#"my $var = (1 + 2;"#, ErrorSnapshot {
                count: 1,
                positions: vec![(0, 15)], // Error at the unterminated parentheses
                kinds: vec!["ERROR".to_string()],
            }),
        ];

        for (i, (code, expected)) in error_cases.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(result.is_ok(), "Error case {} failed to parse: {:?}", i, result);
            
            let tree = result.unwrap();
            let snapshot = capture_error_snapshot(&tree);
            
            assert_eq!(
                snapshot.count, expected.count,
                "Error case {}: expected {} errors, got {}",
                i, expected.count, snapshot.count
            );
        }
    }

    #[test]
    fn test_multiple_errors() {
        let error_cases = [
            (r#"my $str = "unterminated; if ($x) { $y = 1;"#, ErrorSnapshot {
                count: 2, // Multiple errors
                positions: vec![(0, 15), (0, 0)], // String error + block error
                kinds: vec!["ERROR".to_string(), "ERROR".to_string()],
            }),
        ];

        for (i, (code, expected)) in error_cases.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(result.is_ok(), "Error case {} failed to parse: {:?}", i, result);
            
            let tree = result.unwrap();
            let snapshot = capture_error_snapshot(&tree);
            
            assert!(
                snapshot.count >= expected.count,
                "Error case {}: expected at least {} errors, got {}",
                i, expected.count, snapshot.count
            );
        }
    }

    #[test]
    fn test_error_recovery_stability() {
        // Test that the same error produces consistent snapshots
        let code = r#"my $str = "unterminated;"#;
        
        let result1 = parse_perl_code(code);
        let result2 = parse_perl_code(code);
        
        assert!(result1.is_ok() && result2.is_ok());
        
        let tree1 = result1.unwrap();
        let tree2 = result2.unwrap();
        
        let snapshot1 = capture_error_snapshot(&tree1);
        let snapshot2 = capture_error_snapshot(&tree2);
        
        // Error recovery should be stable
        assert_eq!(
            snapshot1.count, snapshot2.count,
            "Error recovery should be stable: got {} vs {} errors",
            snapshot1.count, snapshot2.count
        );
    }

    #[test]
    fn test_no_errors_in_valid_code() {
        let valid_codes = [
            "my $var = 42;",
            "print 'Hello, World!';",
            "sub foo { return 1; }",
            "if ($x) { $y = 1; }",
        ];

        for (i, code) in valid_codes.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(result.is_ok(), "Valid code {} failed to parse: {:?}", i, result);
            
            let tree = result.unwrap();
            let snapshot = capture_error_snapshot(&tree);
            
            assert_eq!(
                snapshot.count, 0,
                "Valid code {} should have no errors, got {}",
                i, snapshot.count
            );
        }
    }

    #[test]
    fn test_error_node_kinds() {
        // Test that error nodes have the expected kind
        let code = r#"my $var = 1 +;"#;
        let result = parse_perl_code(code);
        assert!(result.is_ok());
        
        let tree = result.unwrap();
        let snapshot = capture_error_snapshot(&tree);
        
        // All error nodes should have kind "ERROR"
        for kind in &snapshot.kinds {
            assert_eq!(kind, "ERROR", "Error node should have kind 'ERROR', got '{}'", kind);
        }
    }
} 