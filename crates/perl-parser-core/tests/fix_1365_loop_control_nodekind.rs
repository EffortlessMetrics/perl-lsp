//! Issue #1365: `continue` blocks and `redo`/`next`/`last` loop control inside
//! loops (with and without labels) must produce `NodeKind::LoopControl` nodes.
//!
//! The corpus-wide coverage gate proves `LoopControl` appears broadly; these
//! focused tests pin the labeled / `continue`-block interactions specifically.

mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::Node;

fn collect_kinds(node: &Node, out: &mut Vec<&'static str>) {
    out.push(node.kind.kind_name());
    for child in node.children() {
        collect_kinds(child, out);
    }
}

fn kinds(source: &str) -> Vec<&'static str> {
    let ast = parse(source);
    let mut out = Vec::new();
    collect_kinds(&ast, &mut out);
    out
}

#[test]
fn test_redo_next_last_produce_loop_control() {
    let source = r#"
        my $x = 0;
        while ($x < 10) {
            redo if $x == 2;
            next if $x == 3;
            last if $x == 9;
        } continue {
            $x++;
        }
    "#;
    assert_clean_parse(source);
    let ks = kinds(source);
    assert!(ks.contains(&"LoopControl"), "expected LoopControl NodeKind, got: {ks:?}");
}

#[test]
fn test_labeled_loop_control_produces_loop_control() {
    let source = r#"
        OUTER: for my $i (1 .. 3) {
            INNER: for my $j (1 .. 3) {
                next OUTER if $i == $j;
                redo INNER if $j == 1;
                last INNER if $j == 3;
            }
        }
    "#;
    assert_clean_parse(source);
    let ks = kinds(source);
    assert!(ks.contains(&"LoopControl"), "expected labeled LoopControl NodeKind, got: {ks:?}");
}

#[test]
fn test_continue_block_with_redo_parses_clean() {
    // `continue` blocks attach to the loop; ensure the redo inside still yields
    // a LoopControl node and the snippet parses without recovery.
    let source = r#"
        my $n = 0;
        until ($n >= 3) {
            $n++;
            redo if $n == 1;
        } continue {
            my $seen = $n;
        }
    "#;
    assert_clean_parse(source);
    let ks = kinds(source);
    assert!(ks.contains(&"LoopControl"), "expected LoopControl NodeKind, got: {ks:?}");
}
