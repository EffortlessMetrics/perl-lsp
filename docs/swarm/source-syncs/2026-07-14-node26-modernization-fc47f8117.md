# Source Sync Receipt: 2026-07-14 Node 26 modernization fc47f8117

## Sync identity

| Field | Value |
|---|---|
| Swarm repository | `EffortlessMetrics/perl-lsp-swarm` |
| Pinned swarm cut | `sync/node26-modernization-cut-fc47f8117` |
| Swarm cut SHA | `fc47f8117fa49bd45fde65be2f1cd8e1c625c78d` |
| Target repository | `EffortlessMetrics/perl-lsp` |
| Target base SHA | `ffee2824938f415e54923112c7b79e3f22040699` |
| Sync PR | [#9986](https://github.com/EffortlessMetrics/perl-lsp/pull/9986) |
| Target merge SHA | `c1e600d940a18035d3c32d81409bb33745f433fc` |
| Merge parents | `ffee2824938f415e54923112c7b79e3f22040699`, `0786ebed26735105f5e5353aa3357c6c10a090dc` |
| Direction | swarm → target, history-preserving complete-tree merge |

## Modernization trail

The pinned cut contains the reviewed modernization history, including the
Node 26/npm 11 authority correction (#4121), runtime receipt metadata (#4232,
#4247), closeout reconciliation (#4234, #4249), and the formatting fix (#4250).
The cut was created only after the source main proof and closeout were current.

## Exclusions

The complete-tree diff from the target merge to the pinned cut contains only
the documented target-owned or swarm-only exclusions:

- `.claude/` restored from target `master`;
- `scripts/agent-cleanup.ps1` removed;
- `scripts/agent-preflight.ps1` removed;
- `scripts/swarm-clean` removed.

No per-file resolution was used for shared source files. `RELEASE_HISTORY.md`
and release-lineage documents were preserved by the complete-tree merge.

## Verification

The following checks passed against the final target merge tree:

```text
git log -1 --format='%p'                 # exactly two parents
git diff --name-only HEAD <pinned-cut>   # 75 approved exclusion paths only
cargo check --workspace
Node v26.5.0 / npm 11.18.0 doctor
npm run fmt:check
npm run lint
npm run typecheck:all
npm run compile
npm run test:ci                           # 736 passed, 1 documented skip
npm run package
npm run check:package-inventory           # 28 files, 1,461,354 bytes
npm run test:published:local              # exact-source host smoke
```

The post-merge receipt records outcome `completed`, source and server SHA
`c1e600d940a18035d3c32d81409bb33745f433fc`, toolchain Node `v26.5.0`, npm
`11.18.0`, extension-host Node `v24.17.0`, VS Code `1.128.1`, and successful
restart and shutdown. The VSIX SHA-256 is recorded in the generated receipt.

## Claim boundary

This receipt proves the history-preserving sync ancestry, target workspace
compilation, Node 26 extension gate, package inventory, and exact-source local
editor-host path exercised above. It does not authorize publishing, tagging,
Marketplace or Open VSX upload, Docker publication, or release creation.
