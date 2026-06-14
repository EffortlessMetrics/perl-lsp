# Context: #1297 — Validate NodeKind safe_for_breakpoint/introduces_scope flag values before Phase 7/8 DAP consumption

## Problem

The NodeKind classification keystone (PR #930, merged into #1295) defines 61 variants with static flags (executable, introduces_scope, declares_symbol, references_symbol, contains_children, recovery_artifact, safe_for_breakpoint). Before Phase 8 (DAP breakpoint validator consumer) depends on safe_for_breakpoint as a prefilter, nine flag values must be validated against real Perl debugger behavior (perl 5.40.1 + perl-d / DB module) and DAP 1.x semantics.

**User impact:** Silent breakpoint failures in DAP sessions if flags are wrong. Users expect `use strict;` to not be stoppable; if the flag is true, DAP might incorrectly offer a breakpoint on that line, then fail to verify it at runtime.

**System impact:** Phase 8 (separate PR) will add a DAP setBreakpoints handler that reads safe_for_breakpoint as a quick prefilter and returns per-breakpoint verified status. If the prefilter is wrong, breakpoints are either silently accepted (false positive) or rejected (false negative).

## Why this approach

The ratification is complete (ChatGPT-Pro + Perl debugger probe delivered results on 2026-06-13 comment; see #1297). The approach is:

1. **Encode the evidence directly into the code.** Flip Use/No from true to false (compile-time, not breakable). Keep Eval/Class/Goto/Typeglob/PhaseBlock true/false as proven (code-analyzable via parser AST).

2. **Document instance-dependent rows in code and docs.** Variant-level flags are static; some constructs (Eval, Package, PhaseBlock) have instance-dependent semantics. Encode this in doc comments and a PARSER_CONTRACTS.md section. Consumers must layer instance checks on top.

3. **No enum changes, no consumer migration.** This is a ratification-only PR. The classification infrastructure is unchanged. Phase 8 (separate PR) adds the consumer that reads these flags.

4. **Conservative prefilter design.** Variant-level flags are prefilters for quick rejection. If a flag is false, the construct is never breakable. If a flag is true, the consumer must verify per instance.

**Decision rationale:**

- **Why flip Use/No to false?** Perl debugger (perl 5.40.1 -d) reports `use strict;` and `no warnings;` lines as "not breakable" because they run at compile time before the debugger attaches. No break is possible.

- **Why keep Eval true?** The eval **statement** source line is breakable (debugger accepts `b eval` in perl 5.40.1). However, breakpoints inside the eval'd string are handled separately by the DAP layer at runtime.

- **Why keep Class true?** Perl 5.40.1 debugger accepts breakpoints on `class Foo {` header and body lines. The variant flag matches observed behavior.

- **Why keep Goto true?** Executable statement; one NodeKind::Goto { target } per construct. Pre-goto breakpoint is valid (though control transfers after).

- **Why keep Typeglob false?** `*alias = \&orig` is assignment, not executable statement. References symbols, does not execute. No breakpoint.

- **Why instance-dependent Eval.introduces_scope?** Parser emits `NodeKind::Eval { block }` for both `eval { ... }` (static block scope) and `eval "string"` (no static scope). Variant flag is conservative (true). Consumer must check if block is NodeKind::Block.

- **Why instance-dependent Package.introduces_scope/safe_for_breakpoint?** `package Foo {}` has scope and is breakable. `package Foo;` (statement form) has no scope and is not breakable. AST has `block: Option<Box<Node>>`. Variant flag is conservative (true). Consumer checks block.is_some().

- **Why instance-dependent PhaseBlock.safe_for_breakpoint?** Variant flag is true (all phases have block). BUT BEGIN/CHECK/UNITCHECK run at compile time, not stoppable in a runtime DAP session. END is stoppable. INIT is maybe (attach timing). DAP consumer checks phase field.

## Alternatives rejected

- **Per-phase variant flag for PhaseBlock.** Would require adding a new enum or tuple field on the flag struct. Rejected because: (a) instance-dependent logic belongs in the consumer (not bloating NodeKindFlags), (b) the DAP layer already checks phase name for other reasons (hover, semantics), (c) variant-level flag as prefilter + instance check is the design pattern for Eval/Package too.

- **Dedicated "instance_dependent" flag.** Would pollute NodeKindFlags for three special cases. Rejected because: (a) doc comments + consumer helpers are sufficient, (b) the builder can read PARSER_CONTRACTS.md to understand when to instance-check, (c) the pattern is consistent with other prefilters (e.g., recovery_artifact).

- **Wait for Phase 8 PR to validate flags.** Rejected because this validation must happen before Phase 8 consumes the flags. Encoding the evidence now ensures Phase 8 can proceed without re-validating. Also, a dedicated ratification PR is clearer than embedding the validation logic in the DAP consumer.

## Prior art / duplicates

**Perl debugger research:**
- Perl 5.40.1 -d (perldebug) behavior documented in perldoc perldebug, perlref (compile-time phases).
- ChatGPT-Pro code analysis + perl-debugger probe (2026-06-13) on #1297 confirmed compile-time pragma behavior (use/no not breakable).
- DAP spec (DAP 1.x) setBreakpoints / Breakpoint verified semantics documented in DAP_IMPLEMENTATION_SPECIFICATION.md.

**Related classification work:**
- PR #930 (merged into #1295) — classification keystone baseline (baseline flag values before ratification).
- Issue #1298 — phase-block hover work; unblocked PhaseBlock phase-name handling.
- Issue #1330 — NodeKindCategory drift-guard (exhaustiveness check; not touched by this PR).

**No duplicate ratification PR found.** This is the authoritative ratification; future DAP/scope consumers reference this PR + PARSER_CONTRACTS.md section as the contract.

## Links

- **Issue:** #1297 — Validate NodeKind safe_for_breakpoint/introduces_scope flag values
- **Ratification comment:** https://github.com/Perl-LSP/perl-lsp/issues/1297#issuecomment-2162903844 (ChatGPT-Pro + perl-debugger evidence on 2026-06-13)
- **Phase 8 follow-up:** Separate PR for DAP breakpoint validator (setBreakpoints handler) that reads safe_for_breakpoint as prefilter + instance-checks.
- **PARSER_CONTRACTS.md:** Will contain new §Breakpoint and Scope Classification Contract with static/instance-dependent rows and consumer implementation guidance.
- **Drift-guard:** Issue #1330 (NodeKindCategory exhaustiveness); not touched by this PR. Classification.rs flags() match has drift-guard (no wildcard).
- **Parser contracts:** `docs/reference/PARSER_CONTRACTS.md` (related sections: NodeKind variant classification; recovery artifact semantics).
- **Related issues:**
  - #1298 — Phase-block hover (unblocked by clarifying PhaseBlock behavior)
  - #930 / #1295 — Classification keystone baseline (ratified by this PR)
  - #1445 — DAP-fallback-child-ref-collision (parallel; different subsystem)

## Decision audit trail

**2026-06-11 (first partial answer in #1297):** PhaseBlock timing documented (compile phases not stoppable in runtime DAP; END stoppable; INIT maybe). Identified 3 debugger-semantics questions remaining.

**2026-06-11 (code-analysis half done):** Grep/code inspection resolved 5 of 9 flags (Typeglob, Goto, PhaseBlock, Use/No executable=true correct; Class executable=false correct). Identified 2 instance-dependent flags (Eval.introduces_scope, Package.introduces_scope/safe_for_breakpoint). 3 flags still need deep debugger research.

**2026-06-13 (ratification result):** ChatGPT-Pro + Perl debugger probe delivered final answers:
- Use/No safe_for_breakpoint → false (compile-time, not breakable)
- Eval/Class/Goto safe_for_breakpoint → keep true
- Typeglob/PhaseBlock unchanged
- Instance-dependent rows documented

**Decision:** Encode ratification as-is. No further research needed. Phase 8 implementation can proceed with confidence.
