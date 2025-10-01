use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tree_sitter_perl::{EnhancedFullParser, FullPerlParser};

fn bench_enhanced_heredoc(c: &mut Criterion) {
    let heredoc_code = r#"
my $single = <<'EOF';
This is a single quoted heredoc
with multiple lines
EOF

my $double = <<"END";
This has $interpolation
and @arrays
END

my $backtick = <<`CMD`;
echo "shell command"
CMD

my $indented = <<~'INDENT';
    This is indented
    content
INDENT
"#;

    c.bench_function("enhanced_heredoc_parsing", |b| {
        b.iter(|| {
            let mut parser = EnhancedFullParser::new();
            let _ = parser.parse(black_box(heredoc_code));
        })
    });
}

fn bench_data_section(c: &mut Criterion) {
    let data_code = r#"
#!/usr/bin/perl
use strict;
use warnings;

sub process_data {
    my $line = shift;
    print "Processing: $line\n";
}

while (<DATA>) {
    chomp;
    process_data($_);
}

__DATA__
First line of data
Second line of data
Third line of data
Fourth line of data
Fifth line of data
"#;

    c.bench_function("enhanced_data_section", |b| {
        b.iter(|| {
            let mut parser = EnhancedFullParser::new();
            let _ = parser.parse(black_box(data_code));
        })
    });
}

fn bench_pod_extraction(c: &mut Criterion) {
    let pod_code = r#"
package MyModule;

=head1 NAME

MyModule - A sample module

=head1 SYNOPSIS

    use MyModule;
    my $obj = MyModule->new();
    $obj->method();

=head1 DESCRIPTION

This module provides functionality for something.

=head2 Methods

=over 4

=item new()

Constructor

=item method()

Does something

=back

=cut

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub method {
    my $self = shift;
    print "Method called\n";
}

1;
"#;

    c.bench_function("enhanced_pod_extraction", |b| {
        b.iter(|| {
            let mut parser = EnhancedFullParser::new();
            let _ = parser.parse(black_box(pod_code));
        })
    });
}

fn bench_complex_mixed(c: &mut Criterion) {
    let complex_code = r#"
#!/usr/bin/perl
use strict;
use warnings;

=head1 NAME

ComplexScript - A complex test script

=cut

my $config = {
    name => "Test",
    data => <<'DATA',
Configuration data
spanning multiple lines
DATA
    version => "1.0",
};

print <<EOF, <<'LITERAL';
First heredoc with $interpolation
EOF
Second heredoc without $interpolation
LITERAL

sub process {
    my $input = shift;
    return <<~'RESULT';
        Processed: $input
        Status: OK
RESULT
}

=head2 FUNCTIONS

=over 4

=item process()

Processes input

=back

=cut

print process("test");

__DATA__
Data line 1
Data line 2
Data line 3
"#;

    c.bench_function("enhanced_complex_mixed", |b| {
        b.iter(|| {
            let mut parser = EnhancedFullParser::new();
            let _ = parser.parse(black_box(complex_code));
        })
    });
}

fn bench_comparison(c: &mut Criterion) {
    let simple_code = r#"
my $text = <<'EOF';
Simple heredoc content
EOF
print $text;
"#;

    let mut group = c.benchmark_group("parser_comparison");

    group.bench_function("full_parser", |b| {
        b.iter(|| {
            let mut parser = FullPerlParser::new();
            let _ = parser.parse(black_box(simple_code));
        })
    });

    group.bench_function("enhanced_parser", |b| {
        b.iter(|| {
            let mut parser = EnhancedFullParser::new();
            let _ = parser.parse(black_box(simple_code));
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_enhanced_heredoc,
    bench_data_section,
    bench_pod_extraction,
    bench_complex_mixed,
    bench_comparison
);
criterion_main!(benches);
