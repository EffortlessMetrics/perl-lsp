---
description: Diff auditor step 2 — post findings and set label
user-invocable: false
---

# Diff Audit: Comment

Post your findings and set the appropriate label.

## Operating principle (per the 2026-04-26 directive)

Sign-off IS one of the routing decisions. Each pass produces ONE outcome:

- **CLEAN** → apply `diff-audited` (and only `diff-audited`)
- **ARTIFACTS / REGRESSION / SCOPE DRIFT / CONTAMINATION** → apply `needs-diff-fix` (and only `needs-diff-fix`); do **NOT** also apply `diff-audited`

Default posture: every PR is potentially problematic. "CLEAN, nothing to flag" on a 500+ line diff is almost never right — find a specific concrete observation (regression risk, artifact, test gap, contamination, sketchy commit). Mechanical box-checking output without substantive observations is itself a signal you didn't look hard enough.

## Pre-comment checks (must run before deciding verdict)

1. **Cross-PR source-file contamination** (per `feedback_agent_audit_trail_directories.md` 2026-04-25 update + 2026-04-26 #5870 incident): contamination can live in regular source/test files, not just `.hermes/`. For each file in the diff, ask: "does this path/content align with the PR's stated scope (title + body + linked issue)?" If diff adds tests for crate X but the title is about crate Y (and they aren't named in the spec), flag as CONTAMINATION. Tell-tale: PR title claims a small change but `--stat` shows >100 lines outside the named scope.

2. **Master-green guard** (per the 2026-04-26 master-green directive): verify the PR's CI includes workspace-wide checks SUCCESS (`Compile All Targets`, `PR Smoke` with workspace fmt, `Windows Guardrails`), not just per-crate. Per-crate green is necessary but not sufficient — workspace-wide cascades break master after merge. If `PR Smoke` shows fmt drift in any file (PR's own OR a recently-merged unrelated file the branch hasn't picked up), route to needs-ci-fix with cascade-update instruction.

## Steps

1. Post comment:
   ```bash
   gh pr comment <number> --body "$(cat <<'EOF'
   ## Diff Audit

   **Files changed:** <count>
   **Lines:** +<added> -<removed>
   **Commits:** <count> (<list>)

   ### Spec alignment: [COMPLETE | PARTIAL | DRIFT]
   <acceptance criteria coverage>

   ### Cleanliness: [CLEAN | ARTIFACTS FOUND]
   <leftover TODOs, debug code, out-of-scope files>

   ### Commit coherence: [CLEAN | MESSY]
   <commit history quality>

   ### Verdict: [CLEAN | ARTIFACTS | REGRESSION | SCOPE DRIFT]
   <one sentence>

   ---
   *Diff auditor — final coherence check before merge.*
   EOF
   )"
   ```

2. If CLEAN:
   ```bash
   gh pr edit <number> --add-label "diff-audited"
   ```

3. If not CLEAN:
   ```bash
   gh pr edit <number> --add-label "needs-diff-fix"
   ```
