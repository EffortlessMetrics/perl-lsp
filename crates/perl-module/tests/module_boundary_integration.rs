use perl_module::boundary::{
    contains_standalone_module_token, find_standalone_module_token_ranges,
};
use perl_module::import::parse_module_import_head;
use perl_module::name::module_variant_pairs;

#[test]
fn direct_import_head_offsets_align_with_boundary_range() {
    let line = "use Demo::Worker;";
    let parsed = parse_module_import_head(line);
    assert!(parsed.is_some());

    let parsed = parsed.unwrap_or_else(|| unreachable!());
    let ranges = find_standalone_module_token_ranges(line, parsed.token).collect::<Vec<_>>();

    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].start, parsed.token_start);
    assert_eq!(ranges[0].end, parsed.token_end);
}

#[test]
fn module_variant_pairs_remain_scannable_for_canonical_and_legacy_forms() {
    let variants = module_variant_pairs("My::Module", "My::Renamed");

    assert!(!variants.is_empty());
    let canonical_old = &variants[0].0;
    let canonical_line = format!("use {canonical_old};");
    assert!(contains_standalone_module_token(&canonical_line, canonical_old));

    if variants.len() == 2 {
        let legacy_old = &variants[1].0;
        let legacy_line = format!("use {legacy_old};");
        assert!(contains_standalone_module_token(&legacy_line, legacy_old));
    }
}
