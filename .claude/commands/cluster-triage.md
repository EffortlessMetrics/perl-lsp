---
description: Ensemble-curator step 6 — for a detected cluster, pick winner(s) by file-path, extract edge cases from losers, close dupes with cross-ref
---

# Cluster Triage

Given a cluster detected by `/ensemble-detect`, pick winner(s) and consolidate.

See `docs/articles/BROAD_SCOPE_LAYER_DIVERSITY.md` for the underlying file-path-over-title-triage principle.

## The rule

**File-path first, title second.** Two PRs with similar titles touching the same file + same function = duplicates. Different files = layer diversity, keep both.

## Procedure

### 1. Map files per PR

```bash
for pr in <PR-LIST>; do
  echo "=== #$pr ==="
  gh pr diff $pr --name-only
done
```

### 2. Bucket by file-set

- **Same-file cluster**: 2+ PRs touching identical file sets → real duplicate. Pick winner. Close losers.
- **Layer-diversity cluster**: PRs touch disjoint file sets → complementary, keep ALL. Note this as a broad-scope ensemble in the synthesis comment.
- **Overlap-but-not-identical**: some shared files + some unique → mixed. Winner is the PR with the fuller implementation on the shared files; unique-file contributions from others may still merge independently.

### 3. Pick winner(s) by ranking signals

For same-file clusters, rank PRs by:

1. **Completeness** — does the diff implement the full spec, or only half? (Check PR body acceptance criteria.)
2. **Safety** — uses `Result<()>` / `?`, no `unwrap()` / `expect()` in production.
3. **Test coverage** — has regression tests, not just happy-path.
4. **API tightness** — no unnecessary `as usize`, no cloning where borrow works, no duplicated helper code.
5. **Commit history** — clean linear vs. 8 "fix fix fix" commits.

Pick the highest-ranked. If tied, prefer the PR that was created earliest (stability).

### 4. Extract loser edge cases (cross-pollination)

**Before** closing each loser, read its diff specifically for:

- Additional test cases the winner lacks
- Comments documenting gotchas the winner doesn't mention
- Handling of a variant (e.g., CRLF line-endings, empty input, non-BMP unicode) the winner missed

Options for extraction:

- Post the specific test code as a comment on the winner's PR with "Extracted from #<loser>; recommend adding"
- Push a one-line follow-up commit to the winner's branch adding the edge case
- Note the variant as a follow-up issue if non-trivial

### 5. Close losers with cross-ref

```bash
gh pr close <LOSER> -c "Closing as REDUNDANT — #<WINNER> implements the same scope with <reason: more complete / cleaner / better tests>. Your contributions:
- <novel edge case 1>: extracted as comment on #<WINNER>
- <novel approach 2>: considered; winner's approach preferred because <reason>

Thank you for the contribution — the ensemble perspective helped surface the right approach."
```

### 6. Emit verdict

For winner(s): ALIGNED → continue to `/emit-verdict`
For losers: REDUNDANT → already closed in step 5

## Example: encoding cluster from 2026-04-23

**Detected cluster:** 12 PRs tagged "encoding" / "mojibake" / "UTF".

**File mapping:**

| PRs | Files |
|---|---|
| #5740, #5741, #5743 | `workspace.rs` (same file) |
| #5742 | `util/mod.rs`, `navigation.rs` |
| #5738 | `perl-uri` crate |
| #5739 | `perl-critic` crate |
| #5736, #5737 | `perl-parse` CLI |
| #5732 | URI module |
| #5733 | `position-tracking` |
| #5734, #5735 | Code-actions pragma |

**Triage:**

- Same-file cluster `workspace.rs`: 3 PRs competing. Winner #5743 (lossy UTF-16 with BOM detection, handles odd-length). Close #5740 (strict decode, worse than original), #5741 (no UTF-16). Extract: #5741's test for non-UTF8 reads → added to #5743.
- Same-file cluster pragma detection: 2 PRs. Winner #5735 (regex-based, case-insensitive). Close #5734 (inline, case-sensitive). Extract: nothing novel in loser.
- Everything else: layer-diversity, all KEEP.

**Synthesis comment on #5743 (the hub winner):**

> Ensemble learnings from the encoding cluster (12 PRs, 8 layers):
> - Encoding spans 6 layers: workspace file read, util decode helper, URI parser, LSP navigation, CLI binary, critic output
> - Winning approach is lossy UTF-16 with BOM detection (strict-decode silent-skip was rejected)
> - Code-actions pragma detection is separable; kept in own PR (#5735)

## What NOT to do

- Don't close based on title similarity alone — file-paths are decisive.
- Don't close a cluster member without extracting its novel contributions first.
- Don't pick a winner arbitrarily when tied — prefer earliest creation for stability.

## Output

```
cluster-triage: N=<count>
  winners: <list of winners>
  closures: <list of closures with one-line rationale each>
  extractions: <list of edges pulled from losers into winner>
  synthesis: <one-paragraph learning-from-cluster>
```

Pass winners to `/emit-verdict` for ALIGNED → reviewer-deep routing. Closures are final.
