# Sublime Text Setup Guide for perl-lsp

Use this guide to run `perllsp` in Sublime Text through the Sublime Text `LSP` package.

## Prerequisites

### Required

- Sublime Text 4
- Package Control, unless installing packages manually
- Sublime Text `LSP` package
- `perllsp` installed and available on your `PATH`
- a Perl project opened as a Sublime project or folder

### Optional

- Perl, for running project code and system `@INC` probing
- `perltidy`, for formatting
- `perlcritic`, only if Perl::Critic diagnostics are enabled

Verify `perllsp` before changing Sublime settings:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Install the Sublime LSP Package

Install Package Control first:

1. Open the Command Palette with `Ctrl+Shift+P` or `Cmd+Shift+P`.
2. Run `Install Package Control`.

Then install Sublime's LSP client:

1. Open the Command Palette.
2. Run `Package Control: Install Package`.
3. Select `LSP`.

## Install `perllsp`

### Cargo

```bash
cargo install perllsp
```

### Homebrew

```bash
brew install perl-lsp
```

### From Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perllsp --locked
```

### Prebuilt Binary

Download the archive for your platform from GitHub Releases, extract it, and put the `perllsp` binary on your `PATH`.

Release assets use the `perllsp-<version>-<target>` naming pattern. Check the release page before copying a version number.

## Configure Sublime LSP

Open:

```text
Preferences > Package Settings > LSP > Server Configurations
```

or run this from the Command Palette:

```text
Preferences: LSP Server Configurations
```

Add:

```json
{
  "perl-lsp": {
    "enabled": true,
    "command": ["perllsp", "--stdio"],
    "selector": "source.perl",
    "initialization_options": {
      "perl": {
        "workspace": {
          "includePaths": ["lib", ".", "local/lib/perl5"],
          "useSystemInc": false,
          "resolutionTimeout": 50
        }
      }
    }
  }
}
```

Sublime LSP uses the key `initialization_options`. The LSP protocol field is named `initializationOptions`.

## Optional: Project-Specific Settings

Prefer `.perl-lsp.toml` for settings shared across all editors. Use a `.sublime-project` override only for Sublime-specific behavior.

```json
{
  "folders": [
    { "path": "." }
  ],
  "settings": {
    "LSP": {
      "perl-lsp": {
        "initialization_options": {
          "perl": {
            "workspace": {
              "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
            }
          }
        }
      }
    }
  }
}
```

## Optional: Inlay Hints

`perllsp` can provide inlay hints, but Sublime LSP does not display them by default. Enable display in:

```text
Preferences > Package Settings > LSP > Settings
```

```json
{
  "show_inlay_hints": true
}
```

Optional server-side override:

```json
{
  "perl-lsp": {
    "initialization_options": {
      "perl": {
        "inlayHints": {
          "enabled": true,
          "parameterHints": true,
          "typeHints": true,
          "maxLength": 30
        }
      }
    }
  }
}
```

## Optional: Logging

For client/server protocol logs, run:

```text
LSP: Toggle Log Panel
```

For extra LSP package debug output, add this to `Preferences: LSP Settings`:

```json
{
  "log_debug": true,
  "log_server": ["panel"]
}
```

For server-side `perllsp` logs, launch with `--log`:

```json
{
  "perl-lsp": {
    "enabled": true,
    "command": ["perllsp", "--stdio", "--log"],
    "selector": "source.perl"
  }
}
```

## Verify It Is Running

1. Open a Perl file such as `script/app.pl`, `lib/My/Module.pm`, or `t/basic.t`.
2. Confirm Sublime shows the syntax as Perl.
3. Confirm `perl-lsp` appears in the left side of the status bar.
4. Introduce a temporary syntax error.
5. Confirm diagnostics appear.
6. Remove the syntax error.

Useful commands:

```text
LSP: Troubleshoot Server
LSP: Toggle Log Panel
LSP: Restart Server
```

If the server does not start, run:

```text
Tools > Developer > Show Scope Name
```

The root scope should match the configured selector, usually `source.perl`.

## Recommended Keybindings

Many Sublime LSP commands are intentionally unbound by default. Add only the bindings you want in:

```text
Preferences: Key Bindings
```

```json
[
  {
    "keys": ["f12"],
    "command": "lsp_symbol_definition",
    "context": [
      { "key": "lsp.session_with_capability", "operator": "equal", "operand": "definitionProvider" },
      { "key": "selector", "operator": "equal", "operand": "source.perl" }
    ]
  },
  {
    "keys": ["shift+f12"],
    "command": "lsp_symbol_references",
    "context": [
      { "key": "lsp.session_with_capability", "operator": "equal", "operand": "referencesProvider" },
      { "key": "selector", "operator": "equal", "operand": "source.perl" }
    ]
  },
  {
    "keys": ["f2"],
    "command": "lsp_symbol_rename",
    "context": [
      { "key": "lsp.session_with_capability", "operator": "equal", "operand": "renameProvider" },
      { "key": "selector", "operator": "equal", "operand": "source.perl" }
    ]
  },
  {
    "keys": ["ctrl+."],
    "command": "lsp_code_actions",
    "context": [
      { "key": "lsp.session_with_capability", "operator": "equal", "operand": "codeActionProvider" },
      { "key": "selector", "operator": "equal", "operand": "source.perl" }
    ]
  },
  {
    "keys": ["ctrl+alt+m"],
    "command": "lsp_show_diagnostics_panel",
    "context": [
      { "key": "selector", "operator": "equal", "operand": "source.perl" }
    ]
  }
]
```

Other useful commands from the Command Palette:

```text
LSP: Format Document
LSP: Hover
LSP: Restart Server
LSP: Toggle Inlay Hints
```

## Troubleshooting

### Sublime cannot find `perllsp`

Check from a shell:

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

On Windows PowerShell:

```powershell
where perllsp
perllsp --version
perllsp --health
perllsp --info
```

Sublime Text may see a different `PATH` than your terminal. Use an absolute path in the server command if needed:

```json
{
  "perl-lsp": {
    "enabled": true,
    "command": ["/absolute/path/to/perllsp", "--stdio"],
    "selector": "source.perl"
  }
}
```

### Server does not start for Perl files

- Confirm the file syntax is Perl.
- Run `Tools > Developer > Show Scope Name`.
- Confirm the root scope matches `selector`, usually `source.perl`.
- Confirm `perl-lsp` is enabled globally or in the project.
- Run `LSP: Troubleshoot Server`.

### No diagnostics

- Confirm `perl-lsp` appears in the status bar.
- Run `LSP: Toggle Log Panel`.
- Run a manual check outside Sublime:

  ```bash
  perllsp --check path/to/file.pl
  ```

### Module resolution issues

Prefer `.perl-lsp.toml` for shared project include paths:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]
```

Or pass Sublime-specific initialization options:

```json
{
  "perl-lsp": {
    "initialization_options": {
      "perl": {
        "workspace": {
          "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
        }
      }
    }
  }
}
```

### Formatting does not work

Install `perltidy` and confirm it is available:

```bash
perltidy --version
```

Then run:

```text
LSP: Format Document
```

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for LSP input from the editor. Use these commands for manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

For server-side behavior and configuration details, see:

- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [Editor Setup](../how-to/EDITOR_SETUP.md)
