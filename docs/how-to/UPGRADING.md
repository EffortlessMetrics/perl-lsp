# Upgrading perl-lsp

Use this guide when you already have `perl-lsp` installed and need to move to the current release line without breaking your editor, workspace config, or downstream crate builds.

The live release truth is:

- Workspace version: [`../../Cargo.toml`](../../Cargo.toml)
- Current project posture: [`../project/CURRENT_STATUS.md`](../project/CURRENT_STATUS.md)
- Release planning: [`../project/ROADMAP.md`](../project/ROADMAP.md)
- Change summary: [`../../CHANGELOG.md`](../../CHANGELOG.md)

## Fast paths

| Situation | Read this | What to do |
| --- | --- | --- |
| You installed the binary with Cargo | [Installation Guide](INSTALLATION.md) | Reinstall the current release with `cargo install --locked perllsp` |
| You launch through an editor | [Editor Setup](EDITOR_SETUP.md) | Update the server path, then restart the editor |
| You use a project config file | [Configuration Reference](../reference/CONFIG.md) | Reopen the workspace so new settings are picked up |
| You build against the crates | [Changelog](../../CHANGELOG.md) | Bump related crate versions together and rerun tests |
| Something broke after the upgrade | [Troubleshooting](TROUBLESHOOTING.md) | Check PATH, stale binaries, and config drift |

## 1. Reinstall the server

If you want the current published binary, reinstall it rather than trying to patch an old checkout in place.

```bash
cargo install --locked perllsp
cargo install --locked perl-dap
```

If you install from a local checkout, rebuild from the release line you want to run.

```bash
cargo install --locked --path crates/perllsp --force
cargo install --locked --path crates/perl-dap --force
```

If you use another package manager or a manual binary install, update that path first and then confirm `perllsp --version` reports the expected build.

## 2. Refresh editor wiring

Editors usually fail after an upgrade for one of three reasons: the executable path points at an older install, the editor cached a previous language-server process, or the project config changed.

- Re-check the language server path in the editor settings.
- Restart the editor after reinstalling the binary.
- If you use workspace settings, reopen the project root so the language server reindexes with the current config.
- If the editor still launches the wrong binary, remove stale PATH entries before trying again.

See:

- [Installation Guide](INSTALLATION.md)
- [Editor Setup](EDITOR_SETUP.md)
- [Troubleshooting](TROUBLESHOOTING.md)

## 3. Upgrade downstream crates

If your project depends on `perl-lsp` crates directly, upgrade the related crates together so the workspace version line stays aligned.

- Update the dependent crate versions to the current workspace release line.
- Regenerate your lockfile.
- Rebuild and rerun your tests.
- If compile errors mention removed or renamed APIs, check the matching release notes in `CHANGELOG.md` before changing code.

For release planning and current posture, use:

- [`CURRENT_STATUS.md`](../project/CURRENT_STATUS.md)
- [`ROADMAP.md`](../project/ROADMAP.md)

## 4. Verify the upgrade

After upgrading, confirm the binary and the editor are using the expected build.

```bash
perllsp --version
perllsp --health
```

Then open a small Perl file and confirm:

- the server starts cleanly
- diagnostics appear as expected
- completions and hover still work
- no stale path or config warnings show up in the editor log

## 5. If something still looks wrong

Check these first:

- `perllsp --version` shows the version you expected
- your editor points at the same binary you just installed
- PATH does not still prefer an older install
- workspace config changes were saved and the project was reopened
- the issue is not already covered in [Troubleshooting](TROUBLESHOOTING.md)

If you are jumping across more than one release, read the relevant `CHANGELOG.md` sections before you retry the upgrade.
