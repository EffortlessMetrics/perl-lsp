# Source Sync Receipt: 2026-07-15 workspace capability proof

## Sync identity

| Field | Value |
|---|---|
| Swarm repository | `perl-lsp-swarm` |
| Pinned swarm cut | `sync/vscode-workspace-capabilities-cut-bd3eb11b2` |
| Swarm cut SHA | `bd3eb11b221e18e9914c326326ee1515620bfae2` |
| Target repository | `perl-lsp` |
| Target base SHA | `2aefabe2d6c114864b5048fcb5d4c4302d3ddbf9` |
| Target sync merge SHA | `fc6acaf513675042744c5de1066abc597d3aa79f` |
| Merge parents | `2aefabe2d6c114864b5048fcb5d4c4302d3ddbf9`, `bd3eb11b221e18e9914c326326ee1515620bfae2` |
| Sync PR | [#10001](https://github.com/EffortlessMetrics/perl-lsp/pull/10001) |
| Target final merge SHA | `90d6fb5614841a621a5950e0f9b92044406320a8` |
| Final merge parents | `2aefabe2d6c114864b5048fcb5d4c4302d3ddbf9`, `8856d140985077f4710b0c7c5030b73ad9f15895` |
| Direction | swarm → target, history-preserving complete-tree merge |

## Modernization follow-up

The pinned cut contains the exact-source trusted multi-root and genuinely
untrusted workspace-host proof, normalized trust-mode handling, safe
workspace-claim validation, and server artifact fingerprint receipts added
after the prior modernization sync.

## Exclusions

The complete-tree difference from the pinned cut is limited to the approved
target-owned or swarm-only exclusions:

- `.claude/` restored from target `master`;
- `scripts/agent-cleanup.ps1` removed;
- `scripts/agent-preflight.ps1` removed;
- `scripts/swarm-clean` removed;
- target-owned sync ledgers retained under `docs/swarm/source-syncs/`.

No per-file resolution was used for shared source files. Release-lineage
documents remain governed by the target repository's sync protocol.

## Verification

The following checks passed against this sync tree before opening the target
sync PR:

```text
git log -1 --format='%p'                 # exactly two parents
git diff --name-only <swarm-cut>        # approved exclusions only
cargo check --workspace --locked
Node 26.5.0 / npm 11.18.0 doctor
npm ci
npm run fmt:check
npm run lint                             # Oxlint 0 errors / 0 warnings
npm run typecheck:all
npm run compile
npm run test:ci                           # 794 passed, 1 documented skip
npm run package
npm run check:package-inventory           # 28 files, 1,472,504 bytes
npm run check:source-map
npm run test:workspace-capabilities      # exact-source trusted multi-root/untrusted
```

Observed results:

- `cargo check --workspace --locked` passed.
- Node 26.5.0/npm 11.18.0 doctor, Oxfmt, Oxlint (0 errors / 0 warnings), all
  TS7 configurations, compile, test, package, inventory, and source-map checks
  passed after clean `npm ci`.
- Exact-source capability smoke passed in both modes on Windows with VS Code
  1.128.1: trusted multi-root (2 folders) and genuinely untrusted single-root
  (1 folder). Both receipts recorded initialize, provider requests, restart,
  and shutdown as successful.
- Target smoke source/server revision: `12bc6e16412dd73a10c109566b664bcae5b548f1`.
- Target smoke server artifact SHA-256:
  `2a08086218e81a52fdbd8c011972d416309e9cabd63fd7e323e5231cbe8b2074`.
- Target smoke VSIX SHA-256:
  `3df72e8e427fa690ac27b5b896a54098ea985cb3515c68b025d784f5f555bc9b`.

Post-sync verification passed: the pinned swarm cut is an ancestor of target
`master`, and `git diff --name-only <swarm-cut> origin/master` contains only
the approved `.claude/`, swarm-cleanup, and sync-ledger paths. Target hosted
CI had broad queued/advisory shards at merge time; no release claim is made
from those pending contexts. This receipt does not authorize publishing, tagging,
Marketplace or Open VSX upload, Docker publication, or release creation.
