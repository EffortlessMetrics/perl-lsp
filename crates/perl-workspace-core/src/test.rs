//! Test facts: which framework a test file uses and how many assertions it
//! makes.
//!
//! A first slice of the `TESTS` fact class (PLSP-ADR-0006 PR 4 follow-up).
//! Reads a test file's AST to detect the framework (from `use Test::More` /
//! `use Test2::V0` / …) and count assertion calls, without *running* the test.

use serde::{Deserialize, Serialize};

use perl_parser_core::{Node, NodeKind};

use crate::id::FileId;
use crate::provenance::Confidence;
use crate::range::{SourceRange, Utf8LineIndex};

/// Test frameworks recognized from `use` statements.
const TEST_FRAMEWORKS: &[&str] =
    &["Test2::V0", "Test2::V1", "Test::More", "Test::Simple", "Test::Most", "Test::Spec"];

/// Assertion functions counted across the recognized frameworks.
const ASSERTIONS: &[&str] = &[
    "ok",
    "is",
    "isnt",
    "like",
    "unlike",
    "cmp_ok",
    "is_deeply",
    "isa_ok",
    "can_ok",
    "pass",
    "fail",
    "subtest",
    "done_testing",
    "use_ok",
    "require_ok",
];

/// A test-file fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestFact {
    /// The test file.
    pub file_id: FileId,
    /// The detected framework (`Test::More`, `Test2::V0`, …), if any.
    pub framework: Option<String>,
    /// Number of recognized assertion calls.
    pub assertion_count: u32,
    /// Whether the file declares a plan (`done_testing` or `plan`).
    pub has_plan: bool,
    /// Span of the file.
    pub range: SourceRange,
    /// Confidence in the fact.
    pub confidence: Confidence,
}

/// Extract a [`TestFact`] from a parsed test file, or `None` if it uses no
/// recognized framework and makes no recognized assertions.
#[must_use]
pub fn extract_test_facts(
    ast: &Node,
    file_id: &FileId,
    line_index: &Utf8LineIndex,
) -> Option<TestFact> {
    let mut framework = None;
    let mut assertion_count = 0u32;
    let mut has_plan = false;
    scan(ast, &mut framework, &mut assertion_count, &mut has_plan);

    if framework.is_none() && assertion_count == 0 {
        return None;
    }

    let end = u32::try_from(ast.location.end).unwrap_or(u32::MAX);
    Some(TestFact {
        file_id: file_id.clone(),
        framework,
        assertion_count,
        has_plan,
        range: line_index.source_range(0, end),
        confidence: Confidence::High,
    })
}

fn scan(node: &Node, framework: &mut Option<String>, assertions: &mut u32, has_plan: &mut bool) {
    match &node.kind {
        NodeKind::Use { module, .. } => {
            // `use Test::More 0.98;` puts the version in the module field; take
            // the first whitespace-delimited token.
            let name = module.split_whitespace().next().unwrap_or(module);
            if framework.is_none() && TEST_FRAMEWORKS.contains(&name) {
                *framework = Some(name.to_string());
            }
        }
        NodeKind::FunctionCall { name, .. } => {
            if ASSERTIONS.contains(&name.as_str()) {
                *assertions = assertions.saturating_add(1);
            }
            if name == "done_testing" || name == "plan" {
                *has_plan = true;
            }
        }
        // `done_testing;` / `plan tests => N;` are often written without parens,
        // so they parse as a bareword identifier rather than a function call.
        NodeKind::Identifier { name } if name == "done_testing" || name == "plan" => {
            *has_plan = true;
        }
        _ => {}
    }
    for child in node.children() {
        scan(child, framework, assertions, has_plan);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;
    use perl_parser_core::Parser;

    fn facts_for(src: &str) -> Option<TestFact> {
        let ast = Parser::new(src).parse().unwrap();
        let idx = Utf8LineIndex::new(src);
        extract_test_facts(&ast, &FileId::new("t/x.t", &Digest::of(src)), &idx)
    }

    #[test]
    fn detects_framework_and_counts_assertions() {
        let facts = facts_for("use Test::More;\nok(1);\nis(1, 1);\ndone_testing;\n").unwrap();
        assert_eq!(facts.framework.as_deref(), Some("Test::More"));
        assert!(facts.assertion_count >= 2, "ok + is counted; got {}", facts.assertion_count);
        assert!(facts.has_plan, "parenless done_testing sets has_plan");
    }

    #[test]
    fn detects_test2_and_versioned_use() {
        let facts = facts_for("use Test2::V0;\nok(1, 'yes');\n").unwrap();
        assert_eq!(facts.framework.as_deref(), Some("Test2::V0"));
    }

    #[test]
    fn non_test_file_yields_none() {
        assert!(facts_for("package App;\nsub run { 1 }\n1;\n").is_none());
    }
}
