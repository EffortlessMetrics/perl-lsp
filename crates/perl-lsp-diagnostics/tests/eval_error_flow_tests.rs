use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pl407_messages(source: &str) -> Vec<String> {
    diagnostics_for(source)
        .into_iter()
        .filter(|d| d.code.as_deref() == Some("PL407"))
        .map(|d| d.message)
        .collect()
}

#[test]
fn reading_error_without_prior_eval_or_try_warns() {
    let source = "use v5.40;\nmy $err = $@;\n";
    let messages = pl407_messages(source);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("without a preceding `eval` or `try`")),
        "expected PL407 for bare $@ read, got: {messages:?}"
    );
}

#[test]
fn immediate_check_after_eval_is_allowed() {
    let source = "use v5.40;\neval { risky() };\nif ($@) { warn $@; }\n";
    let messages = pl407_messages(source);
    assert!(
        messages.is_empty(),
        "immediate check after eval should not warn, got: {messages:?}"
    );
}

#[test]
fn intervening_statement_before_check_warns() {
    let source = "use v5.40;\neval { risky() };\nmy $marker = 1;\nif ($@) { warn $@; }\n";
    let messages = pl407_messages(source);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("after an intervening statement")),
        "expected PL407 for stale $@ read, got: {messages:?}"
    );
}

#[test]
fn immediate_check_after_try_is_allowed_for_eval_error() {
    let source =
        "use v5.40;\ntry { risky() } catch { warn $_ };\nif ($EVAL_ERROR) { warn $EVAL_ERROR; }\n";
    let messages = pl407_messages(source);
    assert!(
        messages.is_empty(),
        "immediate check after try should not warn, got: {messages:?}"
    );
}

#[test]
fn localizing_error_variable_is_not_treated_as_a_read() {
    let source = "use v5.40;\nlocal $@ = undef;\nlocal $EVAL_ERROR = undef;\n";
    let messages = pl407_messages(source);
    assert!(
        messages.is_empty(),
        "localizing $@ / $EVAL_ERROR should not warn, got: {messages:?}"
    );
}

#[test]
fn assignment_targets_do_not_count_as_reads() {
    let source = "use v5.40;\n($@, $EVAL_ERROR) = (1, 2);\n";
    let messages = pl407_messages(source);
    assert!(
        messages.is_empty(),
        "assignment targets for $@ / $EVAL_ERROR should not warn, got: {messages:?}"
    );
}

#[test]
fn statement_modifier_reads_are_visible() {
    let source = "use v5.40;\nwarn $@ if $@;\n";
    let messages = pl407_messages(source);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("without a preceding `eval` or `try`")),
        "expected PL407 for a statement-modifier read, got: {messages:?}"
    );
}

#[test]
fn stale_read_inside_handler_body_is_reported() {
    let source = "use v5.40;\neval { risky() };\nif ($@) {\n    my $marker = 1;\n    warn $@;\n}\n";
    let messages = pl407_messages(source);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("after an intervening statement")),
        "expected PL407 for a stale read inside the handler body, got: {messages:?}"
    );
}
