//! Regression gate for PIR-A references directional safety + declaration-anchor
//! parity (#2634 shadow, #2640 parity — a precondition for #2635 promotion).
//!
//! Runs the PIR-A compiler path beside the legacy `find_references_single_file`
//! over realistic multi-scope Perl. After declaration-anchor parity (#2640) the
//! compiler result is a strict SUBSET of legacy for every sigil-unique lexical
//! name: it never over-reports (`extra_in_compiler` is always empty). The
//! `missing_from_compiler` entries are legacy's scope-blind false positives the
//! scope-exact compiler correctly excludes — the migration win this gate keeps
//! exercised.
//!
//! `LexicalExtractorReceipt` is `#[non_exhaustive]`, so receipts are obtained by
//! driving the real pipeline rather than hand-construction. No diagnostic prints
//! (workspace clippy bans `print_stderr`/`print_stdout`); the failure path embeds
//! the full disagreement report in the assertion message.

use perl_lsp_rs_core::providers::navigation::find_references_single_file;
use perl_lsp_rs_core::providers::navigation::references_pir_shadow::shadow_references_with_pir;
use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};
use std::collections::{BTreeMap, BTreeSet};

/// Realistic multi-scope snippets: same lexical name reused across blocks / subs /
/// closures / loop-locals — exactly where scope-blind matching goes wrong.
const CORPUS: &[(&str, &str)] = &[
    ("block_shadow", "my $x = 1;\n{\n    my $x = 2;\n    print $x;\n}\nprint $x;\n"),
    ("two_subs_same_name", "sub a { my $v = 1; print $v; }\nsub b { my $v = 2; print $v; }\n"),
    ("closure_capture", "my $c = 0;\nmy $f = sub { $c = $c + 1; return $c; };\nprint $c;\n"),
    ("loop_local", "for my $i (1 .. 3) { print $i; }\nmy $i = 9;\nprint $i;\n"),
    ("if_block_scope", "my $r = 0;\nif (1) { my $r = 5; print $r; }\nprint $r;\n"),
    ("multi_read", "my $n = 3;\nprint $n;\nprint $n;\nmy $m = $n + 1;\nprint $m;\n"),
    (
        "reassign_then_sub",
        "my $s = 'a';\n$s = 'b';\nprint $s;\nsub t { my $s = 'c'; return $s; }\n",
    ),
];

#[test]
fn burn_in_compiler_is_subset_of_legacy() {
    let mut names_checked = 0usize;
    let mut scope_narrowing = 0usize; // names where compiler correctly drops out-of-scope sites
    let mut total_extra = 0usize; // compiler-over-legacy ranges — MUST stay 0
    let mut report = String::new();

    for (label, src) in CORPUS {
        let mut parser = Parser::new(src);
        let output = parser.parse_with_recovery();
        let hir = lower_ast(&output.ast);
        let receipt = extract_lexical_facts(&hir);

        for (body_idx, body) in receipt.bodies.iter().enumerate() {
            // Only assess sigil-unique names; `$x`/`@x`/`%x` collisions are a known
            // PR2 simplification (the shadow filters by bare name), tracked for PR3.
            let mut sigils: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
            for f in &body.facts {
                sigils.entry(f.name.name.as_str()).or_default().insert(f.name.sigil.as_str());
            }

            for (name, sigs) in &sigils {
                if sigs.len() != 1 {
                    continue;
                }

                // Probe facts for an offset the legacy walker resolves to a Variable.
                let mut legacy: Vec<(usize, usize)> = Vec::new();
                let mut anchored = false;
                for f in body.facts.iter().filter(|f| &f.name.name == name) {
                    if let Some(r) = f.source_anchor.range.as_ref() {
                        if let Some(found) = find_references_single_file(&output.ast, r.start) {
                            if !found.is_empty() {
                                legacy = found;
                                anchored = true;
                                break;
                            }
                        }
                    }
                }
                if !anchored {
                    continue;
                }

                let cmp = shadow_references_with_pir(&receipt, &legacy, name, body_idx);
                names_checked += 1;
                total_extra += cmp.extra_in_compiler.len();
                if !cmp.missing_from_compiler.is_empty() && cmp.extra_in_compiler.is_empty() {
                    scope_narrowing += 1;
                }
                if !cmp.extra_in_compiler.is_empty() || !cmp.missing_from_compiler.is_empty() {
                    report.push_str(&format!(
                        "  [{label}] body{body_idx} ${name}: compiler={} legacy={} missing={:?} extra={:?}\n",
                        cmp.compiler_candidate_count,
                        cmp.legacy_candidate_count,
                        cmp.missing_from_compiler,
                        cmp.extra_in_compiler,
                    ));
                }
            }
        }
    }

    // After #2640 declaration-anchor parity, the compiler is a strict subset of
    // legacy for every sigil-unique lexical name. `missing` is the scope-blind
    // false positives the compiler correctly excludes (the migration win); `extra`
    // must be empty everywhere (directional safety + range parity).
    assert!(names_checked > 0, "burn-in must check at least one lexical name");
    assert!(scope_narrowing > 0, "burn-in must exercise the scope-narrowing win");
    assert_eq!(
        total_extra, 0,
        "directional-safety/parity violation: compiler over-reported vs legacy:\n{report}"
    );
}
