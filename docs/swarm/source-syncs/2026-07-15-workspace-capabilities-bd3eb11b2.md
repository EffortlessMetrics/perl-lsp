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

The following checks are run against this sync tree before opening the target
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
npm run test:ci                           # current count recorded below
npm run package
npm run check:package-inventory
npm run check:source-map
npm run test:workspace-capabilities      # exact-source trusted multi-root/untrusted
```

The final target merge SHA and target-side receipt will be appended after the
sync PR merges. This receipt does not authorize publishing, tagging,
Marketplace or Open VSX upload, Docker publication, or release creation.
