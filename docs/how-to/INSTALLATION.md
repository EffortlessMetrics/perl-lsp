# Installation Guide

Install `perl-lsp` and confirm it works.

## Fast Path

If you just want the shortest path to a working install:

```bash
cargo install perl-lsp
perl-lsp --version
perl-lsp --health
```

If those commands work, the binary is installed correctly and you can move on
to [EDITOR_SETUP.md](EDITOR_SETUP.md).

## Install From crates.io

```bash
cargo install perl-lsp
```

If you already have an older version installed:

```bash
cargo install perl-lsp --force
```

## Install From A Release Binary

Download the latest release from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases)
and install the binary for your platform.

### Linux / macOS

```bash
# Replace VERSION and ARCH with values from the releases page
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v${VERSION}/perl-lsp-${VERSION}-${ARCH}.tar.gz
tar xzf perl-lsp-${VERSION}-${ARCH}.tar.gz
sudo cp perl-lsp-${VERSION}-${ARCH}/perl-lsp /usr/local/bin/
chmod +x /usr/local/bin/perl-lsp
```

### Windows

```powershell
$VERSION = "<published-version>"
Invoke-WebRequest "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v$VERSION/perl-lsp-$VERSION-x86_64-pc-windows-msvc.zip" -OutFile perl-lsp.zip
Expand-Archive perl-lsp.zip -DestinationPath perl-lsp
Copy-Item perl-lsp\perl-lsp.exe "C:\Program Files\perl-lsp\"
```

### Windows Package Managers

If you prefer a package manager on Windows:

- Scoop: `scoop install perl-lsp`
- Chocolatey: `choco install perl-lsp`
- Winget: use the release-linked manifest once it has been published

## Build From Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo build --release --bin perl-lsp -p perl-lsp
```

After building from source, copy the binary onto your `PATH` or run it in
place from `target/release/`.

## Verify The Install

```bash
perl-lsp --version
perl-lsp --health
perl-lsp --info
```

Expected results:

- `--version` prints the installed version.
- `--health` prints `ok <version>`.
- `--info` prints version, build metadata, and feature information.

## Next Steps

- Set up your editor: [EDITOR_SETUP.md](EDITOR_SETUP.md)
- Fix a broken install or editor connection: [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- Review configuration options: [../reference/CONFIG.md](../reference/CONFIG.md)
