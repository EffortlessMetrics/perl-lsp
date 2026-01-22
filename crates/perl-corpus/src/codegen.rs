//! Randomized Perl code generation utilities.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::r#gen;

const DEFAULT_PREAMBLE: &str = "use strict;\nuse warnings;\n\n";

/// Statement categories for randomized code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatementKind {
    /// Minimal valid statements (assignments, conditionals, subs).
    Basic,
    /// Package/subroutine declarations and method calls.
    Declarations,
    /// Object-oriented constructs (bless, inheritance, overload).
    ObjectOriented,
    /// qw(...) and related list constructs.
    Qw,
    /// Quote-like operators (q/qq/qx/qr).
    QuoteLike,
    /// Heredoc syntax in common contexts.
    Heredoc,
    /// Whitespace and comment stress cases.
    Whitespace,
    /// Loop control flow statements.
    ControlFlow,
    /// Format statements and sections.
    Format,
    /// Glob expressions and patterns.
    Glob,
    /// Tie/untie statements.
    Tie,
    /// I/O and filehandle statements.
    Io,
    /// Filetest operators and stacked checks.
    Filetest,
    /// Built-in function calls (pack/unpack, split/join, etc).
    Builtins,
    /// Map/grep/sort list operators.
    ListOps,
    /// Operator-focused expressions.
    Expressions,
    /// Regex match/substitution/transliteration.
    Regex,
    /// Parser ambiguity and stress cases.
    Ambiguity,
    /// Sigil-heavy variable and dereference patterns.
    Sigils,
    /// Compile-time phase blocks (BEGIN/CHECK/UNITCHECK/INIT/END).
    Phasers,
}

const STATEMENT_KINDS_ALL: [StatementKind; 20] = [
    StatementKind::Basic,
    StatementKind::Declarations,
    StatementKind::ObjectOriented,
    StatementKind::Qw,
    StatementKind::QuoteLike,
    StatementKind::Heredoc,
    StatementKind::Whitespace,
    StatementKind::ControlFlow,
    StatementKind::Format,
    StatementKind::Glob,
    StatementKind::Tie,
    StatementKind::Io,
    StatementKind::Filetest,
    StatementKind::Builtins,
    StatementKind::ListOps,
    StatementKind::Expressions,
    StatementKind::Regex,
    StatementKind::Ambiguity,
    StatementKind::Sigils,
    StatementKind::Phasers,
];

impl StatementKind {
    /// Return all available statement kinds.
    pub fn all() -> &'static [StatementKind] {
        &STATEMENT_KINDS_ALL
    }
}

/// Options for randomized Perl code generation.
#[derive(Debug, Clone)]
pub struct CodegenOptions {
    /// Number of statements to generate.
    pub statements: usize,
    /// Seed for deterministic output.
    pub seed: u64,
    /// Optional preamble prepended to output (e.g., `use strict;`).
    pub preamble: Option<String>,
    /// Ensure each selected statement kind appears at least once when possible.
    pub ensure_coverage: bool,
    /// Statement kinds to include in generation.
    pub kinds: Vec<StatementKind>,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            statements: 20,
            seed: default_seed(),
            preamble: Some(DEFAULT_PREAMBLE.to_string()),
            ensure_coverage: false,
            kinds: StatementKind::all().to_vec(),
        }
    }
}

/// Generate random Perl code with a default statement count.
pub fn generate_perl_code() -> String {
    generate_perl_code_with_options(CodegenOptions::default())
}

/// Generate random Perl code with a specific statement count.
pub fn generate_perl_code_with_statements(statements: usize) -> String {
    let options = CodegenOptions { statements, ..Default::default() };
    generate_perl_code_with_options(options)
}

/// Generate random Perl code with explicit statement count and seed.
pub fn generate_perl_code_with_seed(statements: usize, seed: u64) -> String {
    let options = CodegenOptions { statements, seed, ..Default::default() };
    generate_perl_code_with_options(options)
}

/// Generate random Perl code with explicit options.
pub fn generate_perl_code_with_options(options: CodegenOptions) -> String {
    let mut rng = StdRng::seed_from_u64(options.seed);
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::from_seed(RngAlgorithm::ChaCha, &proptest_seed(options.seed)),
    );
    let strategies = build_strategies_for(&options.kinds);

    let mut output = String::new();
    if let Some(preamble) = options.preamble.as_deref() {
        output.push_str(preamble);
    }

    if strategies.is_empty() || options.statements == 0 {
        return output;
    }

    let indices = build_strategy_indices(
        strategies.len(),
        options.statements,
        options.ensure_coverage,
        &mut rng,
    );

    for (i, idx) in indices.into_iter().enumerate() {
        let fallback = format!("my $var{} = {};", i, i);
        let mut snippet = sample_strategy(&strategies[idx], &mut runner, &fallback);

        if !snippet.ends_with('\n') {
            snippet.push('\n');
        }

        output.push_str(&snippet);
        output.push('\n');
    }

    output
}

