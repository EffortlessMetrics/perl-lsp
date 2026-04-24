//! Tests for borrowed token views (`TokenRef`).

use perl_token::{Token, TokenKind, TokenRef};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: This allocator forwards all allocation operations to `System` while
// recording allocation call counts for scorecard-style assertions in tests.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: Delegates to the system allocator with the same `layout`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Delegates deallocation for the pointer/layout pair from `alloc`.
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

fn allocation_delta<F>(mut f: F) -> usize
where
    F: FnMut(),
{
    ALLOCATIONS.store(0, Ordering::Relaxed);
    f();
    ALLOCATIONS.load(Ordering::Relaxed)
}

#[test]
fn token_ref_construction_fields_and_helpers() {
    let tok = TokenRef::new(TokenKind::Identifier, "example", 5, 12);
    assert_eq!(tok.kind, TokenKind::Identifier);
    assert_eq!(tok.text, "example");
    assert_eq!(tok.len(), 7);
    assert!(!tok.is_empty());
    assert_eq!(tok.span(), (5, 12));
    assert_eq!(tok.display_name(), "identifier");
}

#[test]
fn token_ref_to_owned_token_is_explicit() {
    let borrowed = TokenRef::new(TokenKind::My, "my", 0, 2);
    let owned = borrowed.to_owned_token();

    assert_eq!(owned.kind, TokenKind::My);
    assert_eq!(&*owned.text, "my");
    assert_eq!(owned.span(), (0, 2));
}

#[test]
fn token_ref_from_into_token_matches_explicit_conversion() {
    let borrowed = TokenRef::new(TokenKind::Sub, "sub", 10, 13);
    let from_impl: Token = borrowed.into();

    assert_eq!(from_impl.kind, TokenKind::Sub);
    assert_eq!(&*from_impl.text, "sub");
    assert_eq!(from_impl.span(), (10, 13));
}

#[test]
fn token_as_ref_token_reuses_existing_text() {
    let owned = Token::new(TokenKind::Identifier, "alpha", 1, 6);
    let borrowed = owned.as_ref_token();

    assert_eq!(borrowed.kind, TokenKind::Identifier);
    assert_eq!(borrowed.text, "alpha");
    assert_eq!(borrowed.span(), (1, 6));
    assert_eq!(borrowed.display_name(), "identifier");
}

#[test]
fn borrowed_token_construction_avoids_arc_allocation() {
    let borrowed_allocations = allocation_delta(|| {
        for _ in 0..4096 {
            let tok = TokenRef::new(TokenKind::Identifier, "static_text", 3, 14);
            std::hint::black_box(tok);
        }
    });

    let owned_allocations = allocation_delta(|| {
        for _ in 0..4096 {
            let tok = Token::new(TokenKind::Identifier, "static_text", 3, 14);
            std::hint::black_box(tok);
        }
    });

    assert_eq!(borrowed_allocations, 0);
    assert!(owned_allocations > borrowed_allocations);
}
