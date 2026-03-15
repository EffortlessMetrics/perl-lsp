---
description: Check and fix CURRENT_STATUS, corpus baseline, and CPAN manifest drift
argument-hint: "[--check-only] [--commit]"
---

# Status Drift

Regenerate computed metrics and ratchet baselines forward. Context: **$ARGUMENTS**

## Protocol

This should run after every ~5 merges, or after any parser fix merge.

### 1. CURRENT_STATUS.md
```bash
python3 scripts/update-current-status.py
git diff docs/project/CURRENT_STATUS.md
```
If changed and `--commit`:
```bash
git add docs/project/CURRENT_STATUS.md
git commit -m "chore(ci): update CURRENT_STATUS.md"
```

### 2. Corpus baseline (after parser fixes)
```bash
just corpus-sweep-update 2>/dev/null
git diff .ci/parser-corpus-baseline.json
```
If improved and `--commit`:
```bash
git add .ci/parser-corpus-baseline.json
git commit -m "chore(ci): ratchet corpus baseline"
```

### 3. CPAN manifest (after parser fixes)
```bash
just cpan-corpus-ratchet 2>/dev/null
git diff .ci/cpan-corpus-manifest.txt
```
If improved and `--commit`:
```bash
git add .ci/cpan-corpus-manifest.txt
git commit -m "chore(ci): ratchet CPAN corpus manifest"
```

### 4. Common corpus check
```bash
just common-corpus-check
```
If failing, this is a regression — do NOT update the manifest. Investigate.

### 5. Report
| Metric | Before | After | Status |
|--------|--------|-------|--------|
| CURRENT_STATUS | ... | ... | updated/unchanged |
| Corpus baseline | N clean | M clean | ratcheted/unchanged |
| CPAN manifest | N modules | M modules | ratcheted/unchanged |
