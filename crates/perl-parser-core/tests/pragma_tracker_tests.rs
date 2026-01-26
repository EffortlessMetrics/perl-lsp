use perl_parser_core::Parser;
use perl_pragma::PragmaTracker;
use perl_tdd_support::must;

#[test]
fn test_use_strict_enables_all() {
    let source = "use strict;\nmy $x = FOO;";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let pragma_map = PragmaTracker::build(&ast);

    // After "use strict", all strict modes should be enabled
    let state = PragmaTracker::state_for_offset(&pragma_map, 12); // After "use strict;"
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
}

#[test]
fn test_use_strict_specific_category() {
    let source = "use strict 'subs';\nmy $x = FOO;";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let pragma_map = PragmaTracker::build(&ast);

    // After "use strict 'subs'", only strict subs should be enabled
    let state = PragmaTracker::state_for_offset(&pragma_map, 19); // After "use strict 'subs';"
    assert!(!state.strict_vars);
    assert!(state.strict_subs);
    assert!(!state.strict_refs);
}

#[test]
fn test_no_strict_disables() {
    let source = "use strict;\nno strict 'subs';\nmy $x = FOO;";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let pragma_map = PragmaTracker::build(&ast);

    // After "no strict 'subs'", strict subs should be disabled but vars/refs still enabled
    let state = PragmaTracker::state_for_offset(&pragma_map, 30); // After "no strict 'subs';"
    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
}
