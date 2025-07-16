#[cfg(test)]
mod performance {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use crate::test_harness::parse_perl_code;

    // Generate a simple Perl script of given size
    fn generate_perl_script(size: usize) -> String {
        let mut script = String::new();
        for i in 0..size {
            script.push_str(&format!("my $var{} = {};\n", i, i));
        }
        script
    }

    // Generate a complex Perl script with various constructs
    fn generate_complex_perl_script(size: usize) -> String {
        let mut script = String::new();
        script.push_str("package MyPackage;\n");
        script.push_str("use strict;\nuse warnings;\n\n");
        
        for i in 0..size {
            script.push_str(&format!("sub function{} {{\n", i));
            script.push_str(&format!("    my $var = {};\n", i));
            script.push_str(&format!("    if ($var > 0) {{\n"));
            script.push_str(&format!("        return $var * 2;\n"));
            script.push_str(&format!("    }} else {{\n"));
            script.push_str(&format!("        return 0;\n"));
            script.push_str(&format!("    }}\n"));
            script.push_str(&format!("}}\n\n"));
        }
        
        script.push_str("my $result = 0;\n");
        script.push_str("for my $i (1..100) {\n");
        script.push_str("    $result += function0($i);\n");
        script.push_str("}\n");
        script.push_str("print $result;\n");
        
        script
    }

    // Generate a Perl script with many string literals
    fn generate_string_heavy_script(size: usize) -> String {
        let mut script = String::new();
        for i in 0..size {
            script.push_str(&format!("my $str{} = \"String number {} with some content\";\n", i, i));
            script.push_str(&format!("my $str{}_single = 'String {} with single quotes';\n", i, i));
        }
        script
    }

    // Generate a Perl script with many regex patterns
    fn generate_regex_heavy_script(size: usize) -> String {
        let mut script = String::new();
        for i in 0..size {
            script.push_str(&format!("my $regex{} = qr/pattern{}/i;\n", i, i));
            script.push_str(&format!("if ($str =~ /match{}/) {{\n", i));
            script.push_str(&format!("    print \"Matched {};\";\n", i));
            script.push_str(&format!("}}\n"));
        }
        script
    }

    // Generate a Perl script with many heredoc constructs
    fn generate_heredoc_heavy_script(size: usize) -> String {
        let mut script = String::new();
        for i in 0..size {
            script.push_str(&format!("my $heredoc{} = <<EOF;\n", i));
            script.push_str(&format!("This is heredoc number {}\n", i));
            script.push_str(&format!("It contains multiple lines\n"));
            script.push_str(&format!("With some content\n"));
            script.push_str(&format!("EOF\n"));
        }
        script
    }

    // Benchmark simple variable declarations
    fn bench_simple_variables(c: &mut Criterion) {
        let mut group = c.benchmark_group("simple_variables");
        
        for size in [10, 100, 1000, 10000] {
            let script = generate_perl_script(size);
            group.bench_function(&format!("{}_variables", size), |b| {
                b.iter(|| parse_perl_code(black_box(&script)))
            });
        }
        
        group.finish();
    }

    // Benchmark complex function definitions
    fn bench_complex_functions(c: &mut Criterion) {
        let mut group = c.benchmark_group("complex_functions");
        
        for size in [10, 50, 100, 200] {
            let script = generate_complex_perl_script(size);
            group.bench_function(&format!("{}_functions", size), |b| {
                b.iter(|| parse_perl_code(black_box(&script)))
            });
        }
        
        group.finish();
    }

    // Benchmark string-heavy scripts
    fn bench_string_heavy(c: &mut Criterion) {
        let mut group = c.benchmark_group("string_heavy");
        
        for size in [10, 100, 1000, 5000] {
            let script = generate_string_heavy_script(size);
            group.bench_function(&format!("{}_strings", size), |b| {
                b.iter(|| parse_perl_code(black_box(&script)))
            });
        }
        
        group.finish();
    }

    // Benchmark regex-heavy scripts
    fn bench_regex_heavy(c: &mut Criterion) {
        let mut group = c.benchmark_group("regex_heavy");
        
        for size in [10, 50, 100, 200] {
            let script = generate_regex_heavy_script(size);
            group.bench_function(&format!("{}_regexes", size), |b| {
                b.iter(|| parse_perl_code(black_box(&script)))
            });
        }
        
        group.finish();
    }

