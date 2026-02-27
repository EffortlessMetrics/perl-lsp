use perl_parser::{Parser, pragma_tracker::PragmaTracker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"use strict;
my $x = $h{key};
print FOO;"#;

    println!("Code:\n{}", code);

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    println!("AST: {}", ast.to_sexp());

    // Build pragma map
    let pragma_map = PragmaTracker::build(&ast);

    println!("Pragma map has {} entries:", pragma_map.len());
    for (range, state) in &pragma_map {
        println!(
            "  Range {:?}: strict_vars={}, strict_refs={}, strict_subs={}, warnings={}",
            range, state.strict_vars, state.strict_refs, state.strict_subs, state.warnings
        );
    }

    // Test pragma state at specific offsets
    let test_offsets = vec![0, 10, 20, 30, 40];
    for offset in test_offsets {
        let state = PragmaTracker::state_for_offset(&pragma_map, offset);
        println!("  Offset {}: strict_subs={}", offset, state.strict_subs);
    }
    Ok(())
}
