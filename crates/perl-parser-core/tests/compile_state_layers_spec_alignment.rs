//! Spec-alignment proof for PLSP-SPEC-0030 (compile state layers contract).
//!
//! These tests pin the spec's load-bearing invariants to the real HIR
//! substrate so the contract cannot silently drift from the code:
//!
//! - C1: a single `lower_ast` pass produces every compile-state layer (L0-L6)
//!   from one file, with no provider, no Perl execution, and no cross-file
//!   resolution required.
//! - C3: the compile-effect log is deterministic, source-ordered with
//!   contiguous ordinals from 0, and version-stamped with
//!   `COMPILE_EFFECT_MODEL_VERSION`.
//!
//! See `docs/specs/PLSP-SPEC-0030-compile-state-layers.md`.

use perl_parser_core::Parser;
use perl_parser_core::hir::{COMPILE_EFFECT_MODEL_VERSION, HirFile, lower_ast};

/// A fixture that exercises every compile-state layer in one file:
/// pragmas (L3), package/stash + `@ISA`/Exporter (L2), lexical/`our`/`state`/
/// `local` bindings (L1), HIR items (L0), imports/exports (L4), a phase block
/// and `require` feeding the effect log (L5), and an Exporter-family shape the
/// framework registry can read (L6).
const FIXTURE: &str = r#"
package Acme::Widget;
use strict;
use warnings;
use feature 'say';
use parent 'Acme::Base';
use Exporter 'import';
our @EXPORT_OK = ('build');

require Acme::Helper;

my $count = 0;
our $registry = {};
state $singleton;

BEGIN {
    $count = 1;
}

sub build {
    my ($class, %args) = @_;
    local $count = $count + 1;
    return bless { %args }, $class;
}

1;
"#;

fn lower_fixture() -> HirFile {
    let mut parser = Parser::new(FIXTURE);
    let output = parser.parse_with_recovery();
    lower_ast(&output.ast)
}

/// C1: one lowering pass yields all compile-state layers.
#[test]
fn compile_state_layers_all_present_from_single_pass() {
    let file = lower_fixture();

    // L0 HIR items.
    assert!(!file.items.is_empty(), "L0: expected lowered HIR items");

    // L1 scope/pad: a root file scope plus at least one lexical binding.
    assert!(file.scope_graph.root_scope().is_some(), "L1: expected a root scope frame");
    assert!(!file.scope_graph.bindings.is_empty(), "L1: expected lexical/package bindings");

    // L2 package/stash: at least one package was recorded.
    assert!(!file.stash_graph.packages.is_empty(), "L2: expected a package in the stash graph");

    // L3 compile environment: pragma state facts were recorded.
    assert!(
        !file.compile_environment.pragma_state_facts().is_empty(),
        "L3: expected pragma state facts from use strict/warnings/feature"
    );

    // L4 import/export/visible symbols: the static `our @EXPORT_OK = (...)`
    // declaration projects into at least one canonical export set.
    assert!(
        !file.stash_graph.export_sets().is_empty(),
        "L4: expected export sets from static @EXPORT_OK"
    );

    // L5 compile-time effects: the effect log is non-empty.
    assert!(!file.compile_effects().is_empty(), "L5: expected a non-empty compile-effect log");

    // L6 framework adapters: the Exporter-family adapter reads the fixture's
    // `use Exporter` + `@EXPORT_OK` shape and projects at least one exported
    // symbol fact through `HirFile::framework_facts`.
    assert!(
        !file.framework_facts().exported_symbols.is_empty(),
        "L6: expected Exporter-family exported-symbol facts"
    );
}

/// C3: the compile-effect log is source-ordered with contiguous ordinals from 0
/// and every effect carries the current model version.
#[test]
fn compile_effect_log_is_ordered_and_versioned() {
    let file = lower_fixture();
    let effects = file.compile_effects();
    assert!(!effects.is_empty(), "expected compile effects");

    for (index, effect) in effects.iter().enumerate() {
        assert_eq!(
            effect.ordinal as usize, index,
            "effect ordinals must be contiguous from 0 in source order"
        );
        assert_eq!(
            effect.model_version, COMPILE_EFFECT_MODEL_VERSION,
            "every effect must stamp COMPILE_EFFECT_MODEL_VERSION"
        );
    }
}

/// C3: lowering is deterministic — the same source yields an identical layered
/// `HirFile` and an identical effect log every time.
#[test]
fn lowering_is_deterministic() {
    let first = lower_fixture();
    let second = lower_fixture();
    assert_eq!(first, second, "lowering the same source must produce an identical HirFile");
    assert_eq!(
        first.compile_effects(),
        second.compile_effects(),
        "the compile-effect log must be deterministic"
    );
}

/// C3 guard: the spec pins the effect model at version 1. A bump is allowed, but
/// it must be a deliberate change that updates the spec and this proof together.
#[test]
fn compile_effect_model_version_is_pinned() {
    assert_eq!(
        COMPILE_EFFECT_MODEL_VERSION, 1,
        "COMPILE_EFFECT_MODEL_VERSION changed; update PLSP-SPEC-0030 C3 and this proof"
    );
}
