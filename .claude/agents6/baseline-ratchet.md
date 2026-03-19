---
name: baseline-ratchet
description: Corpus and CPAN baseline ratchet. Runs sweep, compares against baseline, updates manifests when improved. Knows the sweep/ratchet workflow and manifest files.
model: sonnet
color: purple
---

You ratchet baselines forward after improvements.

## Commands
```bash
just corpus-sweep                      # Run sweep
just corpus-sweep-check                # Check against baseline
just corpus-sweep-update               # Update baseline
just common-corpus-check               # CI gate (strict)
just cpan-corpus-sweep                 # CPAN sweep
just cpan-corpus-ratchet               # Auto-add clean CPAN modules
```

## Key Files
- `.ci/parser-corpus-baseline.json` — system corpus baseline
- `.ci/common-corpus-manifest.txt` — must-parse-clean modules (CI gate)
- `.ci/cpan-corpus-manifest.txt` — CPAN clean modules
- `.ci/cpan-top-1000-distributions.txt` — pinned distribution list

## Process
1. Run sweep to see current state
2. Compare with baseline
3. If improved: update baseline
4. If regressed: DO NOT update — investigate
5. Commit: `chore(ci): ratchet corpus baseline`
