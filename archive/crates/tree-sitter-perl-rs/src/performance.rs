#[cfg(test)]
mod performance {
    use std::time::Instant;
    use perl_tdd_support::{must, must_some};
    use crate::{parse, language, scanner::RustScanner};
    use tree_sitter::{Parser, Query, QueryCursor};

    #[test]
    fn test_parse_performance_basic() {
        let test_cases = vec![
            "my $var = 42;",
            "print 'Hello, World!';",
            "sub foo { return 1; }",
            "if ($x) { $y = 1; }",
            "for my $i (1..10) { print $i; }",
        ];

        let iterations = 1000;
        let mut total_time = 0u128;

        for code in test_cases {
            let start = Instant::now();
            for _ in 0..iterations {
                let _result = parse(code);
            }
            let duration = start.elapsed();
            total_time += duration.as_micros();
        }

        let avg_time = total_time as f64 / (test_cases.len() * iterations) as f64;
        println!("Basic parse performance: {:.2} μs per parse", avg_time);
        
        // Ensure parsing is reasonably fast
        assert!(avg_time < 100.0, "Basic parsing too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_parse_performance_complex() {
        let complex_code = r#"
            sub fibonacci {
                my ($n) = @_;
                if ($n <= 1) {
                    return $n;
                }
                return fibonacci($n - 1) + fibonacci($n - 2);
            }

            sub factorial {
                my ($n) = @_;
                if ($n <= 1) {
                    return 1;
                }
                return $n * factorial($n - 1);
            }

            my $result = fibonacci(10) + factorial(5);
            print "Result: $result\n";
        "#;

        let iterations = 100;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _result = parse(complex_code);
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Complex parse performance: {:.2} μs per parse", avg_time);
        
        // Ensure complex parsing is reasonably fast
        assert!(avg_time < 500.0, "Complex parsing too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_scanner_performance() {
        let test_inputs = vec![
            b"my $variable = 42;",
            b"print 'Hello, World!';",
            b"sub function { return 1; }",
            b"if ($condition) { $action = 1; }",
            b"for my $i (1..10) { print $i; }",
        ];

        let iterations = 1000;
        let mut total_time = 0u128;

        for input in test_inputs {
            let mut scanner = RustScanner::new();
            let start = Instant::now();
            
            for _ in 0..iterations {
                let _result = scanner.scan(input);
            }
            
            let duration = start.elapsed();
            total_time += duration.as_micros();
        }

        let avg_time = total_time as f64 / (test_inputs.len() * iterations) as f64;
        println!("Scanner performance: {:.2} μs per scan", avg_time);
        
        // Ensure scanning is reasonably fast
        assert!(avg_time < 50.0, "Scanning too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_scanner_performance_large_input() {
        let large_input = b"my $very_long_variable_name_that_goes_on_and_on = 42; print 'This is a very long string that contains many characters and should test the scanner performance with larger inputs'; sub very_long_function_name { my $local_var = 1; return $local_var + 1; }";

        let iterations = 1000;
        let mut scanner = RustScanner::new();
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _result = scanner.scan(large_input);
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Large input scanner performance: {:.2} μs per scan", avg_time);
        
        // Ensure large input scanning is reasonably fast
        assert!(avg_time < 100.0, "Large input scanning too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_highlight_performance() {
        let test_code = r#"
            my $variable = 42;
            print 'Hello, World!';
            sub function { return 1; }
            if ($condition) { $action = 1; }
            for my $i (1..10) { print $i; }
        "#;

        let mut parser = Parser::new();
        must(parser.set_language(&language()));
        
        let query = must(Query::new(&language(), "(variable_declaration) @variable"));
        let mut cursor = QueryCursor::new();

        let iterations = 100;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let tree = must_some(parser.parse(test_code, None));
            let _captures = cursor.captures(&query, tree.root_node(), test_code.as_bytes());
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Highlight performance: {:.2} μs per highlight", avg_time);
        
        // Ensure highlighting is reasonably fast
        assert!(avg_time < 200.0, "Highlighting too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_parser_reuse_performance() {
        let test_cases = vec![
            "my $var1 = 1;",
            "my $var2 = 2;",
            "my $var3 = 3;",
            "my $var4 = 4;",
            "my $var5 = 5;",
        ];

        let iterations = 100;
        let mut parser = Parser::new();
        must(parser.set_language(&language()));

        let start = Instant::now();
        
        for _ in 0..iterations {
            for code in &test_cases {
                let _tree = parser.parse(code, None);
            }
        }
        
        let duration = start.elapsed();
        let total_parses = iterations * test_cases.len();
        let avg_time = duration.as_micros() as f64 / total_parses as f64;
        println!("Parser reuse performance: {:.2} μs per parse", avg_time);
        
        // Ensure parser reuse is efficient
        assert!(avg_time < 50.0, "Parser reuse too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_memory_performance() {
        let large_code = r#"
            sub fibonacci {
                my ($n) = @_;
                if ($n <= 1) {
                    return $n;
                }
                return fibonacci($n - 1) + fibonacci($n - 2);
            }

            sub factorial {
                my ($n) = @_;
                if ($n <= 1) {
                    return 1;
                }
                return $n * factorial($n - 1);
            }

            my $result = fibonacci(10) + factorial(5);
            print "Result: $result\n";

            # Add more code to test memory usage
            my @array = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
            my %hash = (key1 => 'value1', key2 => 'value2', key3 => 'value3');
            
            for my $item (@array) {
                print "Item: $item\n";
            }
            
            while (my ($key, $value) = each %hash) {
                print "$key => $value\n";
            }
        "#;

        let iterations = 100;
        let mut parser = Parser::new();
        must(parser.set_language(&language()));

        let start = Instant::now();
        
        for _ in 0..iterations {
            let tree = must_some(parser.parse(large_code, None));
            let node_count = count_nodes(&tree.root_node());
            
            // Ensure reasonable memory usage
            assert!(node_count < 1000, "Too many nodes: {}", node_count);
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Memory performance: {:.2} μs per parse", avg_time);
        
        // Ensure memory usage is reasonable
        assert!(avg_time < 1000.0, "Memory performance too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_unicode_performance() {
        let unicode_code = r#"
            my $変数 = '値';
            my $über = 'cool';
            my $naïve = 'simple';
            sub 関数 { return '関数です'; }
            my $привет = 'hello';
            my $你好 = 'hello';
            my $안녕 = 'hello';
        "#;

        let iterations = 100;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _result = parse(unicode_code);
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Unicode performance: {:.2} μs per parse", avg_time);
        
        // Ensure Unicode parsing is reasonably fast
        assert!(avg_time < 200.0, "Unicode parsing too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_error_recovery_performance() {
        let error_cases = vec![
            "my $str = \"Unterminated string;",
            "if ($condition { $action = 1; }",
            "my $var = 1 +;",
            "sub foo { return 1; # Missing closing brace",
            "print 'Hello'; # Valid but with error",
        ];

        let iterations = 100;
        let mut total_time = 0u128;

        for code in error_cases {
            let start = Instant::now();
            for _ in 0..iterations {
                let _result = parse(code);
            }
            let duration = start.elapsed();
            total_time += duration.as_micros();
        }

        let avg_time = total_time as f64 / (error_cases.len() * iterations) as f64;
        println!("Error recovery performance: {:.2} μs per parse", avg_time);
        
        // Ensure error recovery is reasonably fast
        assert!(avg_time < 150.0, "Error recovery too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_concurrent_parsing_performance() {
        use std::thread;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let test_code = "my $var = 42; print 'Hello, World!';";
        let iterations = 1000;
        let thread_count = 4;
        let counter = Arc::new(AtomicUsize::new(0));

        let start = Instant::now();
        
        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let counter = Arc::clone(&counter);
                thread::spawn(move || {
                    for _ in 0..iterations {
                        let _result = parse(test_code);
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();

        for handle in handles {
            must(handle.join());
        }
        
        let duration = start.elapsed();
        let total_parses = thread_count * iterations;
        let avg_time = duration.as_micros() as f64 / total_parses as f64;
        println!("Concurrent parsing performance: {:.2} μs per parse", avg_time);
        println!("Total parses: {}", counter.load(Ordering::Relaxed));
        
        // Ensure concurrent parsing is reasonably fast
        assert!(avg_time < 100.0, "Concurrent parsing too slow: {:.2} μs", avg_time);
    }

    // Helper function
    fn count_nodes(node: &tree_sitter::Node) -> usize {
        let mut count = 1;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += count_nodes(&child);
            }
        }
        count
    }
} 