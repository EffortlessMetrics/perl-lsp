# Installation Guide

Use this page when you need to install perl-lsp, upgrade an existing install,
or verify that the binary works on your machine.

If you only need editor integration after installation, jump to
[EDITOR_SETUP.md](EDITOR_SETUP.md). If the binary starts but does not behave as
expected, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## Fastest Path

```bash
cargo install perl-lsp
```

If you already have perl-lsp installed and want the current published build:

```bash
cargo install perl-lsp --force
```

Verify the install before wiring it into an editor:

```bash
perl-lsp --version
perl-lsp --health
perl-lsp --info
```

## Install From Source

Use this when you want to test the workspace locally or build a release binary
before publishing:

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo build --release --bin perl-lsp -p perl-lsp
```

If you want the binary installed into Cargo's bin directory instead:

```bash
cargo install --path crates/perl-lsp
```

## Prebuilt Releases

GitHub Releases provides downloadable archives for the supported platforms.
Check the latest release page before copying a version number.

| Platform | Asset suffix |
| --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` |
| Linux aarch64 | `aarch64-unknown-linux-gnu` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |

## Windows Package Managers

For Windows users, the release workflow also keeps the repo-owned package
manager manifests in sync with GitHub Releases.

- Scoop: `scoop install perl-lsp`
- Chocolatey: `choco install perl-lsp`
- Winget: the repo tracks a local manifest in `distribution/winget/`

Upstream package submission is still a separate manual step.

## After Installation

Once perl-lsp is installed, add it to your editor with the command:

```bash
perl-lsp --stdio
```

Then confirm the install from a shell before debugging editor integration:

```bash
perl-lsp --health
```

## Release Maintainers

If you are preparing a release, keep this page aligned with
[RELEASE.md](../../RELEASE.md) and
[project/PUBLISHING_ROADMAP.md](../project/PUBLISHING_ROADMAP.md). The release
workflow and final checks live there, not here.
