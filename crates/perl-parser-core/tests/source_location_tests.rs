use perl_parser_core::SourceLocation;

#[test]
fn empty_span_at_position() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::empty(5);
    assert_eq!(loc.start, 5);
    assert_eq!(loc.end, 5);
    assert_eq!(loc.len(), 0);
    assert!(loc.is_empty());
    Ok(())
}

#[test]
fn whole_span_covers_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello world";
    let loc = SourceLocation::whole(source);
    assert_eq!(loc.start, 0);
    assert_eq!(loc.end, 11);
    assert_eq!(loc.len(), 11);
    assert!(!loc.is_empty());
    Ok(())
}

#[test]
fn whole_span_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::whole("");
    assert_eq!(loc.start, 0);
    assert_eq!(loc.end, 0);
    assert!(loc.is_empty());
    Ok(())
}

#[test]
fn contains_offset() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::new(5, 10);
    assert!(!loc.contains(4));
    assert!(loc.contains(5));
    assert!(loc.contains(7));
    assert!(loc.contains(9));
    // end is exclusive
    assert!(!loc.contains(10));
    Ok(())
}

#[test]
fn contains_span_inner_outer() -> Result<(), Box<dyn std::error::Error>> {
    let outer = SourceLocation::new(0, 20);
    let inner = SourceLocation::new(5, 15);
    let partial = SourceLocation::new(15, 25);

    assert!(outer.contains_span(inner));
    assert!(!inner.contains_span(outer));
    assert!(!outer.contains_span(partial));
    // A span contains itself
    assert!(outer.contains_span(outer));
    Ok(())
}

#[test]
fn overlaps_various() -> Result<(), Box<dyn std::error::Error>> {
    let a = SourceLocation::new(0, 10);
    let b = SourceLocation::new(5, 15);
    let c = SourceLocation::new(10, 20);

    assert!(a.overlaps(b));
    assert!(b.overlaps(a));
    // Adjacent spans do NOT overlap (half-open)
    assert!(!a.overlaps(c));
    // Empty spans at same position don't overlap
    let e1 = SourceLocation::empty(5);
    let e2 = SourceLocation::empty(5);
    assert!(!e1.overlaps(e2));
    Ok(())
}

#[test]
fn intersection_overlapping() -> Result<(), Box<dyn std::error::Error>> {
    let a = SourceLocation::new(0, 10);
    let b = SourceLocation::new(5, 15);
    if let Some(inter) = a.intersection(b) {
        assert_eq!(inter.start, 5);
        assert_eq!(inter.end, 10);
    } else {
        return Err("expected intersection".into());
    }
    Ok(())
}

#[test]
fn intersection_disjoint() -> Result<(), Box<dyn std::error::Error>> {
    let a = SourceLocation::new(0, 5);
    let b = SourceLocation::new(10, 15);
    assert!(a.intersection(b).is_none());
    Ok(())
}

#[test]
fn intersection_adjacent() -> Result<(), Box<dyn std::error::Error>> {
    let a = SourceLocation::new(0, 5);
    let b = SourceLocation::new(5, 10);
    assert!(a.intersection(b).is_none());
    Ok(())
}

#[test]
fn union_covers_both() -> Result<(), Box<dyn std::error::Error>> {
    let a = SourceLocation::new(3, 7);
    let b = SourceLocation::new(10, 20);
    let u = a.union(b);
    assert_eq!(u.start, 3);
    assert_eq!(u.end, 20);
    Ok(())
}

#[test]
fn try_slice_in_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello world";
    let loc = SourceLocation::new(6, 11);
    if let Some(s) = loc.try_slice(source) {
        assert_eq!(s, "world");
    } else {
        return Err("expected Some slice".into());
    }
    Ok(())
}

#[test]
fn try_slice_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let source = "short";
    let loc = SourceLocation::new(0, 100);
    assert!(loc.try_slice(source).is_none());
    Ok(())
}

#[test]
fn to_range_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::new(3, 9);
    let range = loc.to_range();
    assert_eq!(range, 3..9);
    Ok(())
}

#[test]
fn display_format() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::new(42, 100);
    assert_eq!(format!("{}", loc), "42..100");
    Ok(())
}

#[test]
fn from_range_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let loc: SourceLocation = (5..10).into();
    assert_eq!(loc.start, 5);
    assert_eq!(loc.end, 10);
    Ok(())
}

#[test]
fn from_tuple_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let loc: SourceLocation = (3, 7).into();
    assert_eq!(loc.start, 3);
    assert_eq!(loc.end, 7);
    Ok(())
}

#[test]
fn into_tuple_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::new(3, 7);
    let (s, e): (usize, usize) = loc.into();
    assert_eq!(s, 3);
    assert_eq!(e, 7);
    Ok(())
}

#[test]
fn into_range_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::new(1, 5);
    let range: std::ops::Range<usize> = loc.into();
    assert_eq!(range, 1..5);
    Ok(())
}

#[test]
fn default_is_zero_span() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation::default();
    assert_eq!(loc.start, 0);
    assert_eq!(loc.end, 0);
    assert!(loc.is_empty());
    Ok(())
}

#[test]
fn slice_extracts_text() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello world";
    let loc = SourceLocation::new(0, 5);
    assert_eq!(loc.slice(source), "hello");
    Ok(())
}
