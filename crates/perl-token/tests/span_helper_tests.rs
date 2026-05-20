use perl_token::TokenSpan;
use std::error::Error;

#[test]
fn contains_uses_half_open_range_semantics() -> Result<(), Box<dyn Error>> {
    let span = TokenSpan::new(3, 7);

    assert!(!span.contains(2));
    assert!(span.contains(3));
    assert!(span.contains(6));
    assert!(!span.contains(7));

    Ok(())
}

#[test]
fn touches_includes_boundaries_for_cursor_resolution() -> Result<(), Box<dyn Error>> {
    let span = TokenSpan::new(3, 7);

    assert!(!span.touches(2));
    assert!(span.touches(3));
    assert!(span.touches(7));
    assert!(!span.touches(8));

    let empty = TokenSpan::new(5, 5);
    assert!(!empty.contains(5));
    assert!(empty.touches(5));

    Ok(())
}

#[test]
fn overlaps_distinguishes_overlap_from_adjacency() -> Result<(), Box<dyn Error>> {
    let span = TokenSpan::new(10, 20);

    assert!(span.overlaps(TokenSpan::new(15, 25)));
    assert!(span.overlaps(TokenSpan::new(5, 11)));
    assert!(!span.overlaps(TokenSpan::new(20, 30)));
    assert!(!span.overlaps(TokenSpan::new(0, 10)));
    assert!(!span.overlaps(TokenSpan::new(12, 12)));

    Ok(())
}

#[test]
fn cover_returns_smallest_span_covering_both_inputs() -> Result<(), Box<dyn Error>> {
    let left = TokenSpan::new(8, 12);
    let right = TokenSpan::new(2, 20);

    assert_eq!(left.cover(right), TokenSpan::new(2, 20));
    assert_eq!(right.cover(left), TokenSpan::new(2, 20));

    Ok(())
}
