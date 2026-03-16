---
description: Run corpus sweep, compare baseline, update manifests
argument-hint: "[--system|--cpan|--common] [--update]"
disable-model-invocation: true
---

# Corpus Ratchet

Run parser corpus sweep and ratchet the baseline forward. Mode: **$ARGUMENTS**

## Commands

### System corpus (default)
```bash
just corpus-sweep
```

### Compare against baseline
```bash
just corpus-sweep-check
```

### Common corpus (CI gate — strict)
```bash
just common-corpus-check
```

### CPAN corpus
```bash
just cpan-corpus-sweep
```

### Update baseline after improvements
```bash
# 1. Run sweep to see current state
just corpus-sweep

# 2. If clean files increased, update baseline
just corpus-sweep-update

# 3. Check for newly-clean modules to add to manifest
# (manually review which modules are now clean)

# 4. Verify the ratchet holds
just corpus-sweep-check
just common-corpus-check

# 5. Commit baseline + manifest updates
git add .ci/parser-corpus-baseline.json .ci/common-corpus-manifest.txt
git commit -m "ci: ratchet corpus baseline after parser improvements"
```

### CPAN corpus ratchet
```bash
just cpan-corpus-ratchet
```

## Key files
- `.ci/parser-corpus-baseline.json` — system corpus baseline (ratchet floor)
- `.ci/common-corpus-manifest.txt` — modules that MUST parse cleanly (CI gate)
- `.ci/cpan-corpus-manifest.txt` — CPAN modules that must parse cleanly
- `.ci/cpan-top-1000-distributions.txt` — pinned distribution list
