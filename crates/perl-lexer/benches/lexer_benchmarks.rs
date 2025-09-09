use criterion::{Criterion, criterion_group, criterion_main};
use perl_lexer::{PerlLexer, Token};
use std::hint::black_box;

fn collect_all_tokens(mut lexer: PerlLexer) -> Vec<Token> {
    lexer.collect_tokens()
}

fn bench_simple_tokens(c: &mut Criterion) {
    let input = "my $x = 42; print $x;";

    c.bench_function("simple_tokens", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_slash_disambiguation(c: &mut Criterion) {
    let input = r#"
        my $x = 10 / 2;
        if ($str =~ /pattern/) {
            $str =~ s/foo/bar/g;
        }
        print 1/ /abc/;
    "#;

    c.bench_function("slash_disambiguation", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_string_interpolation(c: &mut Criterion) {
    let input = r#"
        my $name = "World";
        print "Hello, $name!\n";
        print "The answer is ${count + 1}\n";
        print "Array: @items\n";
    "#;

    c.bench_function("string_interpolation", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_large_file(c: &mut Criterion) {
    // Generate a large file
    let mut input = String::new();
    for i in 0..1000 {
        input.push_str(&format!("my $var{} = {};\n", i, i));
        input.push_str(&format!("print \"Value: $var{}\n\";\n", i));
        if i % 10 == 0 {
            input.push_str(&format!("if ($var{} =~ /\\d+/) {{\n", i));
            input.push_str(&format!("    $var{} = $var{} / 2;\n", i, i));
            input.push_str("}\n");
        }
    }

    c.bench_function("large_file", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(&input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_whitespace_heavy(c: &mut Criterion) {
    let input = r#"
    # This is a comment
    my   $x   =   42  ;  # Another comment
    
    print    $x    ;
    
    # More comments
    "#;

    c.bench_function("whitespace_heavy", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_operator_heavy(c: &mut Criterion) {
    let input = "$a += $b -= $c *= $d /= $e %= $f **= $g &&= $h ||= $i //= $j";

    c.bench_function("operator_heavy", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_number_parsing(c: &mut Criterion) {
    let input = "123 456.789 1_234_567 1.23e45 0xFF 0377 0b1010";

    c.bench_function("number_parsing", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_keyword_heavy(c: &mut Criterion) {
    let base =
        "if else while until for foreach return last next redo package require default continue";
    let input = base.repeat(100);

    c.bench_function("keyword_heavy", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(&input));
            collect_all_tokens(lexer)
        });
    });
}

criterion_group!(
    benches,
    bench_simple_tokens,
    bench_slash_disambiguation,
    bench_string_interpolation,
    bench_large_file,
    bench_whitespace_heavy,
    bench_operator_heavy,
    bench_number_parsing,
    bench_keyword_heavy
);
criterion_main!(benches);
