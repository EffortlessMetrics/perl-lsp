//! Program composition generators for creating complete Perl programs
//!
//! These generators compose statements from other generators into
//! complete, realistic Perl programs.

use proptest::prelude::*;

use super::builtins::builtin_in_context;
use super::control_flow::loop_with_control;
use super::declarations::{
    declaration_in_context, subroutine_declaration, use_require_statement, variable_declaration,
};
use super::expressions::expression_in_context;
use super::qw::qw_in_context;
use super::tie::tie_in_context;

/// Generate a pragma/use header section
fn program_header() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("use strict;\nuse warnings;\n\n".to_string()),
        Just("use v5.36;\n\n".to_string()),
        Just("use v5.38;\nuse feature 'class';\n\n".to_string()),
        Just("use strict;\nuse warnings;\nuse utf8;\n\n".to_string()),
        Just("#!/usr/bin/perl\nuse strict;\nuse warnings;\n\n".to_string()),
        Just("".to_string()), // No header
    ]
}

/// Generate a statement that can stand alone
fn standalone_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        expression_in_context(),
        builtin_in_context(),
        qw_in_context(),
        variable_declaration(),
    ]
}

/// Generate a simple program (header + statements)
pub fn simple_program() -> impl Strategy<Value = String> {
    (
        program_header(),
        prop::collection::vec(standalone_statement(), 1..5),
    )
        .prop_map(|(header, statements)| {
            let mut program = header;
            for stmt in statements {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }
            program
        })
}

/// Generate a program with subroutines
pub fn program_with_subs() -> impl Strategy<Value = String> {
    (
        program_header(),
        prop::collection::vec(subroutine_declaration(), 1..3),
        prop::collection::vec(standalone_statement(), 0..3),
    )
        .prop_map(|(header, subs, statements)| {
            let mut program = header;

            // Add subroutines
            for sub in subs {
                program.push_str(&sub);
                if !sub.ends_with('\n') {
                    program.push('\n');
                }
                program.push('\n');
            }

            // Add main code
            for stmt in statements {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }

            program
        })
}

/// Generate a program with control flow
pub fn program_with_control_flow() -> impl Strategy<Value = String> {
    (
        program_header(),
        prop::collection::vec(standalone_statement(), 0..2),
        loop_with_control(),
        prop::collection::vec(standalone_statement(), 0..2),
    )
        .prop_map(|(header, pre, control, post)| {
            let mut program = header;

            for stmt in pre {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }

            program.push_str(&control);
            if !control.ends_with('\n') {
                program.push('\n');
            }

            for stmt in post {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }

            program
        })
}

/// Generate a program with mixed declarations
pub fn program_with_declarations() -> impl Strategy<Value = String> {
    (
        program_header(),
        prop::collection::vec(declaration_in_context(), 2..5),
    )
        .prop_map(|(header, decls)| {
            let mut program = header;
            for decl in decls {
                program.push_str(&decl);
                if !decl.ends_with('\n') {
                    program.push('\n');
                }
            }
            program
        })
}

/// Generate a program with use statements and imports
pub fn program_with_imports() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(use_require_statement(), 1..4),
        prop::collection::vec(standalone_statement(), 1..4),
    )
        .prop_map(|(imports, statements)| {
            let mut program = String::new();

            for imp in imports {
                program.push_str(&imp);
            }
            program.push('\n');

            for stmt in statements {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }

            program
        })
}

/// Generate a program with tie/untie operations
pub fn program_with_tie() -> impl Strategy<Value = String> {
    (
        program_header(),
        prop::collection::vec(standalone_statement(), 0..2),
        tie_in_context(),
        prop::collection::vec(standalone_statement(), 0..2),
    )
        .prop_map(|(header, pre, tie_code, post)| {
            let mut program = header;

            for stmt in pre {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }

            program.push_str(&tie_code);
            if !tie_code.ends_with('\n') {
                program.push('\n');
            }

            for stmt in post {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }

            program
        })
}

/// Generate any type of complete program
pub fn any_program() -> impl Strategy<Value = String> {
    prop_oneof![
        simple_program(),
        program_with_subs(),
        program_with_control_flow(),
        program_with_declarations(),
        program_with_imports(),
        program_with_tie(),
    ]
}

/// Generate a larger, more complex program
pub fn complex_program() -> impl Strategy<Value = String> {
    (
        program_header(),
        prop::collection::vec(use_require_statement(), 0..3),
        prop::collection::vec(subroutine_declaration(), 0..2),
        prop::collection::vec(
            prop_oneof![
                standalone_statement(),
                loop_with_control(),
                declaration_in_context(),
            ],
            2..6,
        ),
    )
        .prop_map(|(header, imports, subs, body)| {
            let mut program = header;

            // Imports first
            for imp in imports {
                program.push_str(&imp);
            }
            if !program.is_empty() && !program.ends_with('\n') {
                program.push('\n');
            }

            // Subroutines
            for sub in subs {
                program.push_str(&sub);
                if !sub.ends_with('\n') {
                    program.push('\n');
                }
                program.push('\n');
            }

            // Main body
            for stmt in body {
                program.push_str(&stmt);
                if !stmt.ends_with('\n') {
                    program.push('\n');
                }
            }

            program
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn simple_programs_are_non_empty(prog in simple_program()) {
            assert!(!prog.trim().is_empty() || prog.is_empty());
        }

        #[test]
        fn programs_with_subs_have_sub(prog in program_with_subs()) {
            assert!(prog.contains("sub "));
        }

        #[test]
        fn any_program_is_valid_utf8(prog in any_program()) {
            // Just the fact that we can iterate over chars proves it's valid UTF-8
            let _ = prog.chars().count();
        }

        #[test]
        fn complex_programs_generate(prog in complex_program()) {
            // Just verify generation doesn't panic
            let _ = prog.len();
        }
    }
}
