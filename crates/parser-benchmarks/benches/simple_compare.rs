use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;

const SIMPLE_CODE: &str = "my $x = 42; print $x;";
const POSTFIX_DEREF: &str = "my %slice = $hashref->%{'foo', 'bar'};";

fn bench_parsers(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser-comparison");
    group.measurement_time(Duration::from_secs(5));

    // Benchmark perl-parser
    group.bench_function("perl-parser/simple", |b| {
        b.iter(|| {
            use perl_parser::Parser;
            let mut parser = Parser::new(black_box(SIMPLE_CODE));
            let _ = parser.parse();
        });
    });

    group.bench_function("perl-parser/postfix-deref", |b| {
        b.iter(|| {
            use perl_parser::Parser;
            let mut parser = Parser::new(black_box(POSTFIX_DEREF));
            let _ = parser.parse();
        });
    });

    // Benchmark tree-sitter-perl-c
    group.bench_function("tree-sitter-c/simple", |b| {
        b.iter(|| {
            use tree_sitter_perl_c::create_parser;
            let mut parser = create_parser();
            let _ = parser.parse(black_box(SIMPLE_CODE), None);
        });
    });

    group.bench_function("tree-sitter-c/postfix-deref", |b| {
        b.iter(|| {
            use tree_sitter_perl_c::create_parser;
            let mut parser = create_parser();
            let _ = parser.parse(black_box(POSTFIX_DEREF), None);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_parsers);
criterion_main!(benches);
