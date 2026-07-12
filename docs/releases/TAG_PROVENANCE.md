# Release tag provenance

This document records the relationship between the repository's live release
tag refs and the immutable commit identifiers previously written into release
documentation.

The machine-readable source of truth is
[`policy/release-tag-provenance.toml`](../../policy/release-tag-provenance.toml).
It was audited against the live `EffortlessMetrics/perl-lsp` refs on
**2026-07-12**.

## Why this exists

Several historical ledger and release-note SHAs no longer resolve to the commit
currently reached by the named tag. Some originally recorded full SHAs no longer
resolve in the repository at all.

That observation does not establish why a ref changed, when it changed, or who
changed it. The archive therefore preserves both facts:

1. the identifier previously recorded in release documentation; and
2. the commit reached by the live tag at audit time.

A stale historical identifier is evidence. It is not an instruction to move the
live tag or silently rewrite the original ledger row.

## Audit result

Three different conditions exist.

### 1. Early tags are currently linear, despite stale recorded SHAs

The live tag chain from `v0.1.0-pest` through `v0.8.5` is forward-moving:

```text
v0.1.0-pest
  → v0.5.0
  → v0.7.2
  → v0.7.3
  → v0.8.0
  → v0.8.2
  → v0.8.3-rc1
  → v0.8.3
  → v0.8.5
```

Most ledger SHAs in that chain differ from the live refs, but the current tags
still form a coherent ancestry line.

### 2. `v0.9.1` is on a divergent historical line

`v0.8.5` is not an ancestor of the current `v0.9.1`, and `v0.9.1` is not an
ancestor of `v0.8.5`. The two refs share an older merge base and then diverge.
Therefore:

- `v0.8.5...v0.9.1` is not a forward release comparison;
- no current earlier release tag is asserted as the linear predecessor of
  `v0.9.1`;
- the later release line resumes with `v0.9.1 → v0.11.0`.

The `0.10.0` entry between them is a changelog-only milestone, not a tag.

### 3. The later line is forward-moving, with two missing tag boundaries

The current line from `v0.9.1` through `v0.17.0` is forward-moving, subject to
the already-documented release accounting caveats:

- no final `v0.13.0` tag exists; the line moves from `v0.13.0-rc1` to
  `v0.13.1`;
- no `v0.13.4` tag exists; its prepared source state first appears in the
  `v0.14.0` tag;
- `v0.16.0` has a correct source-tree parent chain but omitted the logical swarm
  squash ancestry at its tag boundary;
- `v0.17.0` imports that delayed swarm ancestry, so its source comparison is
  inflated for logical release accounting.

## Audited tag table

Short SHAs below are display abbreviations. The manifest contains full 40-byte
commit identifiers.

| Tag | Live ref | Prior record | Record status | Current predecessor | Lineage |
|---|---:|---:|---|---|---|
| `v0.1.0-pest` | `4f92dc57` | `4f92dc57` | match | — | root |
| `v0.5.0` | `8ed75836` | `60190640` | stale / unreachable | `v0.1.0-pest` | linear |
| `v0.7.2` | `79d372e9` | `a19ba90b` | stale / unreachable | `v0.5.0` | linear |
| `v0.7.3` | `4149fee1` | `20751374` | stale / unreachable | `v0.7.2` | linear |
| `v0.8.0` | `a9534827` | `2eeb06c5` | stale / unreachable | `v0.7.3` | linear |
| `v0.8.2` | `9d7584d0` | `0b962684` | stale / unreachable | `v0.8.0` | linear |
| `v0.8.3-rc1` | `150a22b1` | `150a22b1` | match | `v0.8.2` | linear |
| `v0.8.3` | `274005de` | `5331007a…` | stale / unreachable | `v0.8.3-rc1` | linear |
| `v0.8.5` | `a6ca88fb` | `ae75da03…` | stale / unreachable | `v0.8.3` | linear |
| `v0.9.1` | `0e52877d` | `c82a1604…` | stale / unreachable | `v0.8.5` | **diverged** |
| `v0.11.0` | `8dfa6886` | `d22ac734…` | stale / unreachable | `v0.9.1` | linear |
| `v0.12.0` | `68613d83` | `4c909c2d…` | stale / unreachable | `v0.11.0` | linear |
| `v0.12.1` | `5ee16c2c` | `7e8984b5…` | stale / unreachable | `v0.12.0` | linear |
| `v0.12.2` | `6e4e6223` | `1c0620d8…` | stale / unreachable | `v0.12.1` | linear |
| `v0.12.3` | `cc801735` | `a86af221…` | stale / unreachable | `v0.12.2` | linear |
| `v0.12.4` | `181d2b2d` | `5ebb37aa…` | stale / unreachable | `v0.12.3` | linear |
| `v0.13.0-rc1` | `4e4099cd` | not previously ledgered | unrecorded | `v0.12.4` | linear |
| `v0.13.1` | `6ef20484` | `6ef20484` | match | `v0.13.0-rc1` | linear |
| `v0.13.2` | `0e9c5d78` | `0e9c5d78` | match | `v0.13.1` | linear |
| `v0.13.3` | `06fc1443` | `06fc1443` | match | `v0.13.2` | linear |
| `v0.14.0` | `977709e0` | `82e64200` | stale / unreachable | `v0.13.3` | linear |
| `v0.15.0` | `ac8e281e` | pending | newly pinned | `v0.14.0` | linear |
| `v0.15.1` | `15cbe7e6` | `15cbe7e6` | match | `v0.15.0` | linear |
| `v0.15.2` | `746edcb7` | `746edcb7` | match | `v0.15.1` | linear |
| `v0.16.0` | `b6d9f12b` | pending | newly pinned | `v0.15.2` | linear source chain |
| `v0.17.0` | `ffee2824` | pending | newly pinned | `v0.16.0` | linear source chain |

## Verification

Validate the manifest without touching Git or the network:

```bash
python3 scripts/check_release_tag_provenance.py
```

In a full-history checkout with tags fetched, verify live refs and ancestry:

```bash
git fetch --force --tags origin
python3 scripts/check_release_tag_provenance.py --verify-git
```

The second command fails when:

- a live tag no longer resolves to its pinned `current_sha`;
- a tag classified as `linear` does not descend from its predecessor;
- a pair classified as `diverged` becomes ancestral in either direction.

It deliberately does not contact GitHub. The checkout and fetched refs are the
operator-controlled input.

## Release control going forward

For every new tag:

1. create the tag only after release-tree and cross-repository ancestry checks
   pass;
2. resolve the exact commit with `git rev-parse vX.Y.Z^{commit}`;
3. add the tag and SHA to `policy/release-tag-provenance.toml` in release
   closeout;
4. record the predecessor and expected lineage;
5. run the validator with `--verify-git` from a full-history checkout;
6. publish the resulting manifest change before treating release provenance as
   closed.

Release tags are expected to be immutable. If an exceptional correction is ever
required, preserve the prior SHA, add a dated incident/correction note, and
update the manifest explicitly. Do not replace the old record without an audit
trail.

## Relationship to release notes and Changie

This manifest answers **which commit a tag reaches and how tags relate**.

It does not replace:

- Changie fragments, which capture notable change intent before release;
- the logical `perl-lsp-swarm` first-parent range, which is the primary
  cross-repository release ledger;
- final release-tree verification;
- channel publication closeout.

All four controls are needed. A correct tag SHA cannot prove that release notes
are complete, and complete release notes cannot prove that a tag stayed fixed.
