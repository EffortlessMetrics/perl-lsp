---
description: Find and report dead code — unreachable functions, types, modules, and unused dependencies
argument-hint: "[crate] e.g. 'perl-parser' or empty for workspace-wide"
---

# Dead Code Check

Find dead code in **$ARGUMENTS** (default: full workspace).

## Steps

1. **Run dead code analysis**:
   ```bash
   just dead-code                         # Full report
   just dead-code-report                  # JSON report
   just dead-code-strict                  # Fail on any dead code
   cargo machete                          # Unused dependencies
   ```

2. **For each candidate**, verify it is truly unreachable:
   - Not just uncalled from tests — check all callers
   - Check git blame — is this recent work-in-progress?
   - Check if the item is behind a feature flag
   - Check if the item is a pub API used by external consumers
   - Check if the item is referenced in docs or examples

3. **Classify each item**:
   - **Safe to remove**: truly unreachable, not WIP, not pub API
   - **Needs investigation**: ambiguous reachability
   - **False positive**: used via feature flags, macros, or external consumers

4. **For safe removals**, verify:
   ```bash
   cargo build --workspace && cargo test --workspace --lib
   ```

## Safety Rules

- Do not remove pub items that might be used by external consumers
- Do not remove items behind feature flags
- Do not remove test utilities used by other crate tests
- Do not remove items referenced in docs or examples
