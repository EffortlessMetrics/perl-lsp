#![allow(clippy::expect_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use perl_parser::{Parser, ScopeAnalyzer};
use std::hint::black_box;

const MANY_VARS_SCRIPT: &str = r#"
my $var1 = 1;
my $var2 = 2;
my $var3 = 3;
my $var4 = 4;
my $var5 = 5;
my $var6 = 6;
my $var7 = 7;
my $var8 = 8;
my $var9 = 9;
my $var10 = 10;
my $user_count = 100;
my $item_index = 0;
my $is_valid = 1;
my $config_path = "/tmp";
my $temp_file = "temp.txt";

$var1 = $var2 + $var3;
$var4 = $var5 + $var6;
$var7 = $var8 + $var9;
$user_count++;
$item_index++;
$is_valid = 0;
print $config_path;
print $temp_file;
"#;

fn benchmark_scope_analysis(c: &mut Criterion) {
    // Generate a larger script with many variable usages to stress is_builtin_global
    let mut script = String::from(MANY_VARS_SCRIPT);
    for _ in 0..100 {
        script.push_str(MANY_VARS_SCRIPT);
    }

    let mut parser = Parser::new(&script);
    let ast = parser.parse().expect("Failed to parse");
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![];

    c.bench_function("scope_analysis_many_vars", |b| {
        b.iter(|| {
            let _ = analyzer.analyze(black_box(&ast), black_box(&script), &pragma_map);
        });
    });
}

fn benchmark_scope_analysis_barewords(c: &mut Criterion) {
    let mut script = String::from("use strict;\n");
    for i in 0..5000 {
        // Generate many class method calls to stress bareword detection
        // These will hit NodeKind::Identifier and call is_known_function
        script.push_str(&format!("MyClass{}->method();\n", i));
        script.push_str(&format!("AnotherClass{}->static_method();\n", i));
    }

    let mut parser = Parser::new(&script);
    let ast = parser.parse().expect("Failed to parse");
    let analyzer = ScopeAnalyzer::new();
    // Enable strict mode via pragma map
    let pragma_map = vec![
        (0..script.len(), perl_parser::PragmaState { strict_subs: true, ..Default::default() })
    ];

    c.bench_function("scope_analysis_barewords", |b| {
        b.iter(|| {
            let _ = analyzer.analyze(black_box(&ast), black_box(&script), &pragma_map);
        });
    });
}

criterion_group!(benches, benchmark_scope_analysis, benchmark_scope_analysis_barewords);
criterion_main!(benches);
