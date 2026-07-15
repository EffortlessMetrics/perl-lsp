# Source Sync Receipt: 2026-07-15 final modernization cut

## Sync identity

| Field | Value |
|---|---|
| Swarm repository | `perl-lsp-swarm` |
| Pinned swarm cut | `sync/vscode-modernization-final-cut-f6b7b2c66` |
| Swarm cut SHA | `f6b7b2c6626fbefbf01c9c9934cac5789186f8b2` |
| Target repository | `perl-lsp` |
| Target base SHA | `900c63f96507c273a562ab4623dceb1e0f39b843` |
| Target sync merge SHA | `9fab4c6703efb5ed7691aca670263a1827741b4c` |
| Merge parents | `900c63f96507c273a562ab4623dceb1e0f39b843`, `f6b7b2c6626fbefbf01c9c9934cac5789186f8b2` |
| Direction | swarm → target, history-preserving complete-tree merge |

## Included modernization work

This cut includes the merged workspace runtime-state extraction (#4384),
diagnostic command boundary and direct tests (#4389), and the reconciled
modernization closeout (#4392), along with the current mainline source at the
pinned cut. The cut preserves the individual swarm commit trail and does not
publish, tag, or release either repository.

## Exclusions

The complete-tree difference from the pinned cut is limited to the documented
target-owned or swarm-only exclusions:

- `.claude/` restored from target `master`;
- `scripts/agent-cleanup.ps1` removed;
- `scripts/agent-preflight.ps1` removed;
- `scripts/swarm-clean` removed;
- target-owned sync ledgers retained under `docs/swarm/source-syncs/`.

No per-file resolution was used for shared source files. Release-lineage
documents remain governed by the target repository's sync protocol.

## Verification

The following checks passed against the target sync tree:

```text
git log -1 --format='%p'                 # exactly two parents
git diff --name-only <swarm-cut>        # approved exclusions only
cargo check --workspace --locked
npm run doctor                           # Node 26.5.0 / npm 11.18.0
npm ci
npm run fmt:check
npm run lint                             # Oxlint 0 errors / 0 warnings
npm run typecheck:all
npm run compile
npm run test:ci                           # 797 passed, 1 documented skip
npm run package
npm run check:package-inventory           # 28 files, 1,474,277 bytes
npm run check:source-map
npm run test:workspace-capabilities      # exact-source trusted multi-root/untrusted
```

These checks establish the target-tree build, test, package, and workspace
capability contracts. They do not authorize Marketplace/Open VSX upload,
publishing, tagging, Docker publication, or release creation.
