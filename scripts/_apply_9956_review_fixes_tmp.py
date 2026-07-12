#!/usr/bin/env python3
"""One-shot guarded edits for unresolved review threads on PR #9956."""

from pathlib import Path


A87 = "a87f766ab60da513833dfff47349384be96fdae2"
BASE_016 = "151c5ecee69ef465836d2e7e173c310690391574"
SYNC_016 = "6925335fa4a5c142b3ce35b0134104b753b7ffd9"
FREEZE_017 = "33c4ee79a753eccdfdc2dab1de9e674f3ecaefe9"
SYNC_017 = "55bef776fa6865301142a3dd075f0472caa13f36"


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def update_release_checklist() -> None:
    path = Path("docs/project/RELEASE_CHECKLIST.md")
    replace_once(
        path,
        """export PREVIOUS_RC=<previous-swarm-release-sha>
export RC_SHA=<new-swarm-freeze-sha>
export SYNC_SHA=<perl-lsp-history-preserving-sync-sha>
""",
        """export SWARM_DIR="${SWARM_DIR:-../perl-lsp-swarm}"
export PREVIOUS_RC=<previous-swarm-release-sha>
export RC_SHA=<new-swarm-freeze-sha>
export SYNC_SHA=<perl-lsp-history-preserving-sync-sha>
""",
        "release checklist environment",
    )
    replace_once(
        path,
        '  git merge-base --is-ancestor "$PREVIOUS_RC" "$RC_SHA"\n',
        '  git -C "$SWARM_DIR" merge-base --is-ancestor "$PREVIOUS_RC" "$RC_SHA"\n',
        "release checklist swarm ancestry command",
    )
    replace_once(
        path,
        "  git log --first-parent --reverse --format='%H%x09%s' \"$PREVIOUS_RC..$RC_SHA\"\n",
        "  git -C \"$SWARM_DIR\" log --first-parent --reverse --format='%H%x09%s' \"$PREVIOUS_RC..$RC_SHA\"\n",
        "release checklist swarm ledger command",
    )
    replace_once(
        path,
        "- [ ] The release note classifies the source tag comparison as `safe`, `inflated`, `incomplete`, or `tree-only` when it is not a clean logical ledger.\n",
        "- [ ] The release note records the source tag comparison as `safe`, `inflated`, `incomplete`, or `tree-only`; include an explanation whenever it is not `safe`.\n",
        "release checklist source comparison classification",
    )


def update_authoring_guide() -> None:
    path = Path("docs/releases/README.md")
    replace_once(
        path,
        "| Logical range | `<previous-anchor>...<new-freeze>` |\n",
        "| Logical range | `<previous-anchor>..<new-freeze>`, traversed with `git log --first-parent --reverse` |\n",
        "authoring guide logical range semantics",
    )
    replace_once(
        path,
        """export PREVIOUS_RC=<previous-swarm-release-sha>
export RC_SHA=<new-swarm-freeze-sha>
""",
        """export SWARM_DIR="${SWARM_DIR:-../perl-lsp-swarm}"
export PREVIOUS_RC=<previous-swarm-release-sha>
export RC_SHA=<new-swarm-freeze-sha>
""",
        "authoring guide environment",
    )
    replace_once(
        path,
        'git -C ../perl-lsp-swarm merge-base --is-ancestor "$PREVIOUS_RC" "$RC_SHA"\n',
        'git -C "$SWARM_DIR" merge-base --is-ancestor "$PREVIOUS_RC" "$RC_SHA"\n',
        "authoring guide swarm ancestry command",
    )
    replace_once(
        path,
        "git -C ../perl-lsp-swarm log \\\n",
        "git -C \"$SWARM_DIR\" log \\\n",
        "authoring guide swarm ledger command",
    )


