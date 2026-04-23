# Installation Guide

Use this page when you need to install `perllsp`, upgrade an existing install,
or verify that the binary works on your machine.

If you only need editor integration after installation, jump to
[EDITOR_SETUP.md](EDITOR_SETUP.md). If the binary starts but does not behave as
expected, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

If you are wiring `perllsp` into a GitHub Actions workflow, see
[GitHub Actions Integration](GITHUB_ACTIONS.md).

## Fastest Path

Use one of the public install paths that matches how you work:

- VS Code: install the `EffortlessMetrics.perl-lsp-rs` extension and let it download the matching `perllsp` binary.
- macOS or Linux: install via Homebrew (see below).
- Other editors: download a prebuilt binary from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases) and put it on your `PATH`.
- Local testing or pre-release validation: install from this repo with `cargo install --path crates/perllsp`.

Do not use `cargo install perl-lsp` on crates.io. That package name is owned by another project, so the supported Cargo package is `perllsp`.

Verify the install before wiring it into an editor:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Homebrew (macOS and Linux)

Install the latest release with one command:

```bash
brew install perl-lsp
```

This covers macOS Intel, macOS Apple Silicon, Linux x86_64, and Linux aarch64 via Linuxbrew. The formula is automatically bumped on each release.

Shell completions are not installed by default. To add them:

```bash
perllsp --completion bash > "$(brew --prefix)/etc/bash_completion.d/perllsp"
perllsp --completion zsh > "$(brew --prefix)/share/zsh/site-functions/_perllsp"
perllsp --completion fish > "$(brew --prefix)/share/fish/completions/perllsp.fish"
```

## Install From Source

Use this when you want to test the workspace locally or build a release binary
before publishing:

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo build --release --bin perllsp -p perllsp
```

If you want the binary installed into Cargo's bin directory instead:

```bash
cargo install perllsp
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

## After Installation

Once `perllsp` is installed, add it to your editor with the command:

```bash
perllsp --stdio
```

Then confirm the install from a shell before debugging editor integration:

```bash
perllsp --health
```

## Release Maintainers

If you are preparing a release, keep this page aligned with
[RELEASE.md](../../RELEASE.md) and
[project/PUBLISHING_ROADMAP.md](../project/PUBLISHING_ROADMAP.md). The release
workflow and final checks live there, not here.
