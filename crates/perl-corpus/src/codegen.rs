//! Randomized Perl code generation utilities.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::r#gen;

/// Generate random Perl code with a default statement count.
pub fn generate_perl_code() -> String {
    generate_perl_code_with_statements(20)
}

/// Generate random Perl code with a specific statement count.
pub fn generate_perl_code_with_statements(statements: usize) -> String {
    generate_perl_code_with_seed(statements, default_seed())
}

/// Generate random Perl code with explicit statement count and seed.
pub fn generate_perl_code_with_seed(statements: usize, seed: u64) -> String {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::from_seed(RngAlgorithm::ChaCha, &seed.to_le_bytes()),
    );
    let strategies = build_strategies();

    let mut output = String::new();
    output.push_str("use strict;\nuse warnings;\n\n");

    if strategies.is_empty() {
        return output;
    }

    for i in 0..statements {
        let idx = rng.random_range(0..strategies.len());
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

fn build_strategies() -> Vec<BoxedStrategy<String>> {
    vec![
        basic_statement().boxed(),
        r#gen::declarations::declaration_in_context().boxed(),
        r#gen::qw::qw_in_context().boxed(),
        r#gen::quote_like::quote_like_single().boxed(),
        r#gen::heredoc::heredoc_in_context().boxed(),
        r#gen::whitespace::whitespace_stress_test().boxed(),
        r#gen::control_flow::loop_with_control().boxed(),
        r#gen::format_statements::format_statement().boxed(),
        r#gen::glob::glob_in_context().boxed(),
        r#gen::tie::tie_in_context().boxed(),
    ]
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
}
