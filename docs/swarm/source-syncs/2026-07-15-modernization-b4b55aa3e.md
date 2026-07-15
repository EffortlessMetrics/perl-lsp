# Source Sync Receipt: 2026-07-15 VS Code modernization b4b55aa3e

## Sync identity

| Field | Value |
|---|---|
| Swarm repository | `perl-lsp-swarm` |
| Pinned swarm cut | `sync/vscode-modernization-cut-b4b55aa3e` |
| Swarm cut SHA | `b4b55aa3eb66bb836ec9da2c14bdb70215f3f303` |
| Target repository | `perl-lsp` |
| Target base SHA | `c1f3747dcc5509adaf8021ba4ce5a1ba458d909e` |
| Sync branch | `release/sync-modernization-b4b55aa3e` |
| Sync PR | pending |
| Target merge SHA | `9cb93de001f1c7c3dfdb43d5e47a14567b8c01b5` |
| Merge parents | `c1f3747dcc5509adaf8021ba4ce5a1ba458d909e`, `b4b55aa3eb66bb836ec9da2c14bdb70215f3f303` |
| Direction | swarm → target, history-preserving complete-tree merge |

## Modernization trail

The pinned cut contains the reviewed Node 26/npm 11 authority correction,
runtime receipt metadata, strictness and Oxlint ratchets, exact-source host
proof, package/source-map contracts, and the command-composition hardening
through PR #4368. The closeout reconciliation is recorded in PR #4370 and
`vscode-extension/docs/migrations/lane-closeout.md`.

## Exclusions

The complete-tree difference from the pinned cut contains only the documented
target-owned or swarm-only exclusions:

- `.claude/` restored from target `master`;
- `scripts/agent-cleanup.ps1` removed;
- `scripts/agent-preflight.ps1` removed;
- `scripts/swarm-clean` removed.

No per-file resolution was used for shared source files. Release-lineage
documents remain governed by the target repository's sync protocol.

## Verification

The following checks passed against the sync merge tree:

```text
git log -1 --format='%p'                 # exactly two parents
git diff --name-only HEAD <pinned-cut>   # documented exclusions only
cargo check --workspace --locked
Node v26.5.0 / npm 11.18.0 doctor
npm ci
npm run fmt:check
npm run lint                             # Oxlint 0 errors / 0 warnings
npm run typecheck:all
npm run compile
npm run test:ci                           # 791 passed, 1 documented skip
npm run package
npm run check:package-inventory           # 28 files, 1,472,417 bytes
npm run check:source-map
npm run test:published:local              # exact-source host smoke
```

The target smoke receipt records outcome `completed`, source and server SHA
`9cb93de001f1c7c3dfdb43d5e47a14567b8c01b5`, toolchain Node `v26.5.0`, npm
`11.18.0`, extension-host Node `v24.17.0`, VS Code `1.128.1`, successful
restart and shutdown, and VSIX SHA-256
`f03080904d3afcd38ccbb9ebfd3de0435d73f3c0e94c95b5b7396281bd285ed9`.

## Claim boundary

This receipt proves the pinned history-preserving sync ancestry, target
workspace compilation, Node 26 extension gate, package inventory, source-map
contract, and exact-source local editor-host path exercised above. It does not
authorize publishing, tagging, Marketplace or Open VSX upload, Docker
publication, or release creation.
