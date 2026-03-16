---
description: Review a single PR — read diff, fix issues, push fixes, mark ready
argument-hint: "<PR number>"
---

# /review-pr

Review exactly ONE pull request end-to-end. This skill enforces the one-PR-per-agent rule: each review agent handles one PR, never batches.

## Steps

### 1. Load PR context

```bash
gh pr view $ARGUMENTS --json number,title,body,headRefName,baseRefName
gh pr diff $ARGUMENTS
```

Read the title, body, and full diff. Understand **what** the PR does and **why**.

### 2. Check out the branch

```bash
BRANCH=$(gh pr view $ARGUMENTS --json headRefName -q '.headRefName')
git fetch origin "$BRANCH"
git checkout "$BRANCH"
```

### 3. Review checklist

Walk through the diff and check every item:

**Fatal constructs (hard blockers)**
- [ ] No `unwrap()` / `expect()` in production code (tests may use `Result<()>` or `must`/`must_some` helpers)
- [ ] No `panic!()` / `todo!()` / `unimplemented!()` in production code
- [ ] No `dbg!()` — use `tracing::debug!` instead
- [ ] No `std::process::exit()` outside `bin/` directories and `lifecycle.rs`

**Code quality**
- [ ] Imports are clean (no unused imports)
- [ ] No unnecessary `.clone()` on Copy types
- [ ] Uses `.first()` instead of `.get(0)`
- [ ] Uses `.push(char)` instead of `.push_str("x")` for single chars
- [ ] Uses `or_default()` instead of `or_insert_with(Vec::new)`
- [ ] Regex uses `Option<Regex>` with `.ok()` for graceful degradation

**Tests**
- [ ] Test names describe behavior (not `test1`, `test2`)
- [ ] Assertions are meaningful (not just `assert!(true)`)
- [ ] Tests use `Result<()>` return types or `must`/`must_some` helpers

**Documentation**
- [ ] Doc comments are accurate and useful
- [ ] Public items have doc comments
- [ ] No stale comments that contradict the code

**Formatting and lint**
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --lib` passes (or at minimum the affected crate)

### 4. Fix issues found

For each issue:
1. Fix it directly in the code
2. Stage the fix: `git add <specific files>`
3. Commit with a descriptive message:
   ```bash
   git commit -m "review: fix <description of issue>"
   ```

Do NOT batch unrelated fixes into one commit. One commit per logical fix.

### 5. Push fixes

```bash
git push
```

### 6. Verify

Run the verification gate for affected crates:
```bash
cargo fmt --all -- --check
cargo clippy -p <crate> --tests
cargo test -p <crate>
```

### 7. Mark ready

If all checks pass and no blockers remain:
```bash
gh pr ready $ARGUMENTS
```

If blockers remain that you cannot fix, leave a review comment and do NOT mark ready:
```bash
gh pr review $ARGUMENTS --comment --body "Review blocked: <description of remaining issues>"
```

### 8. Report

Summarize:
- Issues found (with file:line references)
- Issues fixed (with commit hashes)
- Remaining blockers (if any)
- Overall assessment: ready / needs-work / blocked

## Rules

- **One PR per invocation.** Never review multiple PRs in a single agent context.
- **Different PRs are different context sets.** Spawn separate agents for separate PRs.
- **Fix what you can, file what you cannot.** If a problem is out of scope, create a GitHub issue with `--label swarm-discovered`.
- **Do not merge.** Review and fix only. The merger handles merges.