    // Benchmark heredoc-heavy scripts
    fn bench_heredoc_heavy(c: &mut Criterion) {
        let mut group = c.benchmark_group("heredoc_heavy");
        
        for size in [10, 50, 100, 200] {
            let script = generate_heredoc_heavy_script(size);
            group.bench_function(&format!("{}_heredocs", size), |b| {
                b.iter(|| parse_perl_code(black_box(&script)))
            });
        }
        
        group.finish();
    }

    // Benchmark specific Perl constructs
    fn bench_specific_constructs(c: &mut Criterion) {
        let mut group = c.benchmark_group("specific_constructs");
        
        // Variable declarations
        group.bench_function("variable_declarations", |b| {
            let code = "my $var1 = 1; my $var2 = 2; my $var3 = 3; my $var4 = 4; my $var5 = 5;";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Function calls
        group.bench_function("function_calls", |b| {
            let code = "print 'Hello'; print 'World'; print 'Test'; print 'Code'; print 'Benchmark';";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Control structures
        group.bench_function("control_structures", |b| {
            let code = "if ($x) { print $x; } elsif ($y) { print $y; } else { print 'default'; }";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Loops
        group.bench_function("loops", |b| {
            let code = "for my $i (1..10) { print $i; } while ($x) { $x--; }";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // String literals
        group.bench_function("string_literals", |b| {
            let code = r#"my $str1 = "Hello"; my $str2 = 'World'; my $str3 = q(Test); my $str4 = qq(Code);"#;
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Regex patterns
        group.bench_function("regex_patterns", |b| {
            let code = r#"my $regex1 = qr/pattern1/; my $regex2 = qr/pattern2/i; if ($str =~ /match/) { }"#;
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Heredoc
        group.bench_function("heredoc", |b| {
            let code = r#"my $heredoc = <<EOF;
This is a heredoc
with multiple lines
EOF"#;
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        group.finish();
    }

    // Benchmark error handling
    fn bench_error_handling(c: &mut Criterion) {
        let mut group = c.benchmark_group("error_handling");
        
        // Unterminated string
        group.bench_function("unterminated_string", |b| {
            let code = r#"my $str = "Hello, World!;"#;
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Unterminated block
        group.bench_function("unterminated_block", |b| {
            let code = "sub foo {";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Malformed expression
        group.bench_function("malformed_expression", |b| {
            let code = "my $var = ;";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        group.finish();
    }

    // Benchmark Unicode handling
    fn bench_unicode_handling(c: &mut Criterion) {
        let mut group = c.benchmark_group("unicode_handling");
        
        // Unicode identifiers
        group.bench_function("unicode_identifiers", |b| {
            let code = "my $変数 = 42; my $über = 'cool'; my $café = 'coffee';";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Unicode strings
        group.bench_function("unicode_strings", |b| {
            let code = r#"my $msg = "Hello, 世界!"; my $msg2 = 'こんにちは';"#;
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        // Unicode comments
        group.bench_function("unicode_comments", |b| {
            let code = "my $var = 1; # This is a comment with 日本語";
            b.iter(|| parse_perl_code(black_box(code)))
        });
        
        group.finish();
    }

    // Benchmark large files
    fn bench_large_files(c: &mut Criterion) {
        let mut group = c.benchmark_group("large_files");
        
        // Large simple file
        group.bench_function("large_simple", |b| {
            let script = generate_perl_script(10000);
            b.iter(|| parse_perl_code(black_box(&script)))
        });
        
        // Large complex file
        group.bench_function("large_complex", |b| {
            let script = generate_complex_perl_script(500);
            b.iter(|| parse_perl_code(black_box(&script)))
        });
        
        group.finish();
    }

    // Run all benchmarks
    criterion_group!(
        benches,
        bench_simple_variables,
        bench_complex_functions,
        bench_string_heavy,
        bench_regex_heavy,
        bench_heredoc_heavy,
        bench_specific_constructs,
        bench_error_handling,
        bench_unicode_handling,
        bench_large_files,
    );
    
    criterion_main!(benches);
} 