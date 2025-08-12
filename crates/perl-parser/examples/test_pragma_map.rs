use perl_parser::{Parser, pragma_tracker::PragmaTracker};

fn main() {
    let source = r#"
use strict;

print FOO;  # Bareword not allowed
"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("AST parsed successfully");

            // Build pragma map
            let pragma_map = PragmaTracker::build(&ast);

            println!("\nPragma map has {} entries:", pragma_map.len());
            for (range, state) in &pragma_map {
                println!(
                    "  Range {:?}: strict_subs={}, strict_vars={}, strict_refs={}",
                    range, state.strict_subs, state.strict_vars, state.strict_refs
                );
            }

            // Check state at different offsets
            println!("\nPragma state at various offsets:");
            for offset in &[0, 5, 10, 15, 20] {
                let state = PragmaTracker::state_for_offset(&pragma_map, *offset);
                println!("  Offset {}: strict_subs={}", offset, state.strict_subs);
            }
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
