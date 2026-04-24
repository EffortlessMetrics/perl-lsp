use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use perl_token::{Token, TokenKind, TokenRef};

struct CountingAlloc;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegating directly to system allocator with same layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegating directly to system allocator with same ptr/layout pair.
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn reset_alloc_counters() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
}

fn allocation_scorecard_owned(iterations: usize) -> usize {
    reset_alloc_counters();
    for _ in 0..iterations {
        black_box(Token::new(TokenKind::Identifier, "classification_target", 10, 31));
    }
    ALLOCATIONS.load(Ordering::Relaxed)
}

fn allocation_scorecard_borrowed(iterations: usize) -> usize {
    reset_alloc_counters();
    for _ in 0..iterations {
        black_box(TokenRef::new(TokenKind::Identifier, "classification_target", 10, 31));
    }
    ALLOCATIONS.load(Ordering::Relaxed)
}

fn token_construction_scorecard(c: &mut Criterion) {
    let iterations = 100_000;
    let owned_allocs = allocation_scorecard_owned(iterations);
    let borrowed_allocs = allocation_scorecard_borrowed(iterations);

    assert!(owned_allocs > borrowed_allocs);

    let mut group = c.benchmark_group("token_construction_scorecard");
    group.throughput(Throughput::Elements(iterations as u64));

    group.bench_with_input(
        BenchmarkId::new("owned_token_new", format!("allocs={owned_allocs}")),
        &iterations,
        |b, &input| {
            b.iter(|| {
                for _ in 0..input {
                    black_box(Token::new(TokenKind::Identifier, "classification_target", 10, 31));
                }
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("borrowed_token_new", format!("allocs={borrowed_allocs}")),
        &iterations,
        |b, &input| {
            b.iter(|| {
                for _ in 0..input {
                    black_box(TokenRef::new(
                        TokenKind::Identifier,
                        "classification_target",
                        10,
                        31,
                    ));
                }
            });
        },
    );

    group.finish();
}

criterion_group!(benches, token_construction_scorecard);
criterion_main!(benches);
