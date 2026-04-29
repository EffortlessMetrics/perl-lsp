---
description: Architecture reviewer step 2 — verify dependency direction, crate boundaries, type placement
user-invocable: false
---

# Architecture: Check

Verify the proposed design respects the codebase's structural contracts.

## Synthesize with prior agents (do this BEFORE running structural checks)

You run after accuracy, research, oppositional, and diaboli. Their comments are on the issue. Your verdict must *engage* with theirs — your job is to add the structural-alignment lens, not echo what earlier agents already said.

For each prior agent comment:

- **accuracy-scout** — file paths and function names corrected? If the proposed design references wrong crate locations, the structural analysis changes. Evaluate against the *corrected* locations.
- **research-verifier** — external claims verified or debunked? If Perl/LSP/crate API claims were debunked, the proposed architecture may be unsound at the foundation — name the structural implication.
- **oppositional-planner** — scope-pivots or alternative approaches surfaced? If a pivot was proposed, evaluate the structural fit of the pivot, not just the original design. If the pivot is structurally cleaner, say so.
- **advocatus-diaboli** — BUILD / DEFER / CLOSE? Diaboli's scope is PREMISE (is the work right in principle?); yours is STRUCTURAL FIT. If diaboli flagged architectural risk, that's potentially *your* lane — engage directly with the structural concern, don't just concur.

**If the issue is part of a committed tracker / ADR / roadmap milestone:** the structural pattern was already decided. Start at ALIGNED and look for NEW structural concerns that weren't visible when the decision was made — don't re-litigate the original architecture from scratch.

## Checks

1. **Dependency direction** — dependencies must flow downward (leaf → core → feature → provider → server). Check:
   ```bash
   # Would the proposed change create an upward dependency?
   cargo tree -p <upstream-crate> -i | grep <downstream-crate>
   ```

2. **Crate boundary** — one crate, one concern. If the spec adds multiple responsibilities to one crate, flag it.

3. **Type placement** — new types belong in the lowest crate that needs them. Check if the proposed type location forces unnecessary dependencies.

4. **Cross-layer bridges** — feature crates must not depend on each other. Check:
   ```bash
   grep -r "perl-lsp-completion\|perl-lsp-diagnostics\|perl-lsp-folding" crates/<proposed-crate>/Cargo.toml
   ```

5. **Feature catalog** — if this adds user-visible LSP capability, verify it's registered in `features.toml`.

6. **Pattern consistency** — does this follow existing patterns or introduce a new one? Check similar crates for precedent.
