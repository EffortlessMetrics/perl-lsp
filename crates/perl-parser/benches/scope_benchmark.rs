use criterion::{Criterion, criterion_group, criterion_main, black_box};
use perl_parser::{Parser, ScopeAnalyzer};

const MANY_DECLARATIONS: &str = r#"
sub test {
    my $a = 1; my $b = 2; my $c = 3; my $d = 4; my $e = 5;
    my $f = 6; my $g = 7; my $h = 8; my $i = 9; my $j = 10;
    my $k = 11; my $l = 12; my $m = 13; my $n = 14; my $o = 15;
    my $p = 16; my $q = 17; my $r = 18; my $s = 19; my $t = 20;

    my ($aa, $ab, $ac, $ad, $ae);
    my ($af, $ag, $ah, $ai, $aj);

    return $a + $b;
}
"#;

fn benchmark_scope_analysis(c: &mut Criterion) {
    let mut parser = Parser::new(MANY_DECLARATIONS);
    let ast = parser.parse().unwrap();
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![];

    c.bench_function("scope_analysis_declarations", |b| {
        b.iter(|| {
            analyzer.analyze(black_box(&ast), black_box(MANY_DECLARATIONS), black_box(&pragma_map))
        });
    });
}

criterion_group!(benches, benchmark_scope_analysis);
criterion_main!(benches);
