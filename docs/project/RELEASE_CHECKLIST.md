# Release Checklist

This checklist is the release gate for the current cut.
The mechanics live in [RELEASE.md](../../RELEASE.md), the release-day sequence lives in
[PUBLISHING_ROADMAP.md](PUBLISHING_ROADMAP.md), and changelog generation lives in
[CHANGELOG_WORKFLOW.md](../CHANGELOG_WORKFLOW.md).

Use `NEW_VERSION` as the target semver string for the release you are preparing.

## Preflight

- [ ] The working tree is clean on the current default branch.
- [ ] `just release-check` passes.
- [ ] `just status-check` passes if any files under `docs/project/status/` changed.
- [ ] `gh secret list` shows `CARGO_REGISTRY_TOKEN`, `VSCE_PAT`, `OVSX_PAT`, `DOCKER_USERNAME`, and `DOCKER_PASSWORD`.
- [ ] `just version-check` passes.
- [ ] `CHANGELOG.md` contains a dated `## [NEW_VERSION]` section and leaves `[Unreleased]` empty.
- [ ] The crates listed in `[workspace.metadata.publish.allow]` report `NEW_VERSION`.
- [ ] `cargo xtask install-surface-check` passes.
- [ ] `cargo xtask release-notes --tag vNEW_VERSION --output /tmp/vNEW_VERSION-body.md` produces release notes with the Linux asset chooser:

  ```bash
  cargo xtask release-notes --tag vNEW_VERSION --output /tmp/vNEW_VERSION-body.md
  grep -q 'Which file should I download?' /tmp/vNEW_VERSION-body.md
  grep -q 'x86_64-unknown-linux-gnu' /tmp/vNEW_VERSION-body.md
  grep -q 'x86_64-unknown-linux-musl' /tmp/vNEW_VERSION-body.md
  ```

- [ ] `gh run list --branch master --limit 5` shows the default branch is green.
- [ ] No stale `.snap.new` files remain in the worktree.

Use this version check when you need to confirm the release target:

```bash
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
workspace_members = set(meta["workspace_members"])
packages = {pkg["name"]: pkg for pkg in meta["packages"] if pkg["id"] in workspace_members}
allow = meta.get("metadata", {}).get("publish", {}).get("allow", [])
target = "NEW_VERSION"
for crate_name in allow:
    pkg = packages.get(crate_name)
    if pkg is None:
        print(f"UNKNOWN: {crate_name}")
        continue
    if pkg["version"] != target:
        print(f"MISMATCH: {pkg[\"name\"]}@{pkg[\"version\"]}")
'
```

## Release Execution

- [ ] Prepare the version bump and changelog with `cargo xtask release-turnkey NEW_VERSION` or the `version-bump.yml` workflow.
- [ ] Review and merge the generated version-bump PR.
- [ ] Dispatch `release-orchestration.yml` with `version=NEW_VERSION`, `prerelease=false`, `skip_crates=false`, `skip_extension=false`, and `skip_docker=false`.
- [ ] If a downstream publish stage fails, re-run orchestration with the relevant `skip_*` flags instead of tagging manually.
- [ ] Let `release.yml`, `publish-crates.yml`, `publish-extension.yml`, and `docker-publish.yml` finish.
- [ ] Confirm the release-published triggers for `brew-bump.yml`, `scoop-bump.yml`, and `chocolatey-bump.yml` fired as expected.
- [ ] After `brew-bump.yml` finishes, confirm the owned tap is updated or reported already current.

## Post-Release Verification

Use [`docs/releases/RELEASE_CLOSEOUT_AUDIT.md`](../releases/RELEASE_CLOSEOUT_AUDIT.md) as the canonical re-runnable checklist. The items below are the minimum subset; the audit doc covers each channel in detail (including dispatch-only workflows that don't fire on `release:published` and require manual `gh workflow run`).

- [ ] `gh release view vNEW_VERSION` shows the expected release notes and assets.
- [ ] `cargo search perl-lsp-rs --limit 1` resolves `perl-lsp-rs = "NEW_VERSION"`.
- [ ] `cargo search perllsp --limit 1` resolves `perllsp = "NEW_VERSION"`.
- [ ] The VS Code Marketplace and Open VSX listings show `NEW_VERSION`.
- [ ] `docker pull effortlessmetrics/perl-lsp:NEW_VERSION` and `docker pull ghcr.io/effortlessmetrics/perl-lsp:NEW_VERSION` succeed.
- [ ] `brew update`, `brew upgrade perllsp`, `perllsp --version`, and `perl-dap --version` show `NEW_VERSION`.
- [ ] `cargo install perllsp` installs the new release and `perllsp --version` prints `NEW_VERSION`.
- [ ] The smoke tests in [RELEASE.md](../../RELEASE.md) pass for the current release artifacts.
- [ ] Any evidence-backed status docs are updated with `just status-update` and validated with `just status-check`.
- [ ] **Closeout audit:** run the per-channel verification block in [`RELEASE_CLOSEOUT_AUDIT.md`](../releases/RELEASE_CLOSEOUT_AUDIT.md) and write the populated instance to `docs/releases/NEW_VERSION-closeout-audit.md`. Flip `notes_status` in `docs/releases/vNEW_VERSION.md` from `pending` to `closed` only when every channel resolves.

## Repo-Native Cleanup Notes

- Manual tag creation, `git checkout -B master origin/master`, and direct `git push origin vNEW_VERSION` are historical instructions from the issue context, not the current release path.
- If you need the operational release procedure, follow [RELEASE.md](../../RELEASE.md) instead of duplicating the workflow in an issue comment.
