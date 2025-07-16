//! Scanner performance benchmarks
//!
//! This module contains benchmarks to compare the performance of
//! the Rust-native scanner against the C scanner implementation.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

// Test different scanner implementations
#[cfg(feature = "rust-scanner")]
use tree_sitter_perl::scanner::{PerlScanner, RustScanner, ScannerConfig};

#[cfg(feature = "c-scanner")]
use tree_sitter_perl::scanner::{PerlScanner, CScanner, ScannerConfig};

fn bench_rust_scanner_direct(c: &mut Criterion) {
    #[cfg(feature = "rust-scanner")]
    {
        let test_cases = vec![
            "my $var = 42;",
            "print 'Hello, World!';",
            "sub foo { return 1; }",
            "if ($x) { $y = 1; }",
            "for my $i (1..10) { print $i; }",
        ];

        c.bench_function("rust_scanner_direct", |b| {
            b.iter(|| {
                let mut scanner = RustScanner::new();
                for code in &test_cases {
                    let bytes = code.as_bytes();
                    let mut pos = 0;
                    while pos < bytes.len() {
                        match scanner.scan(&bytes[pos..]) {
                            Ok(Some(_token)) => {
                                // Simulate advancing position
                                pos += 1;
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    black_box(scanner);
                }
            });
        });
    }
}

fn bench_c_scanner_direct(c: &mut Criterion) {
    #[cfg(feature = "c-scanner")]
    {
        let test_cases = vec![
            "my $var = 42;",
            "print 'Hello, World!';",
            "sub foo { return 1; }",
            "if ($x) { $y = 1; }",
            "for my $i (1..10) { print $i; }",
        ];

        c.bench_function("c_scanner_direct", |b| {
            b.iter(|| {
                let mut scanner = CScanner::new();
                for code in &test_cases {
                    let bytes = code.as_bytes();
                    let mut pos = 0;
                    while pos < bytes.len() {
                        match scanner.scan(&bytes[pos..]) {
                            Ok(Some(_token)) => {
                                // Simulate advancing position
                                pos += 1;
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    black_box(scanner);
                }
            });
        });
    }
}

// Legacy benchmarks that use the parse function (for comparison)
fn bench_parse_function(c: &mut Criterion) {
    use tree_sitter_perl::parse;
    
    let test_cases = vec![
        "my $var = 42;",
        "print 'Hello, World!';",
        "sub foo { return 1; }",
        "if ($x) { $y = 1; }",
        "for my $i (1..10) { print $i; }",
    ];

    c.bench_function("parse_function", |b| {
        b.iter(|| {
            for code in &test_cases {
                black_box(parse(code).unwrap());
            }
        });
    });
}

fn bench_large_file(c: &mut Criterion) {
    use tree_sitter_perl::parse;
    let large_code = generate_large_perl_file(1000);

    c.bench_function("large_file_parsing", |b| {
        b.iter(|| {
            black_box(parse(&large_code).unwrap());
        });
    });
}

fn bench_unicode_heavy(c: &mut Criterion) {
    use tree_sitter_perl::parse;
    let unicode_code = generate_unicode_perl_file();

    c.bench_function("unicode_parsing", |b| {
        b.iter(|| {
            black_box(parse(&unicode_code).unwrap());
        });
    });
}

fn bench_string_heavy(c: &mut Criterion) {
    use tree_sitter_perl::parse;
    let string_code = generate_string_heavy_perl_file();

    c.bench_function("string_heavy_parsing", |b| {
        b.iter(|| {
            black_box(parse(&string_code).unwrap());
        });
    });
}

fn bench_regex_heavy(c: &mut Criterion) {
    use tree_sitter_perl::parse;
    let regex_code = generate_regex_heavy_perl_file();

    c.bench_function("regex_heavy_parsing", |b| {
        b.iter(|| {
            black_box(parse(&regex_code).unwrap());
        });
    });
}

fn generate_large_perl_file(size: usize) -> String {
    let mut code = String::new();

    for i in 0..size {
        code.push_str(&format!("my $var{} = {};\n", i, i));
        code.push_str(&format!("print \"Variable {} = $var{}\";\n", i, i));
    }

    code
}

fn generate_unicode_perl_file() -> String {
    r#"
my $変数 = "値";
my $über = "cool";
my $naïve = "simple";
my $café = "coffee";
my $résumé = "summary";
my $日本語 = "こんにちは";
my $emoji = "🚀 rocket";

sub 関数 {
    my ($param) = @_;
    return "関数です: $param";
}

print $変数;
print $über;
print $naïve;
print $café;
print $résumé;
print $日本語;
print $emoji;
print 関数("test");
"#
    .to_string()
}

fn generate_string_heavy_perl_file() -> String {
    r#"
my $str1 = "This is a very long string with lots of content";
my $str2 = 'Another long string with single quotes';
my $str3 = qq{Interpolated string with $variables};
my $str4 = q{Non-interpolated string with lots of content};

my $heredoc = <<"EOF";
This is a here document
with multiple lines
of content
EOF

my $template = qq{
<html>
<head><title>$title</title></head>
<body>
<h1>$heading</h1>
<p>$content</p>
</body>
</html>
};

print $str1;
print $str2;
print $str3;
print $str4;
print $heredoc;
print $template;
"#
    .to_string()
}

fn generate_regex_heavy_perl_file() -> String {
    r#"
my $pattern1 = qr/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
my $pattern2 = qr{\d{3}-\d{3}-\d{4}};
my $pattern3 = qr/^(https?:\/\/)?([\da-z\.-]+)\.([a-z\.]{2,6})([\/\w \.-]*)*\/?$/;

my $text = "test@example.com";
if ($text =~ $pattern1) {
    print "Valid email";
}

my $phone = "123-456-7890";
if ($phone =~ $pattern2) {
    print "Valid phone";
}

my $url = "https://example.com/path";
if ($url =~ $pattern3) {
    print "Valid URL";
}

my $complex_regex = qr{
    ^
    (?<protocol>https?://)?
    (?<domain>[a-zA-Z0-9.-]+)
    (?<port>:\d+)?
    (?<path>/[^\s]*)?
    $
}ix;

my $result = $text =~ s/old/new/g;
my $transliteration = $text =~ tr/a-z/A-Z/;
"#
    .to_string()
}

criterion_group!(
    benches,
    bench_parse_function,
    bench_large_file,
    bench_unicode_heavy,
    bench_string_heavy,
    bench_regex_heavy,
    bench_rust_scanner_direct,
    bench_c_scanner_direct,
);
criterion_main!(benches);
