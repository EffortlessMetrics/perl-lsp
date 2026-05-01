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
- macOS or Linux: install via Homebrew, or use the installer script.
- Other editors: install `perllsp`, then configure your editor to run `perllsp --stdio`.
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

## Which file should I download?

Most users should not choose a release asset manually:

- VS Code / VSCodium / Cursor: install the Perl Language Server extension; it downloads the right server automatically.
- macOS or Linux with Homebrew: `brew install perl-lsp`
- Linux/macOS without Homebrew: use the installer script.

Manual downloads: choose exactly one archive for your operating system and CPU.

| Your system | Download suffix |
| --- | --- |
| Linux x64 / AMD64, most distributions | `x86_64-unknown-linux-gnu` |
| Linux ARM64, most distributions | `aarch64-unknown-linux-gnu` |
| Linux x64 / AMD64, Alpine or other musl systems | `x86_64-unknown-linux-musl` |
| Linux ARM64, Alpine or other musl systems | `aarch64-unknown-linux-musl` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| Windows x64 | `x86_64-pc-windows-msvc` |

For Linux, `gnu` means glibc, used by most distributions such as Ubuntu, Debian, Fedora, RHEL, Arch, and Amazon Linux. `musl` is mainly for Alpine Linux and musl-based containers.

You do not need both GNU and musl archives.

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
