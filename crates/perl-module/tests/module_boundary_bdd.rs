use perl_module::boundary::{
    contains_standalone_module_token, find_standalone_module_token_ranges,
};

#[test]
fn given_direct_import_line_when_module_token_is_standalone_then_match_is_found() {
    let line = "use My::Module;";
    let ranges = find_standalone_module_token_ranges(line, "My::Module").collect::<Vec<_>>();

    assert_eq!(ranges.len(), 1);
    assert_eq!(&line[ranges[0].start..ranges[0].end], "My::Module");
    assert!(contains_standalone_module_token(line, "My::Module"));
}

#[test]
fn given_partial_module_suffix_when_scanning_then_match_is_not_found() {
    let line = "use My::ModuleX;";

    assert!(
        find_standalone_module_token_ranges(line, "My::Module")
            .collect::<Vec<_>>()
            .is_empty()
    );
    assert!(!contains_standalone_module_token(line, "My::Module"));
}

#[test]
fn given_legacy_separator_with_trailing_segment_when_scanning_then_boundary_rejects_partial_match()
{
    let line = "use My'Module'Child;";

    assert!(
        find_standalone_module_token_ranges(line, "My'Module")
            .collect::<Vec<_>>()
            .is_empty()
    );
    assert!(!contains_standalone_module_token(line, "My'Module"));
}

#[test]
fn given_empty_inputs_when_scanning_then_no_match_is_returned() {
    assert!(!contains_standalone_module_token("", "My::Module"));
    assert!(!contains_standalone_module_token("use My::Module;", ""));
}
