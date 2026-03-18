//! Shared execute-command identifiers for `perl-lsp` crates.
//!
//! This microcrate has one narrow responsibility: define the canonical LSP
//! `workspace/executeCommand` identifiers used across capability advertisement,
//! request validation, and command classification.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// Execute `prove`/test-suite style workflows for a file.
pub const RUN_TESTS: &str = "perl.runTests";
/// Execute a Perl file directly.
pub const RUN_FILE: &str = "perl.runFile";
/// Execute a specific test subroutine.
pub const RUN_TEST_SUB: &str = "perl.runTestSub";
/// Run Perl::Critic or the built-in critic fallback.
pub const RUN_CRITIC: &str = "perl.runCritic";
/// Run a single test target.
pub const RUN_TEST: &str = "perl.runTest";
/// Run a test file target.
pub const RUN_TEST_FILE: &str = "perl.runTestFile";
/// Launch debugging for a file.
pub const DEBUG_FILE: &str = "perl.debugFile";
/// Extract a variable refactor.
pub const EXTRACT_VARIABLE: &str = "perl.extractVariable";
/// Extract a subroutine refactor.
pub const EXTRACT_SUBROUTINE: &str = "perl.extractSubroutine";
/// Organize and optimize imports.
pub const OPTIMIZE_IMPORTS: &str = "perl.optimizeImports";
/// Format the current document.
pub const FORMAT_DOCUMENT: &str = "perl.formatDocument";

const ADVERTISED_EXECUTE_COMMANDS: &[&str] =
    &[RUN_TESTS, RUN_FILE, RUN_TEST_SUB, RUN_CRITIC, RUN_TEST, RUN_TEST_FILE, DEBUG_FILE];

const VALIDATED_EXECUTE_COMMANDS: &[&str] =
    &[RUN_CRITIC, FORMAT_DOCUMENT, EXTRACT_VARIABLE, EXTRACT_SUBROUTINE, OPTIMIZE_IMPORTS];

const REFACTOR_COMMANDS: &[&str] = &[EXTRACT_VARIABLE, EXTRACT_SUBROUTINE, OPTIMIZE_IMPORTS];

/// Borrow the canonical supported command identifiers advertised in server capabilities.
#[must_use]
pub const fn advertised_execute_commands() -> &'static [&'static str] {
    ADVERTISED_EXECUTE_COMMANDS
}

/// Clone the advertised command list into owned strings.
#[must_use]
pub fn advertised_execute_command_strings() -> Vec<String> {
    advertised_execute_commands().iter().map(|command| (*command).to_string()).collect()
}

/// Borrow the canonical command identifiers accepted by input validation.
#[must_use]
pub const fn validated_execute_commands() -> &'static [&'static str] {
    VALIDATED_EXECUTE_COMMANDS
}

/// Check whether a command is in the input-validation allowlist.
#[must_use]
pub fn is_validated_execute_command(command: &str) -> bool {
    validated_execute_commands().contains(&command)
}

/// Check whether a command belongs to the refactor/import-management subset.
#[must_use]
pub fn is_refactor_command(command: &str) -> bool {
    REFACTOR_COMMANDS.contains(&command)
}

#[cfg(test)]
mod tests {
    use super::{
        DEBUG_FILE, EXTRACT_SUBROUTINE, EXTRACT_VARIABLE, FORMAT_DOCUMENT, OPTIMIZE_IMPORTS,
        RUN_CRITIC, RUN_FILE, RUN_TEST, RUN_TEST_FILE, RUN_TEST_SUB, RUN_TESTS,
        advertised_execute_command_strings, advertised_execute_commands, is_refactor_command,
        is_validated_execute_command, validated_execute_commands,
    };
    use std::collections::BTreeSet;

    #[test]
    fn advertised_commands_are_unique_and_stable() {
        let commands = advertised_execute_commands();
        let unique = commands.iter().copied().collect::<BTreeSet<_>>();

        assert_eq!(commands.len(), unique.len());
        assert_eq!(
            commands,
            &[RUN_TESTS, RUN_FILE, RUN_TEST_SUB, RUN_CRITIC, RUN_TEST, RUN_TEST_FILE, DEBUG_FILE,]
        );
    }

    #[test]
    fn validation_allowlist_matches_security_expectations() {
        assert_eq!(
            validated_execute_commands(),
            &[RUN_CRITIC, FORMAT_DOCUMENT, EXTRACT_VARIABLE, EXTRACT_SUBROUTINE, OPTIMIZE_IMPORTS,]
        );

        for command in validated_execute_commands() {
            assert!(is_validated_execute_command(command));
        }

        assert!(!is_validated_execute_command(RUN_TESTS));
        assert!(!is_validated_execute_command("perl.unknownCommand"));
    }

    #[test]
    fn owned_string_list_matches_borrowed_advertised_list() {
        let borrowed = advertised_execute_commands();
        let owned = advertised_execute_command_strings();

        assert_eq!(owned.len(), borrowed.len());
        assert!(owned.iter().zip(borrowed.iter()).all(|(left, right)| left == right));
        assert!(is_refactor_command(EXTRACT_VARIABLE));
        assert!(is_refactor_command(EXTRACT_SUBROUTINE));
        assert!(is_refactor_command(OPTIMIZE_IMPORTS));
        assert!(!is_refactor_command(RUN_TESTS));
    }
}
