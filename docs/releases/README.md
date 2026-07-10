# Release-note authoring guide

This directory records what each `perl-lsp` release actually contained.
Release notes are a product and provenance artifact, not a generated dump of
whatever commits happen to appear between two tags.

Use this guide together with:

- [`RELEASE_CHECKLIST.md`](../project/RELEASE_CHECKLIST.md) for the cut gate;
- [`sync-protocol.md`](../swarm/sync-protocol.md) for cross-repository mechanics;
- [`RELEASE.md`](../../RELEASE.md) for publication operations;
- [`RELEASE_HISTORY.md`](../../RELEASE_HISTORY.md) for channel closeout.

## The history model

`perl-lsp-swarm/main` is the active development line. Each merged PR is normally
one logical squash commit. `perl-lsp/master` is the canonical release and package
lineage.

A release therefore has two distinct comparisons:

1. **Logical development range** — previous swarm release anchor through the new
   swarm RC/freeze. This is the primary input for release-note completeness,
   authorship, PR attribution, and change classification.
2. **Release-tree range** — previous `perl-lsp` tag through the new source tag.
   This verifies the final packaged tree and release-lineage changes.

These ranges usually describe the same product delta, but they are not
interchangeable. A delayed history import, emergency source fix, excluded swarm
harness, or incorrect snapshot sync can make the source tag range too small,
too large, or historically misleading.

## Non-negotiable rule

> Draft release notes from the logical swarm squash-commit range, then verify
> them against the final release tree. Do not use `previous_tag..new_tag` as the
> sole source of truth for a cross-repository release.

## Required provenance record

Before writing prose, record this block in the release PR and the release note:

| Field | Required value |
|---|---|
| Development repository | `EffortlessMetrics/perl-lsp-swarm` |
| Previous development anchor | Exact RC/freeze SHA associated with the previous release |
| New development freeze | Exact reviewed swarm SHA being promoted |
| Logical range | `<previous-anchor>...<new-freeze>` |
| Release repository | `EffortlessMetrics/perl-lsp` |
| Release sync commit | Exact two-parent sync SHA |
| Sync method | History-preserving complete-tree merge |
| Exclusions | Exact paths intentionally different from swarm |
| Tree verification | Result of source-versus-swarm diff after exclusions |
| Source tag comparison | `safe`, `inflated`, `incomplete`, or `tree-only` with explanation |

Do not write `latest main`, `current head`, or another moving reference in the
final record. Resolve every input to an immutable SHA.

## Authoring workflow

### 1. Freeze the development boundary

Record the previous release's development anchor and the new RC SHA before the
release sync begins.

```bash
export PREVIOUS_RC=<previous-swarm-release-sha>
export RC_SHA=<new-swarm-freeze-sha>
```

Confirm both commits exist and the range is forward-moving:

```bash
git -C ../perl-lsp-swarm merge-base --is-ancestor "$PREVIOUS_RC" "$RC_SHA"
```

### 2. Enumerate logical changes

Use first-parent history so each squash-merged PR remains one reviewable unit:

```bash
git -C ../perl-lsp-swarm log \
  --first-parent \
  --reverse \
  --format='%H%x09%s' \
  "$PREVIOUS_RC..$RC_SHA"
```

Do not substitute a raw all-parent commit count. Merge ancestry, reverse syncs,
and imported history can make that count meaningless for release narration.

### 3. Build a classification ledger

Classify every logical commit before compressing it into prose.

| Commit / PR | Surface | Disposition | Release-note destination |
|---|---|---|---|
| `feat` / user-visible `fix` | LSP, DAP, parser, extension, CLI, install | Include | User or integration section |
| Protocol-shape change | LSP/DAP client contract | Include | Editor integrations / debugger |
| Parser acceptance or recovery | Valid-code coverage or diagnostic quality | Include | Parser and diagnostics |
| Internal substrate | HIR/PIR, shadow path, receipt, gate | Include with boundary or omit | Under the hood / non-claims |
| Test-only proof | No behavior change | Usually omit or summarize | Validation |
| Swarm-only orchestration | `.claude`, agent scripts, private receipts | Exclude | Exclusions record |
| Revert / superseded work | Does not survive RC tree | Exclude with reason | Ledger only |

Every user-visible `feat` and `fix` must be represented in the final notes or
carry an explicit exclusion reason. This is the completeness check that a large
sync commit cannot provide.

