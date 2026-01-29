use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use perl_lsp_tooling::performance::AstCache;
use perl_parser_core::{Node, NodeKind, SourceLocation};
use std::hint::black_box;
use std::sync::Arc;

fn create_test_ast(size: usize) -> Arc<Node> {
    Arc::new(Node::new(
        NodeKind::Program {
            statements: (0..size)
                .map(|_| {
                    Node::new(
                        NodeKind::Program { statements: vec![] },
                        SourceLocation { start: 0, end: 0 },
                    )
                })
                .collect(),
        },
        SourceLocation { start: 0, end: 100 },
    ))
}

fn bench_cache_put(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_put");

    for size in [1, 10, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let cache = AstCache::new(100, 300);
            let ast = create_test_ast(size);
            let content = "x".repeat(size * 100);

            b.iter(|| {
                cache.put(
                    black_box(format!("file_{}.pl", size)),
                    black_box(&content),
                    black_box(ast.clone()),
                );
            });
        });
    }
    group.finish();
}

fn bench_cache_get_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_get_hit");

    for size in [1, 10, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let cache = AstCache::new(100, 300);
            let ast = create_test_ast(size);
            let content = "x".repeat(size * 100);
            let uri = format!("file_{}.pl", size);

            // Pre-populate cache
            cache.put(uri.clone(), &content, ast.clone());

            b.iter(|| cache.get(black_box(&uri), black_box(&content)));
        });
    }
    group.finish();
}

fn bench_cache_get_miss(c: &mut Criterion) {
    let cache = AstCache::new(100, 300);

    c.bench_function("cache_get_miss", |b| {
        b.iter(|| cache.get(black_box("nonexistent.pl"), black_box("content")));
    });
}

fn bench_cache_eviction(c: &mut Criterion) {
    c.bench_function("cache_eviction", |b| {
        let cache = AstCache::new(10, 300);
        let ast = create_test_ast(1);

        b.iter(|| {
            // Fill cache beyond capacity to trigger eviction
            for i in 0..15 {
                cache.put(
                    black_box(format!("file_{}.pl", i)),
                    black_box(&format!("content_{}", i)),
                    black_box(ast.clone()),
                );
            }
        });
    });
}

fn bench_cache_concurrent_access(c: &mut Criterion) {
    c.bench_function("cache_concurrent_access", |b| {
        let cache = Arc::new(AstCache::new(100, 300));
        let ast = create_test_ast(1);

        b.iter(|| {
            let mut handles = vec![];

            for i in 0..4 {
                let cache_clone = Arc::clone(&cache);
                let ast_clone = ast.clone();

                let handle = std::thread::spawn(move || {
                    for j in 0..10 {
                        let uri = format!("file_{}_{}.pl", i, j);
                        let content = format!("content_{}_{}", i, j);
                        cache_clone.put(uri.clone(), &content, ast_clone.clone());
                        let _ = cache_clone.get(&uri, &content);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.join();
            }
        });
    });
}

fn bench_cache_cleanup(c: &mut Criterion) {
    c.bench_function("cache_cleanup", |b| {
        let cache = AstCache::new(100, 1); // Short TTL
        let ast = create_test_ast(1);

        // Pre-populate cache
        for i in 0..50 {
            cache.put(format!("file_{}.pl", i), &format!("content_{}", i), ast.clone());
        }

        // Wait for some entries to expire
        std::thread::sleep(std::time::Duration::from_millis(1100));

        b.iter(|| cache.cleanup());
    });
}

criterion_group!(
    benches,
    bench_cache_put,
    bench_cache_get_hit,
    bench_cache_get_miss,
    bench_cache_eviction,
    bench_cache_concurrent_access,
    bench_cache_cleanup,
);
criterion_main!(benches);
