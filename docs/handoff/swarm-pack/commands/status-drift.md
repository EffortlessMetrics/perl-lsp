---
description: Check and fix computed metric drift (status, baselines, manifests)
argument-hint: "[--check-only] [--commit]"
---

# Status Drift

Regenerate computed metrics and ratchet baselines. Context: **$ARGUMENTS**

Run after every ~5 merges, or after any bug-fix merge.

## Steps

### 1. Regenerate status
```bash
$STATUS_REGEN_CMD
# If changed and --commit:
# git add <status-file> && git commit -m "chore(ci): update status"
```

### 2. Ratchet baselines
```bash
$BASELINE_RATCHET_CMD
# If improved and --commit:
# git add <baseline-file> && git commit -m "chore(ci): ratchet baseline"
```

### 3. Report
| Metric | Before | After | Status |
|--------|--------|-------|--------|
| Status | ... | ... | updated/unchanged |
| Baseline | N | M | ratcheted/unchanged |
