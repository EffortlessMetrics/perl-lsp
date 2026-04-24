---
description: Architecture reviewer step 2 — verify dependency direction, crate boundaries, type placement
user-invocable: false
---

# Architecture: Check

Verify the proposed design respects the codebase's structural contracts.

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
