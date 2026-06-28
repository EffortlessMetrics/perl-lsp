//! HIR control-flow lowering coverage.
//!
//! These tests pin the PIR-v0-aligned control-flow substrate slice: branches,
//! loops, control transfers, and statement modifiers each lower to an explicit
//! HIR shell with a preserved source anchor and no provider behavior change.

use perl_parser_core::Parser;
use perl_parser_core::hir::{
    BranchKeyword, BranchShell, ControlTransfer, ControlTransferKind, HirFile, HirItem, HirKind,
    LoopKind, LoopShell, RecoveryConfidence, StatementModifierKind, StatementModifierShell,
    lower_ast,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn lower_source(source: &str) -> HirFile {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    lower_ast(&output.ast)
}

fn first_branch(file: &HirFile) -> Result<&BranchShell, Box<dyn std::error::Error>> {
    file.items
        .iter()
        .find_map(|item| match &item.kind {
            HirKind::BranchShell(shell) => Some(shell),
            _ => None,
        })
        .ok_or_else(|| "expected a branch shell".into())
}

fn first_statement_modifier(
    file: &HirFile,
) -> Result<&StatementModifierShell, Box<dyn std::error::Error>> {
    file.items
        .iter()
        .find_map(|item| match &item.kind {
            HirKind::StatementModifierShell(shell) => Some(shell),
            _ => None,
        })
        .ok_or_else(|| "expected a statement-modifier shell".into())
}

fn loops(file: &HirFile) -> Vec<&LoopShell> {
    file.items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::LoopShell(shell) => Some(shell),
            _ => None,
        })
        .collect()
}

fn transfers(file: &HirFile) -> Vec<&ControlTransfer> {
    file.items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::ControlTransfer(transfer) => Some(transfer),
            _ => None,
        })
        .collect()
}

fn branch_item(file: &HirFile) -> Result<&HirItem, Box<dyn std::error::Error>> {
    file.items
        .iter()
        .find(|item| matches!(item.kind, HirKind::BranchShell(_)))
        .ok_or_else(|| "expected a branch item".into())
}

#[test]
fn if_elsif_else_lowers_to_branch_shell_with_condition_anchor() -> TestResult {
    let source = "if ($x > 1) { 1 } elsif ($y) { 2 } elsif ($z) { 3 } else { 4 }\n";
    let file = lower_source(source);
    let branch = first_branch(&file)?;

    assert_eq!(branch.keyword, BranchKeyword::If);
    assert_eq!(branch.elsif_count, 2);
    assert!(branch.has_else);

    // Source anchor points at the primary condition expression.
    let condition = &source[branch.condition_range.start..branch.condition_range.end];
    assert!(condition.contains("$x"), "condition anchor was {condition:?}");
    Ok(())
}

#[test]
fn plain_if_without_else_records_no_fallthrough() -> TestResult {
    let file = lower_source("if ($ready) { go() }\n");
    let branch = first_branch(&file)?;
    assert_eq!(branch.keyword, BranchKeyword::If);
    assert_eq!(branch.elsif_count, 0);
    assert!(!branch.has_else);
    Ok(())
}

#[test]
fn unless_block_keeps_its_surface_keyword() -> TestResult {
    let file = lower_source("unless ($done) { wait() }\n");
    let branch = first_branch(&file)?;
    assert_eq!(branch.keyword, BranchKeyword::Unless);
    Ok(())
}

#[test]
fn ternary_lowers_to_branch_shell_with_both_arms() -> TestResult {
    let file = lower_source("my $v = $cond ? 1 : 2;\n");
    let branch = first_branch(&file)?;
    assert_eq!(branch.keyword, BranchKeyword::Ternary);
    assert_eq!(branch.elsif_count, 0);
    assert!(branch.has_else, "ternary always has an else arm");
    Ok(())
}

