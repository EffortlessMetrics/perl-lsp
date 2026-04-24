use criterion::{Criterion, criterion_group, criterion_main};
use perl_token::{Token, TokenKind, TokenRef};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: This allocator only forwards operations to `System` while keeping
// allocation counters for benchmark scorecard output.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: Delegates to the system allocator with an unchanged layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Delegates deallocation with the same pointer/layout pair.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: Delegates reallocation to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: Delegates zeroed allocation to the system allocator.
        unsafe { System.alloc_zeroed(layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const SAMPLE_SIZE: usize = 16_384;

fn count_allocations<F>(mut f: F) -> usize
where
    F: FnMut(),
{
    ALLOCATIONS.store(0, Ordering::Relaxed);
    f();
    ALLOCATIONS.load(Ordering::Relaxed)
}

fn benchmark_borrowed_vs_owned(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_borrowed_scorecard");

    group.bench_function("construct_borrowed_token_ref", |b| {
        b.iter(|| {
            let token = TokenRef::new(TokenKind::Identifier, "hot_path_value", 20, 34);
            std::hint::black_box(token);
        })
    });

    group.bench_function("construct_owned_token", |b| {
        b.iter(|| {
            let token = Token::new(TokenKind::Identifier, "hot_path_value", 20, 34);
            std::hint::black_box(token);
        })
    });

    let borrowed_allocs = count_allocations(|| {
        for _ in 0..SAMPLE_SIZE {
            let token = TokenRef::new(TokenKind::Identifier, "hot_path_value", 20, 34);
            std::hint::black_box(token);
        }
    });

    let owned_allocs = count_allocations(|| {
        for _ in 0..SAMPLE_SIZE {
            let token = Token::new(TokenKind::Identifier, "hot_path_value", 20, 34);
            std::hint::black_box(token);
        }
    });

    eprintln!(
        "Token allocation scorecard: borrowed={} owned={} samples={}",
        borrowed_allocs, owned_allocs, SAMPLE_SIZE
    );

    assert!(borrowed_allocs < owned_allocs);
    group.finish();
}

criterion_group!(benches, benchmark_borrowed_vs_owned);
criterion_main!(benches);
