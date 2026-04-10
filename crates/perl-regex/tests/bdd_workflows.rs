//! BDD-style workflow coverage for `perl-regex`.
//!
//! These scenarios describe end-user outcomes in Given/When/Then form.

use perl_regex::{RegexAnalyzer, RegexError, RegexValidator};

struct BddScenario {
    name: &'static str,
}

impl BddScenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, step: &str) {
        eprintln!("[{}] Given {step}", self.name);
    }

    fn when(&self, step: &str) {
        eprintln!("[{}] When {step}", self.name);
    }

    fn then(&self, step: &str) {
        eprintln!("[{}] Then {step}", self.name);
    }
}

#[test]
fn bdd_security_review_flags_runtime_code_paths() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Security review flags runtime code execution regexes");
    let validator = RegexValidator::new();
    let input = "(?{ system('id') })";

    scenario.given("an untrusted regex pattern from user configuration");
    scenario.when("checking for embedded Perl code execution constructs");
    let has_code_exec = validator.detects_code_execution(input);

    scenario.then("the analyzer reports that the pattern can execute code");
    assert!(has_code_exec);
    Ok(())
}

#[test]
fn bdd_editor_validation_reports_offsets_for_budget_violations()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Editor validation reports precise offsets for safety limits");
    let validator = RegexValidator::new();
    let pattern = (0..51).map(|_| r"\p{Letter}").collect::<String>();
    let start_pos = 12;

    scenario.given("a regex with excessive Unicode property classes");
    scenario.when("running regex validation from a source slice offset");
    let err = validator.validate(&pattern, start_pos).expect_err("expected complexity error");

    scenario.then("the result is a syntax error with a source-relative byte offset");
    match err {
        RegexError::Syntax { message, offset } => {
            assert!(message.contains("Too many Unicode properties"));
            assert_eq!(offset, start_pos + 500);
        }
    }
    Ok(())
}

#[test]
fn bdd_backtracking_risk_is_advisory_not_fatal() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Backtracking heuristics remain advisory for valid patterns");
    let validator = RegexValidator::new();
    let pattern = "(a+)+";

    scenario.given("a pattern that can exhibit catastrophic backtracking");
    scenario.when("calling validate for hard-fail syntax and complexity checks");
    validator.validate(pattern, 0)?;

    scenario.then("validation succeeds while advisory nested-quantifier detection still warns");
    assert!(validator.detect_nested_quantifiers(pattern));
    Ok(())
}

#[test]
fn bdd_named_capture_workflow_surfaces_hover_context() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Named capture authoring provides informative hover details");
    let pattern = "(?<year>\\d{4})-(?<month>\\d{2})";

    scenario.given("a regex with semantic named captures and modifiers");
    scenario.when("extracting captures and preparing hover content");
    let captures = RegexAnalyzer::extract_named_captures(pattern);
    let hover = RegexAnalyzer::hover_text_for_regex(pattern, "im");

    scenario.then("capture indices and modifier guidance are visible to tooling");
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].name, "year");
    assert_eq!(captures[0].index, 1);
    assert_eq!(captures[1].name, "month");
    assert_eq!(captures[1].index, 2);
    assert!(hover.contains("${year}"));
    assert!(hover.contains("case-insensitive"));
    assert!(hover.contains("multiline"));
    Ok(())
}

#[test]
fn bdd_branch_reset_guardrail_blocks_branch_explosion() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = BddScenario::new("Branch reset guardrails block extreme alternation fan-out");
    let validator = RegexValidator::new();
    let alternates = (0..51).map(|_| "alt").collect::<Vec<_>>().join("|");
    let pattern = format!("(?|{alternates})");

    scenario.given("a branch-reset group with more alternatives than allowed");
    scenario.when("validating branch-reset complexity constraints");
    let err = validator.validate(&pattern, 4).expect_err("expected branch limit error");

    scenario.then("validation fails with an explicit branch-limit message");
    match err {
        RegexError::Syntax { message, offset } => {
            assert!(message.contains("Too many branches in branch reset group"));
            assert!(offset >= 4);
        }
    }
    Ok(())
}
