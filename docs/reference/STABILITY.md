# Public API Stability Contract

**MSRV:** 1.92 • **Edition:** 2024 • **Status:** Public alpha (`0.12.x` line)

This document defines the API stability contract for published crates in this repository.

## Scope: what is a public API in this repo?

The **source of truth** is `[workspace.metadata.publish].allow` in `Cargo.toml`.

- Any crate in that allowlist is a **public SemVer contract**.
- Any crate not in that allowlist is **internal implementation detail**, even if `pub` within the workspace.

As of `0.12.4`, the publish allowlist includes **31 crates**.

## Current contract for the public-alpha line (`0.y.z`)

Because the project is pre-1.0, SemVer allows breaking changes in minor releases. We still apply stricter project rules:

### Patch releases (`0.y.Z`)

- Intended for bug fixes, security fixes, perf changes, and docs.
- Must not intentionally break documented public APIs.
- If an unavoidable break is discovered after release, it is treated as a release-process defect and documented in release notes.

### Minor releases (`0.Y.0`)

- May include breaking API changes.
- Breaking changes require explicit migration notes in release documentation.
- We strongly prefer additive evolution first (new APIs + deprecations) before removals.

## Required checks before merging public API changes

For any PR that changes exported items in a published crate:

1. Run `just public-api-check` (baseline diff guard).
2. Run `just semver-check-package <crate>` for each touched published crate when practical.
3. If the change is intentionally breaking, include a migration note and call it out as breaking in PR/release notes.

Repository helpers:

- `just public-api-check`
- `just public-api-update`
- `just semver-check`
- `just semver-check-package <crate>`

## Published crate inventory

To avoid stale hand-maintained lists, derive inventory from `Cargo.toml`:

```bash
python - <<'PY'
import tomllib
from pathlib import Path
cargo = tomllib.loads(Path('Cargo.toml').read_text())
allow = cargo['workspace']['metadata']['publish']['allow']
print(f"published crates: {len(allow)}")
for name in allow:
    print(name)
PY
```

## Non-goals for the alpha line

The following are **not** yet guaranteed until a post-alpha stability milestone:

- Multi-release deprecation windows for every removal
- Strict no-break-minor policy across all crates
- Full semver-check automation for every published crate on every PR

## Related docs

- [CONTRIBUTING.md](../../CONTRIBUTING.md)
- [../project/ROADMAP.md](../project/ROADMAP.md)
- [../project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md)