def update_v016_note() -> None:
    path = Path("docs/releases/v0.16.0.md")
    replace_once(
        path,
        """The `v0.16.0` source tree was promoted from `perl-lsp-swarm` RC
`a87f766ab` through source commit `6925335f` (PR #9909) as a **content-state
mirror**. That promotion preserved the complete release tree but did not make
the individual swarm squash-merge commits ancestors of `v0.16.0`.
""",
        f"""The `v0.16.0` source tree was promoted from `perl-lsp-swarm` RC
`{A87}` through source commit `{SYNC_016}`
([PR #9909](https://github.com/EffortlessMetrics/perl-lsp/pull/9909)) as a
**content-state mirror**. That promotion preserved the complete release tree but
did not make the individual swarm squash-merge commits ancestors of `v0.16.0`.
""",
        "0.16 provenance SHAs",
    )
    replace_once(
        path,
        """- the canonical development boundary is swarm merge-base `151c5ecee` through
  RC `a87f766ab`;
""",
        f"""- the canonical development boundary is swarm merge-base `{BASE_016}` through
  RC `{A87}`;
""",
        "0.16 development boundary SHAs",
    )
    replace_once(
        path,
        """## Validation performed

- **pr-fast 10/10 PASS** at swarm RC `a87f766ab`.
- **Product smoke:** 6 fixtures / 40 requests green at the RC.
- **Quality gate:** new RIPR+ gaps = 0 at the RC.
- **Publish dry-run package gate:** green on sync PR #9909, including
  `cargo check --locked` on the unpacked `perl-lsp-rs-core` crate.
""",
        f"""## Validation performed

- **RC gate and product-smoke evidence:** the
  [0.16 source-sync receipt](../swarm/source-syncs/2026-06-06-promote-swarm-a87f766ab.md#verification-commands-run)
  records the pre-sync checks at swarm RC `{A87}`.
- **Quality-gate evidence:** the cut's policy boundary is recorded in
  [`policy/quality-gate-exceptions.toml`](../../policy/quality-gate-exceptions.toml)
  and the source-sync receipt above. This note does not duplicate mutable gate
  counts.
- **Publish dry-run package evidence:**
  [sync PR #9909](https://github.com/EffortlessMetrics/perl-lsp/pull/9909)
  is the canonical review record, including `cargo check --locked` on the
  unpacked `perl-lsp-rs-core` crate.
""",
        "0.16 validation evidence links",
    )
    replace_once(
        path,
        """- Canonical development RC: `EffortlessMetrics/perl-lsp-swarm@a87f766ab`
- Logical development range: `151c5ecee...a87f766ab`
""",
        f"""- Canonical development RC: `EffortlessMetrics/perl-lsp-swarm@{A87}`
- Logical development ledger: `git log --first-parent --reverse {BASE_016}..{A87}`
""",
        "0.16 related provenance",
    )


def update_v017_note() -> None:
    path = Path("docs/releases/v0.17.0.md")
    replace_once(
        path,
        """The 0.17 release sync was a history-preserving, two-parent complete-tree merge
from `perl-lsp-swarm` into `perl-lsp` (PR #9941). The canonical development
boundary is:

- previous swarm release RC: `a87f766ab`;
- 0.17 swarm freeze: `c04e06b8c`;
- source sync merge: `55bef776f`.

The source comparison `v0.16.0...v0.17.0` is **not** a clean count of only new
0.17 logical work. The 0.17 merge also connected swarm commits that produced the
0.16 tree but were not ancestors of `v0.16.0`. Use the swarm first-parent range
`a87f766ab...c04e06b8c` for logical release accounting, and use the source tag
comparison for final-tree verification.
""",
        f"""The 0.17 release sync was a history-preserving, two-parent complete-tree merge
from `perl-lsp-swarm` into `perl-lsp`
([PR #9941](https://github.com/EffortlessMetrics/perl-lsp/pull/9941)). The
immutable merged boundary is:

- previous swarm release RC: `{A87}`;
- final 0.17 swarm freeze imported by the sync: `{FREEZE_017}`;
- source sync merge: `{SYNC_017}`.

The source comparison `v0.16.0...v0.17.0` is **inflated** for logical release
accounting. The 0.17 merge also connected swarm commits that produced the 0.16
tree but were not ancestors of `v0.16.0`. The merged sync commit is authoritative
over the earlier boundary named in the PR description. Enumerate the logical
0.17 ledger with the exact first-parent command:

```bash
git -C "${{SWARM_DIR:-../perl-lsp-swarm}}" log \\
  --first-parent \\
  --reverse \\
  --format='%H%x09%s' \\
  "{A87}..{FREEZE_017}"
```

Use the source tag comparison only for final-tree verification.
""",
        "0.17 release provenance boundary",
    )
    replace_once(
        path,
        """- **Source compare inflation.** `v0.16.0...v0.17.0` includes delayed ancestry
  imported from the 0.16 development cycle. Use `a87f766ab...c04e06b8c` for
  logical 0.17 change accounting.
""",
        """- **Source compare inflation.** `v0.16.0...v0.17.0` includes delayed ancestry
  imported from the 0.16 development cycle. Use the exact
  `git log --first-parent` command in [Release provenance](#release-provenance)
  for logical 0.17 change accounting.
""",
        "0.17 known-limitations range semantics",
    )
    replace_once(
        path,
        """- The source sync was a history-preserving complete-tree merge of swarm freeze
  `c04e06b8c` into `perl-lsp`.
""",
        f"""- The source sync was a history-preserving complete-tree merge of swarm freeze
  `{FREEZE_017}` into `perl-lsp`.
""",
        "0.17 validation freeze SHA",
    )
    replace_once(
        path,
        """- Canonical development freeze: `EffortlessMetrics/perl-lsp-swarm@c04e06b8c`
- Logical development range: `a87f766ab...c04e06b8c`
""",
        f"""- Canonical development freeze: `EffortlessMetrics/perl-lsp-swarm@{FREEZE_017}`
- Logical development ledger: [first-parent command](#release-provenance)
""",
        "0.17 related provenance",
    )


def main() -> None:
    update_release_checklist()
    update_authoring_guide()
    update_v016_note()
    update_v017_note()


if __name__ == "__main__":
    main()
