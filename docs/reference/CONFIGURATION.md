# Configuration User Guide

**Practical guide to configuring perl-lsp for real projects.**

For the full technical reference, see [CONFIG.md](CONFIG.md). This guide focuses on copy-paste scenarios.

## Table of Contents

- [Quick Start](#quick-start)
- [Where to Put Your Config](#where-to-put-your-config)
- [All Settings at a Glance](#all-settings-at-a-glance)
- [Copy-Paste Scenarios](#copy-paste-scenarios)
  - [Basic CPAN-style project](#basic-cpan-style-project)
  - [Monorepo with multiple distributions](#monorepo-with-multiple-distributions)
  - [Custom Perl path](#custom-perl-path)
  - [Enable perlcritic linting](#enable-perlcritic-linting)
  - [Large codebase (10K+ files)](#large-codebase-10k-files)
  - [Low-resource or remote environment](#low-resource-or-remote-environment)
  - [CI / headless environment](#ci--headless-environment)
- [VSCode Settings Equivalents](#vscode-settings-equivalents)
- [Feature Flags](#feature-flags)
- [Troubleshooting Configuration](#troubleshooting-configuration)

---

## Quick Start

Copy this into your project root as `.perl-lsp.toml` and adjust as needed. Commit it so the whole team shares the same settings:

```toml
# .perl-lsp.toml — commit this to share settings with your team

[perl]
# Perl version hint (reserved for future use — safe to set now)
version = "5.38"

# Module search paths, relative to project root.
# Leave empty (or remove this line) to keep built-in defaults: lib, ., local/lib/perl5
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
# Uncomment to enable perlcritic (requires perlcritic installed)
# perlcritic = true
# perlcritic_severity = 3  # 1 = least severe (reports more), 5 = most severe (reports less)

[features]
# Inlay hints show parameter names and types inline while you code
inlay_hints = true
```

That is all most projects need. Everything else has sensible defaults.

---

## Where to Put Your Config

perl-lsp accepts configuration from three places, applied in order (later overrides earlier):

```
Priority 1 (lowest): .perl-lsp.toml     — project file, committed to version control
Priority 2:          initializationOptions — sent by your editor at startup
Priority 3 (highest): didChangeConfiguration — live editor settings
```

| File / mechanism | Who sets it | Scope |
|---|---|---|
| `.perl-lsp.toml` | Team (committed) | All editors, all team members |
| `settings.json` (VSCode) | Individual | That person's editor only |
| `init.lua` (Neovim) | Individual | That person's editor only |
| CLI flags | Launcher / CI | Server process only |

**Rule of thumb**: Anything the whole team should share goes in `.perl-lsp.toml`. Personal preferences go in your editor config.

---

## All Settings at a Glance

### `.perl-lsp.toml` keys

| Section | Key | Type | Default | Description |
|---|---|---|---|---|
| `[perl]` | `version` | string | none | Perl version hint, e.g. `"5.38"`. Reserved; not yet used. |
| `[perl]` | `include_paths` | string[] | `[]` | Extra module paths. Empty = keep built-in defaults. |
| `[diagnostics]` | `perlcritic` | bool | false | Enable perlcritic linting (opt-in). |
| `[diagnostics]` | `perlcritic_severity` | int 1-5 | 3 | Minimum severity to report. 1 = least severe (reports everything), 5 = most severe (reports only strictest). |
| `[features]` | `inlay_hints` | bool | true | Enable/disable all inlay hints globally. |

### LSP workspace settings (all editors, under `perl.*`)

| Key | Type | Default | Description |
|---|---|---|---|
| `perl.workspace.includePaths` | string[] | `["lib", ".", "local/lib/perl5"]` | Module search paths |
| `perl.workspace.useSystemInc` | bool | `false` | Include system `@INC` in resolution |
| `perl.workspace.resolutionTimeout` | number (ms) | `50` | Module resolution deadline |
| `perl.inlayHints.enabled` | bool | `true` | Master inlay hints switch |
| `perl.inlayHints.parameterHints` | bool | `true` | Show parameter names at call sites |
| `perl.inlayHints.typeHints` | bool | `true` | Show inferred variable types |
| `perl.inlayHints.chainedHints` | bool | `false` | Show method chain type annotations |
| `perl.inlayHints.maxLength` | number | `30` | Max characters before hint is truncated |
| `perl.testRunner.enabled` | bool | `true` | Enable integrated test runner |
| `perl.testRunner.command` | string | `"perl"` | Test executable (`"perl"` or `"prove"`) |
| `perl.testRunner.args` | string[] | `[]` | Extra args for the test command |
| `perl.testRunner.timeout` | number (ms) | `60000` | Test execution deadline |
| `perl.perlcritic.enabled` | bool | `false` | Enable perlcritic diagnostics |
| `perl.perlcritic.severity` | int 1-5 | `3` | Minimum severity to report |
| `perl.perlcritic.profile` | string | none | Path to `.perlcriticrc` profile |
| `perl.telemetry.enabled` | bool | `false` | Send telemetry events to client |
| `perl.limits.*` | various | see below | Resource caps and timeouts |

### `perl.limits` reference

| Key | Default | Description |
|---|---|---|
| `workspaceSymbolCap` | `200` | Max `workspace/symbol` results |
| `referencesCap` | `500` | Max `textDocument/references` results |
| `completionCap` | `100` | Max completion items |
| `documentSymbolCap` | `500` | Max `textDocument/documentSymbol` results |
| `codeLensCap` | `100` | Max code lens items per file |
| `diagnosticsPerFileCap` | `200` | Max diagnostics per file |
| `inlayHintsCap` | `500` | Max inlay hints per file |
| `astCacheMaxEntries` | `100` | AST cache size (LRU eviction) |
| `astCacheTtlSecs` | `300` | AST cache TTL in seconds |
| `symbolCacheMaxEntries` | `1000` | Symbol cache size |
| `maxIndexedFiles` | `10000` | Max files indexed for workspace features |
| `maxSymbolsPerFile` | `5000` | Max symbols indexed per file |
| `maxTotalSymbols` | `500000` | Max symbols across all indexed files |
| `maxFileSizeBytes` | `1048576` | Skip files larger than this (default: 1 MB) |
| `parseStormThreshold` | `10` | Pending parses before degradation mode |
| `workspaceScanDeadlineMs` | `30000` | Initial workspace scan budget (ms) |
| `referenceSearchDeadlineMs` | `2000` | Reference search budget (ms) |

---

## Copy-Paste Scenarios

### Basic CPAN-style project

The standard `ExtUtils::MakeMaker` or `Module::Build` layout: modules in `lib/`, tests in `t/`, dependencies in `local/`.

`.perl-lsp.toml`:

```toml
[perl]
version = "5.36"
include_paths = ["lib", "local/lib/perl5"]
```

`.vscode/settings.json`:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", "local/lib/perl5"]
    }
  }
}
```

---

### Monorepo with multiple distributions

You have several Perl distributions under one repository root, each with their own `lib/` directory:

```
my-monorepo/
  services/
    auth/
      lib/
      t/
    billing/
      lib/
      t/
  shared/
    lib/
```

Put `.perl-lsp.toml` at the repo root and list all the lib directories:

```toml
[perl]
include_paths = [
  "services/auth/lib",
  "services/billing/lib",
  "shared/lib",
  "local/lib/perl5"
]
```

Alternatively, open each sub-project in its own editor window — each will use the local `.perl-lsp.toml` in its own directory if you create one there.

VSCode workspace settings (`.vscode/settings.json` at repo root):

```json
{
  "perl": {
    "workspace": {
      "includePaths": [
        "services/auth/lib",
        "services/billing/lib",
        "shared/lib",
        "local/lib/perl5"
      ]
    }
  }
}
```

---

### Custom Perl path

You need to use a specific Perl binary (perlbrew, plenv, system Perl at a non-standard path) for running tests or the debugger.

The LSP server itself uses whichever `perl` is on your `PATH`. To use a custom Perl, set it in your shell before starting the editor, or configure it per-tool:

**Debugger (`launch.json`)** — set `perlPath` to the binary you want:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Debug with perlbrew Perl",
      "program": "${workspaceFolder}/script.pl",
      "perlPath": "/home/you/.perlbrew/perls/perl-5.38.0/bin/perl",
      "includePaths": ["${workspaceFolder}/lib"],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

**Test runner** — tell perl-lsp which binary to use for running tests:

```json
{
  "perl": {
    "testRunner": {
      "command": "/home/you/.perlbrew/perls/perl-5.38.0/bin/perl",
      "args": [],
      "timeout": 60000
    }
  }
}
```

**Shell approach** (recommended for the LSP server itself):

```bash
# In your shell profile, or before starting your editor:
eval "$(perlbrew env perl-5.38.0)"
code .
```

---

### Enable perlcritic linting

perlcritic integration is opt-in. It runs `perlcritic` on every open file and shows violations as diagnostics.

**Requirements**: `perlcritic` must be installed and on `$PATH`:

```bash
cpanm Perl::Critic
which perlcritic   # verify
```

**Enable via `.perl-lsp.toml`** (team-wide):

```toml
[diagnostics]
perlcritic = true
perlcritic_severity = 3   # 1 = least severe (reports more), 5 = most severe (reports less)
```

**Enable via editor settings** (personal preference):

```json
{
  "perl": {
    "perlcritic": {
      "enabled": true,
      "severity": 3
    }
  }
}
```

**Use a custom `.perlcriticrc` profile**:

```json
{
  "perl": {
    "perlcritic": {
      "enabled": true,
      "severity": 2,
      "profile": "${workspaceFolder}/.perlcriticrc"
    }
  }
}
```

When `profile` is not set, perlcritic auto-discovers `.perlcriticrc` in the workspace root (standard perlcritic behavior).

**Severity levels**:

| Severity | Name | What it catches |
|---|---|---|
| 1 | Brutal | Critical code smells only |
| 2 | Cruel | Serious issues |
| 3 (default) | Harsh | Common problems — good starting point |
| 4 | Stern | Style and best practices |
| 5 | Gentle | Everything, including minor style nits |

---

### Large codebase (10K+ files)

For monorepos or corporate codebases with tens of thousands of Perl files, increase the index limits and scan budget:

`.perl-lsp.toml`:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5"]

[features]
inlay_hints = true
```

Editor settings:

```json
{
  "perl": {
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 100
    },
    "limits": {
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "workspaceScanDeadlineMs": 120000,
      "workspaceSymbolCap": 300,
      "referencesCap": 1000
    }
  }
}
```

Tips for large codebases:
- Keep `useSystemInc: false` — system `@INC` queries block on network filesystems
- Increase `workspaceScanDeadlineMs` to give the initial index time to complete
- If the server feels slow on first open, it is indexing. Subsequent opens are fast.

---

### Low-resource or remote environment

Running on a VM, container, or remote SSH session with limited RAM or slow I/O:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib"],
      "useSystemInc": false,
      "resolutionTimeout": 25
    },
    "inlayHints": {
      "enabled": false
    },
    "limits": {
      "maxIndexedFiles": 3000,
      "maxTotalSymbols": 100000,
      "astCacheMaxEntries": 30,
      "workspaceSymbolCap": 100,
      "referencesCap": 200,
      "completionCap": 50,
      "workspaceScanDeadlineMs": 15000,
      "referenceSearchDeadlineMs": 1000
    }
  }
}
```

---

### CI / headless environment

Running `perllsp --check` in CI pipelines or pre-commit hooks:

```bash
# Check a single file
perllsp --check lib/MyModule.pm

# Check all Perl files in a directory
perllsp --check-project lib/

# Check with exit code (non-zero on parse errors)
perllsp --check-project . && echo "All files parse clean"
```

For a project that also uses perlcritic in CI, use the `perl.perlcritic` settings together with the test runner:

```json
{
  "perl": {
    "workspace": {
      "useSystemInc": false
    },
    "testRunner": {
      "enabled": true,
      "command": "prove",
      "args": ["-l", "-r", "--timer"],
      "timeout": 300000
    },
    "perlcritic": {
      "enabled": true,
      "severity": 3
    }
  }
}
```

---

## VSCode Settings Equivalents

Every `.perl-lsp.toml` setting has a VSCode `settings.json` counterpart. The table below maps between them.

| `.perl-lsp.toml` | `settings.json` (under `"perl"`) | Notes |
|---|---|---|
| `[perl] include_paths = [...]` | `"workspace": {"includePaths": [...]}` | TOML key is `include_paths`, LSP key is `includePaths` |
| `[perl] version = "5.38"` | — | No LSP equivalent yet; TOML only |
| `[diagnostics] perlcritic = true` | `"perlcritic": {"enabled": true}` | |
| `[diagnostics] perlcritic_severity = 3` | `"perlcritic": {"severity": 3}` | Note: LSP key is `severity`, not `perlcritic_severity` |
| `[features] inlay_hints = true` | `"inlayHints": {"enabled": true}` | TOML is global toggle; LSP has finer-grained control |

**Full VSCode `settings.json` with all settings:**

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"],
      "useSystemInc": false,
      "resolutionTimeout": 50
    },
    "inlayHints": {
      "enabled": true,
      "parameterHints": true,
      "typeHints": true,
      "chainedHints": false,
      "maxLength": 30
    },
    "testRunner": {
      "enabled": true,
      "command": "prove",
      "args": ["-l"],
      "timeout": 60000
    },
    "perlcritic": {
      "enabled": false,
      "severity": 3
    },
    "telemetry": {
      "enabled": false
    },
    "limits": {
      "workspaceSymbolCap": 200,
      "referencesCap": 500,
      "completionCap": 100,
      "maxIndexedFiles": 10000,
      "maxTotalSymbols": 500000,
      "workspaceScanDeadlineMs": 30000
    }
  },

  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.channel": "latest",
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.enableRefactoring": true,
  "perl-lsp.enableTestIntegration": true,
  "perl-lsp.includePaths": ["lib", "local/lib/perl5"]
}
```

---

## Feature Flags

Feature profiles control which LSP capabilities the server advertises. Select them at startup:

```bash
perllsp --stdio --feature-profile production  # default: full GA feature set
perllsp --stdio --feature-profile ga-lock     # conservative: GA-locked features only
perllsp --stdio --feature-profile all         # all features including experimental
```

In VSCode, set the profile in `settings.json`:

```json
{
  "perl-lsp.featureProfile": "production"
}
```

**When to use a non-default profile:**

- **`ga-lock`**: You need maximum stability and want to opt out of any features that are not fully GA. Good for production editor environments.
- **`all`**: You want to test experimental features. Expect rough edges.
- **`production`** (default): Use this unless you have a reason not to.

To see which features are active in the current profile:

```bash
perllsp --features-json --feature-profile production | python3 -m json.tool
```

---

## Troubleshooting Configuration

### Settings not taking effect

1. Check precedence: editor settings always override `.perl-lsp.toml`. If your TOML change is not showing up, check if your editor settings.json overrides it.
2. Restart the language server after changing settings. In VSCode: Command Palette > "Restart Extension Host".
3. Verify the TOML is valid:

   ```bash
   perllsp --check-project .  # will warn about bad .perl-lsp.toml
   ```

### Module resolution not finding your modules

1. Add the missing path to `include_paths` (TOML) or `includePaths` (editor).
2. Make sure the path is relative to the workspace root (the directory you opened in your editor).
3. Use `perl -I lib -e 'use My::Module; print 1'` to verify the path is actually correct.

### perlcritic shows no diagnostics

1. Confirm `perlcritic` is installed: `which perlcritic && perlcritic --version`
2. Confirm `perlcritic = true` is set (it is opt-in and defaults to false).
3. Check the severity — at severity 1, perlcritic reports the broadest set. Try severity 5 to restrict to only the most severe violations.

### Inlay hints are missing

1. Confirm your editor supports LSP inlay hints (VSCode 1.79+, Neovim 0.10+, Helix 24.x+).
2. Check that `inlayHints.enabled` is `true` (default).
3. In VSCode, confirm "Editor > Inlay Hints" is enabled in your preferences.

### Server is slow to start on a large project

This is expected on first open — the server is indexing your workspace. Subsequent opens are fast (the index is cached). If it is taking more than a few minutes, raise `workspaceScanDeadlineMs`:

```json
{
  "perl": {
    "limits": {
      "workspaceScanDeadlineMs": 120000
    }
  }
}
```

---

## See Also

- [CONFIG.md](CONFIG.md) — Complete technical configuration reference with all defaults
- [EDITOR_SETUP.md](../how-to/EDITOR_SETUP.md) — Editor-specific setup (Neovim, Emacs, Helix, Sublime)
- [PERFORMANCE_TUNING.md](../how-to/PERFORMANCE_TUNING.md) — Performance optimisation guide
- [CONFIGURATION_SCHEMA.md](CONFIGURATION_SCHEMA.md) — JSON Schema for machine validation
- [DAP_USER_GUIDE.md](../tutorials/DAP_USER_GUIDE.md) — Debugger (DAP) setup and `launch.json`