fn default_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs()
}

fn proptest_seed(seed: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, chunk) in bytes.chunks_mut(8).enumerate() {
        let mixed = seed.wrapping_add((i as u64).wrapping_mul(0x9E3779B97F4A7C15));
        chunk.copy_from_slice(&mixed.to_le_bytes());
    }
    bytes
}

fn build_strategies_for(kinds: &[StatementKind]) -> Vec<BoxedStrategy<String>> {
    let mut strategies = Vec::new();

    for kind in kinds {
        match kind {
            StatementKind::Basic => strategies.push(basic_statement().boxed()),
            StatementKind::Declarations => {
                strategies.push(r#gen::declarations::declaration_in_context().boxed());
            }
            StatementKind::ObjectOriented => {
                strategies.push(r#gen::object_oriented::object_oriented_in_context().boxed());
            }
            StatementKind::Qw => {
                let qw = r#gen::qw::qw_in_context();
                let constants = r#gen::qw::use_constant_qw().prop_map(|(src, _)| src);
                strategies.push(prop_oneof![qw, constants].boxed());
            }
            StatementKind::QuoteLike => {
                let quote = r#gen::quote_like::quote_like_single()
                    .prop_map(|expr| format!("my $text = {};\n", expr));
                strategies.push(quote.boxed());
            }
            StatementKind::Heredoc => strategies.push(r#gen::heredoc::heredoc_in_context().boxed()),
            StatementKind::Whitespace => {
                let whitespace = r#gen::whitespace::whitespace_stress_test();
                let commented = r#gen::whitespace::commented_code();
                strategies.push(prop_oneof![whitespace, commented].boxed());
            }
            StatementKind::ControlFlow => {
                strategies.push(r#gen::control_flow::loop_with_control().boxed());
            }
            StatementKind::Format => {
                strategies.push(r#gen::format_statements::format_statement().boxed());
            }
            StatementKind::Glob => strategies.push(r#gen::glob::glob_in_context().boxed()),
            StatementKind::Tie => strategies.push(r#gen::tie::tie_in_context().boxed()),
            StatementKind::Io => strategies.push(r#gen::io::io_in_context().boxed()),
            StatementKind::Filetest => {
                strategies.push(r#gen::filetest::filetest_in_context().boxed());
            }
            StatementKind::Builtins => {
                strategies.push(r#gen::builtins::builtin_in_context().boxed());
            }
            StatementKind::ListOps => {
                strategies.push(r#gen::list_ops::list_op_in_context().boxed());
            }
            StatementKind::Expressions => {
                strategies.push(r#gen::expressions::expression_in_context().boxed());
            }
            StatementKind::Regex => {
                strategies.push(r#gen::regex::regex_in_context().boxed());
            }
            StatementKind::Ambiguity => {
                strategies.push(r#gen::ambiguity::ambiguity_in_context().boxed());
            }
            StatementKind::Sigils => {
                strategies.push(r#gen::sigils::sigil_in_context().boxed());
            }
            StatementKind::Phasers => {
                strategies.push(r#gen::phasers::phaser_block().boxed());
            }
        }
    }

    strategies
}

fn build_strategy_indices(
    strategy_len: usize,
    statements: usize,
    ensure_coverage: bool,
    rng: &mut StdRng,
) -> Vec<usize> {
    if strategy_len == 0 || statements == 0 {
        return Vec::new();
    }

    let mut indices = Vec::with_capacity(statements);

    if ensure_coverage {
        let mut all_indices: Vec<usize> = (0..strategy_len).collect();
        all_indices.shuffle(rng);

        if statements <= strategy_len {
            indices.extend(all_indices.into_iter().take(statements));
            return indices;
        }

        indices.extend(all_indices);
    }

    while indices.len() < statements {
        indices.push(rng.random_range(0..strategy_len));
    }

    indices
}

fn sample_strategy(
    strategy: &BoxedStrategy<String>,
    runner: &mut TestRunner,
    fallback: &str,
) -> String {
    match strategy.new_tree(runner) {
        Ok(tree) => tree.current(),
        Err(_) => fallback.to_string(),
    }
}

fn basic_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("my $x = 1;".to_string()),
        Just("my @items = (1, 2, 3);".to_string()),
        Just("my %map = (a => 1, b => 2);".to_string()),
        Just("sub add { return $_[0] + $_[1]; }".to_string()),
        Just("if ($x) { print $x; }".to_string()),
        Just("my $msg = \"hello\"; print $msg;".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_code_is_stable_for_seed() {
        let first = generate_perl_code_with_seed(5, 42);
        let second = generate_perl_code_with_seed(5, 42);
        assert_eq!(first, second);
    }

    #[test]
    fn codegen_respects_empty_kinds() {
        let options = CodegenOptions {
            statements: 5,
            seed: 123,
            preamble: None,
            ensure_coverage: false,
            kinds: Vec::new(),
        };
        let code = generate_perl_code_with_options(options);
        assert!(code.is_empty());
    }
}