#[test]
fn while_and_until_lower_to_loop_shells() {
    let file = lower_source("while ($more) { step() } until ($stop) { spin() }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 2);
    assert_eq!(shells[0].kind, LoopKind::While);
    assert!(shells[0].has_condition);
    assert!(!shells[0].declares_iterator);
    assert_eq!(shells[1].kind, LoopKind::Until);
    assert!(shells[1].has_condition);
}

#[test]
fn while_with_continue_block_records_continue() {
    let file = lower_source("while ($more) { step() } continue { tick() }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 1);
    assert!(shells[0].has_continue);
}

#[test]
fn c_style_for_records_optional_condition_and_iterator() {
    let file = lower_source("for (my $i = 0; $i < 10; $i++) { body() }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 1);
    assert_eq!(shells[0].kind, LoopKind::CStyleFor);
    assert!(shells[0].has_condition);
    assert!(shells[0].declares_iterator, "`my $i` declares the iterator");
}

#[test]
fn infinite_c_style_for_has_no_condition() {
    let file = lower_source("for (;;) { body() }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 1);
    assert_eq!(shells[0].kind, LoopKind::CStyleFor);
    assert!(!shells[0].has_condition);
    assert!(!shells[0].declares_iterator);
}

#[test]
fn foreach_with_my_iterator_records_declaration() {
    let file = lower_source("foreach my $item (@list) { use_item($item) }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 1);
    assert_eq!(shells[0].kind, LoopKind::Foreach);
    assert!(!shells[0].has_condition, "foreach iterates a list, not a condition");
    assert!(shells[0].declares_iterator);
}

#[test]
fn foreach_over_topic_variable_does_not_declare_iterator() {
    let file = lower_source("foreach (@list) { use_topic() }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 1);
    assert_eq!(shells[0].kind, LoopKind::Foreach);
    assert!(!shells[0].declares_iterator);
}

#[test]
fn labeled_loop_threads_label_into_loop_shell() {
    let file = lower_source("OUTER: while ($go) { last OUTER }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 1);
    assert_eq!(shells[0].label.as_deref(), Some("OUTER"));
}

#[test]
fn label_does_not_leak_to_sibling_loop() {
    let file = lower_source("OUTER: while ($a) { 1 } while ($b) { 2 }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 2);
    assert_eq!(shells[0].label.as_deref(), Some("OUTER"));
    assert_eq!(shells[1].label, None, "the second loop is unlabeled");
}

#[test]
fn nested_labeled_loops_keep_their_own_labels() {
    let file = lower_source("OUTER: while ($a) { INNER: while ($b) { last OUTER } }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 2);
    assert_eq!(shells[0].label.as_deref(), Some("OUTER"));
    assert_eq!(shells[1].label.as_deref(), Some("INNER"));
}

#[test]
fn label_on_bare_block_does_not_propagate_to_inner_loop() {
    // `OUTER: { while (...) { } }` — label is on the bare block, not the while.
    // The while inside the block should NOT receive the label; `last OUTER`
    // from inside the while exits the outer block, not the while's own iteration.
    let file = lower_source("OUTER: { while ($x) { body() } }\n");
    let shells = loops(&file);
    assert_eq!(shells.len(), 1);
    assert_eq!(
        shells[0].label, None,
        "label on an enclosing bare block must not propagate to the inner loop"
    );
}

#[test]
fn return_with_and_without_value() {
    let with_value = lower_source("sub f { return 42; }\n");
    let bare = lower_source("sub f { return; }\n");

    let with = transfers(&with_value);
    assert_eq!(with.len(), 1);
    assert_eq!(with[0].kind, ControlTransferKind::Return);
    assert!(with[0].has_value);

    let without = transfers(&bare);
    assert_eq!(without.len(), 1);
    assert_eq!(without[0].kind, ControlTransferKind::Return);
    assert!(!without[0].has_value);
}

#[test]
fn loop_control_verbs_lower_with_optional_label() {
    let file = lower_source("while ($x) { next OUTER; last; redo; }\n");
    let xfers = transfers(&file);
    assert_eq!(xfers.len(), 3);
    assert_eq!(xfers[0].kind, ControlTransferKind::Next);
    assert_eq!(xfers[0].label.as_deref(), Some("OUTER"));
    assert_eq!(xfers[1].kind, ControlTransferKind::Last);
    assert_eq!(xfers[1].label, None);
    assert_eq!(xfers[2].kind, ControlTransferKind::Redo);
    for transfer in &xfers {
        assert!(!transfer.has_value, "loop control never carries a value");
    }
}

#[test]
fn goto_label_target_is_preserved() {
    let file = lower_source("goto DONE;\n");
    let xfers = transfers(&file);
    assert_eq!(xfers.len(), 1);
    assert_eq!(xfers[0].kind, ControlTransferKind::Goto);
    assert_eq!(xfers[0].label.as_deref(), Some("DONE"));
}

#[test]
fn statement_modifiers_lower_with_modifier_kind_and_anchor() -> TestResult {
    let cases = [
        ("print 1 if $cond;\n", StatementModifierKind::If),
        ("print 1 unless $cond;\n", StatementModifierKind::Unless),
        ("step() while $more;\n", StatementModifierKind::While),
        ("spin() until $stop;\n", StatementModifierKind::Until),
        ("use_it($_) for @list;\n", StatementModifierKind::Foreach),
        ("use_it($_) foreach @list;\n", StatementModifierKind::Foreach),
    ];

    for (source, expected) in cases {
        let file = lower_source(source);
        let shell = first_statement_modifier(&file)
            .map_err(|_| format!("expected modifier shell for {source:?}"))?;
        assert_eq!(shell.modifier, expected, "source: {source:?}");
        assert!(
            shell.condition_range.end >= shell.condition_range.start,
            "condition anchor should be ordered for {source:?}"
        );
    }
    Ok(())
}

#[test]
fn labeled_postfix_loop_modifier_preserves_label() -> TestResult {
    // `LABEL: STMT while COND` — the label belongs to the postfix loop, so the
    // loop-form modifier shell preserves it for labeled control-transfer edges.
    for (source, expected) in [
        ("OUTER: print 1 while $cond;\n", StatementModifierKind::While),
        ("LOOP: step() until $done;\n", StatementModifierKind::Until),
        ("EACH: use_it($_) for @list;\n", StatementModifierKind::Foreach),
    ] {
        let file = lower_source(source);
        let shell = first_statement_modifier(&file)?;
        assert_eq!(shell.modifier, expected, "source: {source:?}");
        assert!(shell.label.is_some(), "expected a preserved label for {source:?}");
    }
    Ok(())
}

#[test]
fn branch_form_modifier_does_not_capture_label() -> TestResult {
    // `if`/`unless` modifiers are not loop targets, so they never carry a label,
    // even when an enclosing label is syntactically present.
    let file = lower_source("DONE: print 1 if $cond;\n");
    let shell = first_statement_modifier(&file)?;
    assert_eq!(shell.modifier, StatementModifierKind::If);
    assert_eq!(shell.label, None, "branch-form modifiers must not capture labels");
    Ok(())
}

#[test]
fn control_flow_items_preserve_source_anchor_and_parse_confidence() -> TestResult {
    let file = lower_source("if ($x) { return 1 } else { return 2 }\n");
    for item in &file.items {
        assert!(item.range.end >= item.range.start, "HIR item range should be ordered: {item:?}");
        assert_eq!(item.range, item.anchor.range, "anchor range mirrors item range");
        assert_eq!(item.recovery_confidence, RecoveryConfidence::Parsed);
    }

    // The branch shell anchors back to the parser `If` node.
    assert_eq!(branch_item(&file)?.anchor.node_kind, "If");
    Ok(())
}

#[test]
fn control_flow_lowering_does_not_emit_dynamic_boundaries() {
    // Static control flow is fully modeled, so it must not fall back to a
    // dynamic boundary (those are reserved for genuinely unanalyzable forms).
    let file = lower_source(
        "foreach my $i (@list) {\n\
         next if $i < 0;\n\
         last if $i > 100;\n\
         return $i if $i == 42;\n\
         }\n",
    );
    let boundaries =
        file.items.iter().filter(|item| matches!(item.kind, HirKind::DynamicBoundary(_))).count();
    assert_eq!(boundaries, 0, "static control flow must not be a dynamic boundary");
}
