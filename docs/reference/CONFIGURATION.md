# Configuration Reference

**Source of truth for all configurable options in perl-lsp.**

This document consolidates every setting available in the Perl Language Server,
organized by where the setting is expressed: LSP workspace settings, CLI flags,
environment variables, VS Code extension properties, and DAP launch/attach
configuration.

## Table of Contents

- [How Configuration Works](#how-configuration-works)
- [Workspace Settings (LSP)](#workspace-settings-lsp)
  - [perl.workspace](#perlworkspace)
  - [perl.inlayHints](#perlinlayhints)
  - [perl.testRunner](#perltestrunner)
  - [perl.perlcritic](#perlperlcritic)
  - [perl.telemetry](#perltelemetry)
  - [perl.limits](#perllimits)
- [CLI Flags](#cli-flags)
- [Environment Variables](#environment-variables)
- [VS Code Extension Settings](#vs-code-extension-settings)
- [DAP Debug Configuration](#dap-debug-configuration)
  - [Launch Configuration](#launch-configuration)
  - [Attach Configuration](#attach-configuration)
- [Feature Profiles](#feature-profiles)
- [Example Configurations](#example-configurations)

---

## How Configuration Works

Settings reach the server through four independent channels:

1. **`initializationOptions`** — Passed in the LSP `initialize` request. Applied once at startup.
2. **`workspace/didChangeConfiguration`** — Sent by the editor whenever settings change. Applied incrementally; unspecified keys keep their current value.
3. **CLI flags** — Passed on the command line when launching the binary.
4. **Environment variables** — Set in the shell before starting the server.

All LSP workspace settings live under the `perl` namespace:

```json
{
  "perl": {
    "workspace": { "includePaths": ["lib"] },
    "inlayHints": { "enabled": true },
    "testRunner": { "command": "prove" },
    "perlcritic": { "enabled": false },
    "telemetry": { "enabled": false },
    "limits": { "completionCap": 100 }
  }
}
```

---

## Workspace Settings (LSP)

These settings are read from the LSP client via `initializationOptions` or
`workspace/didChangeConfiguration`. Source: `crates/perl-lsp-config/src/lib.rs`.

### perl.workspace

Controls module resolution and workspace scanning behaviour.

#### `perl.workspace.includePaths`

| Property | Value |
|---|---|
| Type | `string[]` |
| Default | `["lib", ".", "local/lib/perl5"]` |
| Key | `includePaths` |

Directories to search for Perl modules, relative to the workspace root. Appended
to the internal `@INC` used for go-to-definition and hover documentation.

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
    }
  }
}
```

#### `perl.workspace.useSystemInc`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `false` |
| Key | `useSystemInc` |

Include the system `@INC` paths (queried from `perl -e 'print join("\n", @INC)'`)
in module resolution. Disabled by default to avoid blocking on network
filesystems. The current directory `.` is always filtered out of the system
`@INC` for security.

Changing this value at runtime clears the internal `@INC` cache.

```json
{ "perl": { "workspace": { "useSystemInc": true } } }
```

#### `perl.workspace.resolutionTimeout`

| Property | Value |
|---|---|
| Type | `number` (milliseconds) |
| Default | `50` |
| Key | `resolutionTimeout` |

Maximum time the server will spend resolving a single module path. Prevents UI
stalls on slow or network-mounted filesystems.

```json
{ "perl": { "workspace": { "resolutionTimeout": 100 } } }
```

---

### perl.inlayHints

Controls inlay hints displayed inline in the editor.

#### `perl.inlayHints.enabled`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `true` |

Master switch for all inlay hints. Setting this to `false` suppresses all hint
types regardless of the individual settings below.

#### `perl.inlayHints.parameterHints`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `true` |

Show parameter name hints at function call sites.

```perl
# With parameterHints enabled:
some_function(/* name: */ "value", /* count: */ 42);
```

#### `perl.inlayHints.typeHints`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `true` |

Show inferred type annotations for `my` variables.

#### `perl.inlayHints.chainedHints`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `false` |

Show intermediate type annotations on chained method calls.

#### `perl.inlayHints.maxLength`

| Property | Value |
|---|---|
| Type | `number` |
| Default | `30` |

Maximum character length for a single hint label before it is truncated.

```json
{
  "perl": {
    "inlayHints": {
      "enabled": true,
      "parameterHints": true,
      "typeHints": true,
      "chainedHints": false,
      "maxLength": 30
    }
  }
}
```

---

### perl.testRunner

Configuration for the integrated test runner (Test::More, Test2, prove).

#### `perl.testRunner.enabled`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `true` |

Enable the integrated test runner. When `false`, test-related code lenses and
commands are suppressed.

#### `perl.testRunner.command`

| Property | Value |
|---|---|
| Type | `string` |
| Default | `"perl"` |

Executable used to run tests. Common values: `"perl"`, `"prove"`.

#### `perl.testRunner.args`

| Property | Value |
|---|---|
| Type | `string[]` |
| Default | `[]` |

Additional arguments passed to the test command.

#### `perl.testRunner.timeout`

| Property | Value |
|---|---|
| Type | `number` (milliseconds) |
| Default | `60000` |

Maximum time to wait for a test run before the server considers it timed out.

```json
{
  "perl": {
    "testRunner": {
      "enabled": true,
      "command": "prove",
      "args": ["-l", "-v"],
      "timeout": 120000
    }
  }
}
```

---

### perl.perlcritic

Controls optional Perl::Critic static analysis integration.

#### `perl.perlcritic.enabled`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `false` |

**Opt-in.** When `true`, the server runs `perlcritic` on open documents and
merges violations into the diagnostic stream. Silently skipped if `perlcritic`
is not installed on the system.

```json
{ "perl": { "perlcritic": { "enabled": true } } }
```

---

### perl.telemetry

#### `perl.telemetry.enabled`

| Property | Value |
|---|---|
| Type | `boolean` |
| Default | `false` |

Enable server-side telemetry events (`telemetry/event` notifications to the
client). Off by default; no data leaves the machine — this only controls
whether the client receives telemetry payloads from the server.

---

### perl.limits

Resource caps and deadline settings. Increase values for large workspaces;
decrease them for resource-constrained environments.

#### Result caps

| Key | Default | Description |
|---|---|---|
| `workspaceSymbolCap` | `200` | Maximum results from `workspace/symbol` |
| `referencesCap` | `500` | Maximum results from `textDocument/references` |
| `completionCap` | `100` | Maximum completion items returned |
| `documentSymbolCap` | `500` | Maximum results from `textDocument/documentSymbol` |
| `codeLensCap` | `100` | Maximum code lens items per file |
| `diagnosticsPerFileCap` | `200` | Maximum diagnostics per file |
| `inlayHintsCap` | `500` | Maximum inlay hints per file |

#### Cache settings

| Key | Default | Description |
|---|---|---|
| `astCacheMaxEntries` | `100` | AST cache size (LRU eviction) |
| `astCacheTtlSecs` | `300` | AST cache TTL in seconds |
| `symbolCacheMaxEntries` | `1000` | Symbol cache size |

#### Index limits

| Key | Default | Description |
|---|---|---|
| `maxIndexedFiles` | `10000` | Maximum files indexed for workspace features |
| `maxSymbolsPerFile` | `5000` | Maximum symbols indexed per file |
| `maxTotalSymbols` | `500000` | Maximum total symbols across all indexed files |
| `parseStormThreshold` | `10` | Pending parse count before degradation |

#### Deadline settings (milliseconds)

| Key | Default | Description |
|---|---|---|
| `workspaceScanDeadlineMs` | `30000` | Initial workspace folder scan budget |
| `fileIndexDeadlineMs` | `5000` | Single file indexing budget |
| `referenceSearchDeadlineMs` | `2000` | Reference search budget |
| `regexScanDeadlineMs` | `1000` | Regex scan budget |
| `fsOperationDeadlineMs` | `500` | Filesystem operation budget |

```json
{
  "perl": {
    "limits": {
      "workspaceSymbolCap": 300,
      "referencesCap": 1000,
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "workspaceScanDeadlineMs": 60000
    }
  }
}
```

---

## CLI Flags

Flags passed when launching the `perl-lsp` binary. Source:
`crates/perl-lsp-launcher/src/lib.rs`.

### Server mode

| Flag | Description |
|---|---|
| `--stdio` | Use stdio transport (default) |
| `--socket` | Use TCP socket transport |
| `--port <n>` | TCP port to listen on (default: `9257`; implies `--socket`) |
| `--log` | Enable logging to stderr |
| `--feature-profile <name>` | Select feature profile (see [Feature Profiles](#feature-profiles)) |

### Diagnostic and info

| Flag | Description |
|---|---|
| `--health` | Print `ok <version>` and exit |
| `--info` | Print version, parser, profile, feature count, and executable path |
| `--version` | Print version string and exit |
| `--features-json` | Print the active feature catalog as JSON and exit |

### Tool mode (no editor required)

| Flag | Description |
|---|---|
| `--check <files...>` | Validate Perl files and report parse errors to stdout |
| `--check-project [dir]` | Scan a project directory and print parsability summary (defaults to `.`) |
| `--completion <shell>` | Print shell completion script (`bash`, `zsh`, `fish`, `powershell`) |

Examples:

```bash
perl-lsp --stdio                         # stdio mode (default)
perl-lsp --stdio --log                   # with logging to stderr
perl-lsp --socket --port 9257            # TCP socket mode
perl-lsp --stdio --feature-profile prod  # production feature profile
perl-lsp --check lib/MyModule.pm         # batch syntax check
perl-lsp --check-project lib/            # project-wide parsability scan
perl-lsp --info                          # print server information
perl-lsp --completion bash >> ~/.bashrc  # install bash completions
```

---

## Environment Variables

Environment variables read at startup by the `perl-lsp` binary. Source:
`crates/perl-lsp-launcher/src/lib.rs`.

### `PERL_LSP_LOG`

Set to any non-empty value to enable logging to stderr. Equivalent to the
`--log` flag. When both are present, environment wins over the flag default but
either enables logging.

```bash
PERL_LSP_LOG=1 perl-lsp --stdio
```

### `RUST_LOG`

Standard `tracing`/`env_logger` filter directive. Controls log level and
per-module filtering. Takes precedence over the `--log` flag default filter.

```bash
RUST_LOG=perl_lsp=debug perl-lsp --stdio
RUST_LOG=perl_parser=trace perl-lsp --stdio
RUST_LOG=warn perl-lsp --stdio
```

Common filter tokens:

| Token | Effect |
|---|---|
| `error` | Errors only |
| `warn` | Warnings and errors |
| `info` | Info, warnings, errors (typical) |
| `debug` | Debug output |
| `trace` | Maximum verbosity |
| `perl_lsp=debug` | Debug for the LSP crate only |

### `NO_COLOR`

When set, disables ANSI colour in log output. Follows the
[no-color.org](https://no-color.org) convention.

```bash
NO_COLOR=1 perl-lsp --stdio
```

---

## VS Code Extension Settings

Settings specific to the VS Code extension (`vscode-extension/package.json`).
These are separate from the LSP workspace settings above and control extension
behaviour such as binary management and feature toggles.

### Binary management

| Setting | Type | Default | Description |
|---|---|---|---|
| `perl-lsp.serverPath` | `string` | `""` | Absolute path to the `perl-lsp` binary. Empty = auto-download. |
| `perl-lsp.autoDownload` | `boolean` | `true` | Download the binary automatically if not found locally. |
| `perl-lsp.downloadBaseUrl` | `string` | `""` | Override the GitHub releases base URL for internal mirrors. |
| `perl-lsp.channel` | `"latest"\|"stable"\|"tag"` | `"latest"` | Release channel to track. |
| `perl-lsp.versionTag` | `string` | `""` | Specific release tag (e.g., `v0.8.3`) when `channel` is `"tag"`. |

### Debugging

| Setting | Type | Default | Description |
|---|---|---|---|
| `perl-lsp.trace.server` | `"off"\|"messages"\|"verbose"` | `"off"` | Log LSP message traffic for diagnostics. |

### Language features

| Setting | Type | Default | Description |
|---|---|---|---|
| `perl-lsp.enableDiagnostics` | `boolean` | `true` | Real-time syntax diagnostics. |
| `perl-lsp.enableSemanticTokens` | `boolean` | `true` | Enhanced syntax highlighting. |
| `perl-lsp.enableFormatting` | `boolean` | `true` | Document formatting with Perl::Tidy. |
| `perl-lsp.formatOnSave` | `boolean` | `false` | Auto-format on save. |
| `perl-lsp.enableRefactoring` | `boolean` | `true` | Advanced refactoring features (rename, extract). |
| `perl-lsp.enableTestIntegration` | `boolean` | `true` | Test::More and Test2 integration. |
| `perl-lsp.autoPopulateNewFiles` | `boolean` | `true` | Insert package boilerplate into new `.pm` files and Test::More boilerplate into new `.t` files. Files with existing content are not modified. |

### Perl-specific

| Setting | Type | Default | Description |
|---|---|---|---|
| `perl-lsp.includePaths` | `string[]` | `["lib", "local/lib/perl5"]` | Additional module search paths (merged with server-side `perl.workspace.includePaths`). |
| `perl-lsp.perltidyConfig` | `string` | `""` | Path to a `.perltidyrc` configuration file. Empty = use Perl::Tidy defaults. |
| `perl-lsp.featureProfile` | `string` | `"auto"` | Feature profile passed to the server at startup (see [Feature Profiles](#feature-profiles)). |

---

## DAP Debug Configuration

Debug Adapter Protocol configuration used in `launch.json`. Source:
`crates/perl-dap-config/src/lib.rs` and `vscode-extension/package.json`.

### Launch Configuration

Start a new Perl process under the debugger.

| Property | Type | Required | Default | Description |
|---|---|---|---|---|
| `program` | `string` | Yes | — | Path to the Perl script to debug. |
| `args` | `string[]` | No | `[]` | Command-line arguments passed to the script. |
| `perlPath` | `string` | No | `"perl"` | Path to the Perl executable. |
| `includePaths` | `string[]` | No | `[]` | Paths added to `@INC` (as `-I` flags). |
| `cwd` | `string` | No | `${workspaceFolder}` | Working directory for the debugged process. |
| `env` | `object` | No | `{}` | Environment variables for the debugged process. |
| `stopOnEntry` | `boolean` | No | `true` | Pause immediately on the first line. |

Example `launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Launch Perl Script",
      "program": "${workspaceFolder}/script.pl",
      "args": ["--verbose"],
      "perlPath": "perl",
      "includePaths": ["${workspaceFolder}/lib"],
      "cwd": "${workspaceFolder}",
      "env": { "PERL5LIB": "${workspaceFolder}/lib" },
      "stopOnEntry": true
    }
  ]
}
```

### Attach Configuration

Attach to a running Perl process via TCP (requires the target to load
`Perl::LanguageServer` or a compatible debug bridge).

| Property | Type | Default | Description |
|---|---|---|---|
| `host` | `string` | `"localhost"` | Hostname or IP of the running debugger. |
| `port` | `number` | `13603` | TCP port the debugger is listening on. |
| `timeout` | `number` (ms) | `5000` | Connection timeout in milliseconds. |

Example:

```json
{
  "type": "perl",
  "request": "attach",
  "name": "Attach to Perl Debugger",
  "host": "localhost",
  "port": 13603,
  "timeout": 5000
}
```

---

## Feature Profiles

Feature profiles control which LSP capabilities the server advertises to the
client. The active profile is selected at startup via the `--feature-profile`
CLI flag or the `perl-lsp.featureProfile` VS Code setting.

| Profile | Aliases | Description |
|---|---|---|
| `production` | `prod` | Default profile. Full GA feature set. |
| `ga-lock` | `ga`, `ga_lock` | Conservative profile. Minimal surface, all features GA-locked. |
| `all` | — | All in-tree features, including proposed/experimental. |
| `auto` | — | Resolves to the compile-time default (usually `production`). |

```bash
perl-lsp --stdio --feature-profile ga-lock
perl-lsp --stdio --feature-profile all
perl-lsp --features-json --feature-profile production
```

---

## Example Configurations

### Minimal project

```json
{ "perl": { "workspace": { "includePaths": ["lib"] } } }
```

### Typical project

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
      "typeHints": true
    },
    "perlcritic": { "enabled": false }
  }
}
```

### Large codebase (10K+ files)

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"],
      "useSystemInc": false,
      "resolutionTimeout": 100
    },
    "limits": {
      "workspaceSymbolCap": 300,
      "referencesCap": 1000,
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "workspaceScanDeadlineMs": 120000
    }
  }
}
```

### Resource-constrained environment

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib"],
      "useSystemInc": false,
      "resolutionTimeout": 25
    },
    "inlayHints": { "enabled": false },
    "limits": {
      "workspaceSymbolCap": 100,
      "referencesCap": 200,
      "astCacheMaxEntries": 50,
      "maxIndexedFiles": 5000,
      "referenceSearchDeadlineMs": 1000
    }
  }
}
```

### CI / testing environment

```json
{
  "perl": {
    "workspace": { "useSystemInc": false },
    "testRunner": {
      "enabled": true,
      "command": "prove",
      "args": ["-l", "-v", "--timer"],
      "timeout": 300000
    },
    "perlcritic": { "enabled": true }
  }
}
```

### Editor-specific snippets

#### Neovim (lua)

```lua
require("lspconfig").perl_ls.setup({
  settings = {
    perl = {
      workspace = {
        includePaths = { "lib", ".", "local/lib/perl5" },
        useSystemInc = false,
      },
      inlayHints = { enabled = true, parameterHints = true },
    },
  },
})
```

#### Helix (`languages.toml`)

```toml
[language-server.perl-lsp.config.perl]
workspace.includePaths = ["lib", ".", "local/lib/perl5"]
workspace.useSystemInc = false
inlayHints.enabled = true
```

#### Emacs (eglot)

```elisp
(setq-default eglot-workspace-configuration
  '((perl
     (workspace
      (includePaths . ["lib" "." "local/lib/perl5"])
      (useSystemInc . :json-false)))))
```

#### Sublime Text (LSP package)

```json
{
  "clients": {
    "perl-lsp": {
      "initializationOptions": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"]
          }
        }
      }
    }
  }
}
```

---

## See Also

- [EDITOR_SETUP.md](../how-to/EDITOR_SETUP.md) — Editor-specific setup guides
- [PERFORMANCE_TUNING.md](../how-to/PERFORMANCE_TUNING.md) — Performance optimisation
- [PERFORMANCE_SLO.md](PERFORMANCE_SLO.md) — Performance targets and limits
- [LSP_FEATURES.md](LSP_FEATURES.md) — Supported LSP features and maturity
- [THREADING_CONFIGURATION_GUIDE.md](../how-to/THREADING_CONFIGURATION_GUIDE.md) — Threading options
- [CONFIG.md](CONFIG.md) — Legacy configuration reference (superseded by this document)
- [CONFIGURATION_SCHEMA.md](CONFIGURATION_SCHEMA.md) — JSON Schema for machine validation
