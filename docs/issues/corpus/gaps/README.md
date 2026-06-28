# Corpus Gap Index

This directory contains documented gaps in the parser's corpus coverage.

**Status**: Most gaps have been addressed. Remaining items tracked below.

---

## Summary

| Category | Count | Priority | Status |
|----------|-------|----------|--------|
| GA Feature Missing Coverage | 4 | P0 | 4 resolved |
| NodeKind Coverage Status | 4 | P1 | 3 resolved, 1 clarified |
| Timeout/Hang Risks | 13 | P0-P2 | tracking |
| Strengthening Fixtures | 2 | P2 | added (#1381, #1383) |

**Note**: The parser has 59 NodeKind variants (not 68 as previously stated).

---

## GA Feature Missing Coverage (P0)

Features advertised as GA but lacking test fixtures:

- ~~[continue-redo-statements](ga-feature-missing-coverage/continue-redo-statements.md)~~ ✅ RESOLVED (#1365) - corpus fixtures (`continue_redo_statements.pl`, `loop_control_comprehensive.pl`) + `NodeKind::LoopControl` covered at 21 angles; explicit guards in `perl-parser-core/tests/fix_1365_loop_control_nodekind.rs`
- ~~[format-statements](ga-feature-missing-coverage/format-statements.md)~~ ✅ RESOLVED - corpus added
- ~~[glob-expressions](ga-feature-missing-coverage/glob-expressions.md)~~ ✅ RESOLVED - corpus added
- ~~[tie-interface](ga-feature-missing-coverage/tie-interface.md)~~ ✅ RESOLVED (#1366) - `NodeKind::Tie`/`Untie` exist (`perl-ast/src/ast.rs`) and are corpus-covered (Tie=3, Untie=2 angles); explicit guards in `perl-parser-core/tests/fix_1366_tie_untie_nodekind.rs`

**Status**: All listed GA-feature coverage gaps are resolved.

---

## NodeKind Coverage Status (P1)

Status of NodeKinds previously flagged as "never seen":

- ~~[format](nodekind-never-seen/format.md)~~ ✅ RESOLVED - NodeKind exists, corpus added (`test_corpus/format_statements.pl`)
- ~~[glob](nodekind-never-seen/glob.md)~~ ✅ RESOLVED - NodeKind exists, corpus added (`test_corpus/glob_expressions.pl`)
- [sigil](nodekind-never-seen/sigil.md) - ⚠️ NOT A NODEKIND - sigils are fields in `Variable` nodes (intentional design)
- ~~[tie](nodekind-never-seen/tie.md)~~ ✅ RESOLVED - `Tie` (and `Untie`) ARE NodeKinds (`perl-ast/src/ast.rs`); both pass the corpus angle≥2 coverage gate

**Required action**: For Sigil, document design decision (sigils intentionally remain `Variable` fields).

---

## Strengthening Fixtures (P2)

Comprehensive edge-case fixtures added to broaden corpus coverage. All parse
cleanly and are enforced by the auto-discovery gate in `corpus_gap_tests.rs`.

- ~~obscure-perl-constructs~~ ✅ ADDED (#1381) - `test_corpus/obscure_perl_constructs.pl`: `__SUB__` recursion, `CORE::GLOBAL::` builtin override, explicit `CORE::` calls, smartmatch `~~`, scalar flip-flop, `vec` lvalue, nested-sigil variable-variables.
- ~~special-package-sections~~ ✅ ADDED (#1383) - `test_corpus/special_package_sections.pl`: lifecycle phasers (BEGIN/UNITCHECK/CHECK/INIT/END), AUTOLOAD/DESTROY magic methods, package version token, terminal `__END__` data section — exercised together in one module context.

---

## Timeout/Hang Risks (P0-P2)

Inputs that may cause parser hangs or excessive time:

### P0 (Must fix for v0.9)

- [ambiguous-slash-division-regex](timeout-hang-risks/ambiguous-slash-division-regex.md)
- [deep-nesting-stack-overflow](timeout-hang-risks/deep-nesting-stack-overflow.md)
- [catastrophic-regex-backtracking](timeout-hang-risks/catastrophic-regex-backtracking.md)

### P1

- [hash-vs-block-ambiguity](timeout-hang-risks/hash-vs-block-ambiguity.md)
- [indirect-object-syntax-ambiguity](timeout-hang-risks/indirect-object-syntax-ambiguity.md)
- [complex-quote-operator-delimiters](timeout-hang-risks/complex-quote-operator-delimiters.md)
- [multiple-heredocs-single-line](timeout-hang-risks/multiple-heredocs-single-line.md)
- [recursive-heredoc-terminators](timeout-hang-risks/recursive-heredoc-terminators.md)

### P2

- [branch-reset-groups](timeout-hang-risks/branch-reset-groups.md)
- [regex-code-execution](timeout-hang-risks/regex-code-execution.md)
- [source-filter-code-execution](timeout-hang-risks/source-filter-code-execution.md)
- [unicode-property-regex](timeout-hang-risks/unicode-property-regex.md)
- [variable-length-lookbehind](timeout-hang-risks/variable-length-lookbehind.md)

**Required action**: Add boundedness tests that prove parser terminates in acceptable time.

---

## Closing Gaps

For each gap:

1. Create a minimal fixture that exercises the feature/NodeKind
2. Add a test that validates correct behavior
3. For hang risks: add a boundedness test with timeout assertion
4. Update this index when fixed

See [Corpus Audit Tooling](../README.md) for running coverage analysis.