### 4. Verify the history-preserving sync

After the complete-tree merge lands in `perl-lsp`, verify ancestry before the
version bump or tag:

```bash
export SYNC_SHA=<perl-lsp-sync-commit>

git merge-base --is-ancestor "$RC_SHA" "$SYNC_SHA"

test "$(git show -s --format='%P' "$SYNC_SHA" | wc -w)" -eq 2
```

Verify the release tree differs from the RC only by documented exclusions:

```bash
git diff --name-only "$SYNC_SHA" "$RC_SHA"
```

The exact command may need explicit remotes or temporary refs, but the invariant
is stable: the promoted RC must be an ancestor, the sync must have two parents,
and tree differences must be explained.

A content snapshot, patch replay, archive copy, or one-parent replacement commit
is not an acceptable release sync when logical commit history exists.

### 5. Draft from behavior, not commit subjects alone

Commit subjects are the index. Read the PR body, tests, claim boundary, and final
RC implementation before stating what users received.

Prefer:

- concrete editor or runtime behavior;
- the condition under which it activates;
- fallback and fail-closed behavior;
- important compatibility or migration effects;
- explicit non-claims for scaffolded or disabled work.

Avoid:

- treating tests, receipts, or gates as product features;
- claiming a planned provider or disabled feature shipped;
- listing every internal refactor as user-facing;
- compressing a large feature family into “improvements”;
- relying on PR count or lines changed as evidence of value.

### 6. Verify against the final tree

After drafting from the logical range:

- inspect the final tag/tree for each material claim;
- ensure reverted or excluded work is absent;
- identify release-repo-only fixes added after the swarm freeze;
- confirm install paths, binaries, extension behavior, and channel caveats;
- state whether publication is verified or merely pending in the ledger.

### 7. Review the source compare honestly

Classify the source tag comparison:

- **safe** — logical release history is already ancestral and the tag range maps
  cleanly to the release;
- **inflated** — the range imports older history, as happened in 0.17 when the
  missing 0.16 swarm ancestry was connected;
- **incomplete** — the release tree contains content whose logical commits are
  not ancestors, as happened at the 0.16 tag;
- **tree-only** — useful for final-file verification but not logical accounting.

Put the classification in the release note whenever it is not `safe`.

## Release-note structure

Use the following structure unless the release has a strong reason to differ:

```markdown
# vX.Y.Z

## Summary

## Release provenance

## What improved for users

### Setup / toolchain / CLI
### Symbols, navigation, completion, diagnostics
### Parser and formatter
### Debug adapter

## What changed for editor integrations

## Internal foundations and non-claims

## Install / package / release path

## Known limitations

## Validation performed

## Related
```

Keep the summary short. Put breadth into grouped sections rather than one dense
paragraph or an undifferentiated commit list.

## Claim boundaries

Release notes must distinguish these states:

| State | Allowed wording |
|---|---|
| Live and default | “now does” / “now supports” |
| Live but capability- or config-gated | Name the exact gate and fallback |
| Shadow / comparison only | “records”, “compares”, or “measures”; no live cutover claim |
| Disabled scaffold | “added a boundary/scaffold”; state no editor-visible behavior |
| Test or receipt only | Put under validation, not user features |
| Planned | Do not place in shipped changes |

## Historical correction procedure

When a past release note is incomplete or its ancestry is wrong:

1. Do not silently move or recreate the published tag.
2. Add a dated provenance correction to the existing note.
3. Identify the exact release tree, development RC, sync commit, and merge base.
4. State whether the logical commits are currently reachable in the canonical
   repository and when they became reachable.
5. Reconstruct the note from the logical development range and final tagged tree.
6. Mark later source tag comparisons that are inflated by delayed history import.
7. Add a prevention check to the current release process.

The objective is an honest archive, not a cleaner-looking graph.

## Pre-cut acceptance

A release is not ready to tag until all of the following are true:

- the development RC is immutable and recorded;
- the logical first-parent commit list has been reviewed;
- all user-visible features and fixes are included or explicitly excluded;
- the sync commit has exactly two parents;
- the RC is an ancestor of the sync commit;
- the release tree differs only by documented exclusions;
- the release note contains a provenance section and honest compare
  classification;
- disabled, shadow-only, and proof-only work has correct claim boundaries;
- the generated GitHub Release body was compared with the curated note;
- publication status is not claimed ahead of channel verification.
