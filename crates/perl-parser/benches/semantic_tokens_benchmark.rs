use criterion::{Criterion, criterion_group, criterion_main};
use perl_parser::{
    Parser,
    semantic_tokens_provider::{SemanticTokensProvider, encode_semantic_tokens},
};
#[allow(unused_imports)] // Used in benchmark functions but clippy may not detect it
use std::hint::black_box;

fn benchmark_semantic_tokens_small(c: &mut Criterion) {
    let code = r#"
package MyPackage;
use strict;
use warnings;

my $var = 42;
sub test_function {
    my ($param) = @_;
    print $param;
    return $param * 2;
}

test_function($var);
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = SemanticTokensProvider::new(code.to_string());

    c.bench_function("semantic_tokens_extract_small", |b| {
        b.iter(|| {
            let tokens = provider.extract(std::hint::black_box(&ast));
            std::hint::black_box(tokens)
        });
    });

    let tokens = provider.extract(&ast);
    c.bench_function("semantic_tokens_encode_small", |b| {
        b.iter(|| {
            let encoded = encode_semantic_tokens(std::hint::black_box(&tokens));
            std::hint::black_box(encoded)
        });
    });

    c.bench_function("semantic_tokens_full_pipeline_small", |b| {
        b.iter(|| {
            let tokens = provider.extract(std::hint::black_box(&ast));
            let encoded = encode_semantic_tokens(std::hint::black_box(&tokens));
            std::hint::black_box(encoded)
        });
    });
}

fn benchmark_semantic_tokens_medium(c: &mut Criterion) {
    let code = r#"
package WebServer;
use strict;
use warnings;
use JSON;
use DBI;

our $VERSION = '1.0.0';

sub new {
    my ($class, %args) = @_;
    my $self = {
        host => $args{host} || 'localhost',
        port => $args{port} || 8080,
        dbh  => undef,
    };
    return bless $self, $class;
}

sub connect_db {
    my ($self, $dsn, $user, $pass) = @_;
    $self->{dbh} = DBI->connect($dsn, $user, $pass, {
        RaiseError => 1,
        AutoCommit => 1,
    });
    return $self->{dbh};
}

sub start_server {
    my ($self) = @_;
    my $host = $self->{host};
    my $port = $self->{port};
    
    print "Starting server on $host:$port\n";
    
    while (my $request = $self->get_request()) {
        my $response = $self->handle_request($request);
        $self->send_response($response);
    }
}

sub handle_request {
    my ($self, $request) = @_;
    my $path = $request->{path};
    my $method = $request->{method};
    
    if ($method eq 'GET' && $path eq '/api/users') {
        return $self->get_users();
    } elsif ($method eq 'POST' && $path eq '/api/users') {
        return $self->create_user($request->{body});
    } else {
        return { status => 404, body => 'Not Found' };
    }
}

sub get_users {
    my ($self) = @_;
    my $sth = $self->{dbh}->prepare('SELECT * FROM users');
    $sth->execute();
    my $users = $sth->fetchall_arrayref({});
    return { status => 200, body => encode_json($users) };
}

sub create_user {
    my ($self, $data) = @_;
    my $user_data = decode_json($data);
    my $sth = $self->{dbh}->prepare(
        'INSERT INTO users (name, email) VALUES (?, ?)'
    );
    $sth->execute($user_data->{name}, $user_data->{email});
    return { status => 201, body => 'User created' };
}

1;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = SemanticTokensProvider::new(code.to_string());

    c.bench_function("semantic_tokens_extract_medium", |b| {
        b.iter(|| {
            let tokens = provider.extract(std::hint::black_box(&ast));
            std::hint::black_box(tokens)
        });
    });

    let tokens = provider.extract(&ast);
    c.bench_function("semantic_tokens_encode_medium", |b| {
        b.iter(|| {
            let encoded = encode_semantic_tokens(std::hint::black_box(&tokens));
            std::hint::black_box(encoded)
        });
    });

    c.bench_function("semantic_tokens_full_pipeline_medium", |b| {
        b.iter(|| {
            let tokens = provider.extract(std::hint::black_box(&ast));
            let encoded = encode_semantic_tokens(std::hint::black_box(&tokens));
            std::hint::black_box(encoded)
        });
    });
}

fn benchmark_semantic_tokens_concurrent(c: &mut Criterion) {
    let code = r#"
package TestConcurrency;
use strict;
use warnings;

my $shared_var = 'test';
sub func1 { return $shared_var . '1'; }
sub func2 { return $shared_var . '2'; }
sub func3 { return $shared_var . '3'; }
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = SemanticTokensProvider::new(code.to_string());

    c.bench_function("semantic_tokens_concurrent_access", |b| {
        b.iter(|| {
            // Simulate concurrent access by calling extract multiple times
            let tokens1 = provider.extract(std::hint::black_box(&ast));
            let tokens2 = provider.extract(std::hint::black_box(&ast));
            let tokens3 = provider.extract(std::hint::black_box(&ast));
            std::hint::black_box((tokens1, tokens2, tokens3))
        });
    });

    c.bench_function("semantic_tokens_consistency_check", |b| {
        b.iter(|| {
            // Check that multiple calls produce identical results
            let tokens1 = provider.extract(std::hint::black_box(&ast));
            let tokens2 = provider.extract(std::hint::black_box(&ast));
            assert_eq!(tokens1.len(), tokens2.len());
            for (t1, t2) in tokens1.iter().zip(&tokens2) {
                assert_eq!(t1.line, t2.line);
                assert_eq!(t1.start_char, t2.start_char);
                assert_eq!(t1.token_type, t2.token_type);
            }
            std::hint::black_box((tokens1, tokens2))
        });
    });
}

fn benchmark_semantic_tokens_vs_old_implementation(c: &mut Criterion) {
    let code = r#"
package Comparison;
my $var = 42;
sub func { return $var; }
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();

    // Test new thread-safe implementation
    let provider = SemanticTokensProvider::new(code.to_string());

    c.bench_function("semantic_tokens_new_implementation", |b| {
        b.iter(|| {
            let tokens = provider.extract(std::hint::black_box(&ast));
            let encoded = encode_semantic_tokens(std::hint::black_box(&tokens));
            std::hint::black_box(encoded)
        });
    });

    // Compare with direct old-style collection (simulate the old approach)
    use perl_parser::semantic_tokens::collect_semantic_tokens;

    c.bench_function("semantic_tokens_old_style_collection", |b| {
        b.iter(|| {
            let tokens = collect_semantic_tokens(std::hint::black_box(&ast), code, &|offset| {
                // Simple byte-to-position conversion for benchmarking
                let mut line = 0u32;
                let mut char = 0u32;
                for (i, ch) in code.char_indices() {
                    if i >= offset {
                        break;
                    }
                    if ch == '\n' {
                        line += 1;
                        char = 0;
                    } else {
                        char += 1;
                    }
                }
                (line, char)
            });
            std::hint::black_box(tokens)
        });
    });
}

criterion_group!(
    benches,
    benchmark_semantic_tokens_small,
    benchmark_semantic_tokens_medium,
    benchmark_semantic_tokens_concurrent,
    benchmark_semantic_tokens_vs_old_implementation
);
criterion_main!(benches);
