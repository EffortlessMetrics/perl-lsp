# Configuration Schema Reference

This document provides a comprehensive reference for all perl-lsp configuration options, including JSON Schema validation, default values, and examples.

## Table of Contents

- [Overview](#overview)
- [Configuration Format](#configuration-format)
- [JSON Schema](#json-schema)
- [Configuration Options](#configuration-options)
  - [Workspace Settings](#workspace-settings)
  - [Inlay Hints](#inlay-hints)
  - [Test Runner](#test-runner)
  - [Resource Limits](#resource-limits)
  - [DAP Configuration](#dap-settings)
  - [Environment Variables](#environment-variables)
- [Example Configurations](#example-configurations)
- [Validation](#validation)

---

## Overview

The perl-lsp server configuration is hierarchical, with all settings nested under the `perl` namespace. Configuration can be provided via:

1. **LSP initialization options** - Passed during server initialization
2. **workspace/didChangeConfiguration** - Updated dynamically
3. **Editor-specific settings** - Editor configuration files
4. **Environment variables** - Shell environment
5. **Project-specific files** - `.nvimrc`, `.sublime-project`, etc.

---

## Configuration Format

### Basic Structure

```json
{
  "perl": {
    "workspace": { ... },
    "inlayHints": { ... },
    "testRunner": { ... },
    "limits": { ... }
  }
}
```

Debug Adapter Protocol configuration is *not* part of this `perl.*` namespace — see
[DAP Settings](#dap-settings).

### LSP Initialization Options

```json
{
  "initializationOptions": {
    "perl": {
      // Configuration here
    }
  }
}
```

### workspace/didChangeConfiguration

```json
{
  "settings": {
    "perl": {
      // Configuration here
    }
  }
}
```

---

## JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "perl-lsp Configuration Schema",
  "description": "Configuration schema for the Perl Language Server",
  "type": "object",
  "properties": {
    "perl": {
      "type": "object",
      "description": "Perl language server configuration",
      "properties": {
        "workspace": {
          "$ref": "#/definitions/workspace"
        },
        "inlayHints": {
          "$ref": "#/definitions/inlayHints"
        },
        "testRunner": {
          "$ref": "#/definitions/testRunner"
        },
        "formatting": {
          "$ref": "#/definitions/formatting"
        },
        "perlcritic": {
          "$ref": "#/definitions/perlcritic"
        },
        "critic": {
          "$ref": "#/definitions/critic"
        },
        "telemetry": {
          "$ref": "#/definitions/telemetry"
        },
        "limits": {
          "$ref": "#/definitions/limits"
        }
      }
    }
  },
  "definitions": {
    "workspace": {
      "type": "object",
      "description": "Workspace and module resolution settings",
      "properties": {
        "includePaths": {
          "type": "array",
          "description": "Directories to search for Perl modules",
          "items": {
            "type": "string"
          },
          "default": ["lib", ".", "local/lib/perl5"]
        },
        "useSystemInc": {
          "type": "boolean",
          "description": "Include interpreter startup @INC (the result of `perl -e 'print join(\"\\n\", @INC)'`) in module resolution. Independent of PERL5LIB, which is controlled by usePerl5lib.",
          "default": false
        },
        "usePerl5lib": {
          "type": "boolean",
          "description": "Read the PERL5LIB environment variable and merge its paths into module resolution. Independent of useSystemInc; usePerl5lib controls only PERL5LIB.",
          "default": true
        },
        "perl5libPrecedence": {
          "type": "string",
          "enum": ["prepend", "append"],
          "description": "Whether PERL5LIB entries are searched before or after configured includePaths. 'prepend' matches Perl's normal runtime semantics.",
          "default": "prepend"
        },
        "resolutionTimeout": {
          "type": "number",
          "description": "Maximum time (ms) to spend resolving a module path",
          "minimum": 10,
          "maximum": 5000,
          "default": 50
        }
      },
      "additionalProperties": false
    },
    "inlayHints": {
      "type": "object",
      "description": "Inlay hints configuration",
      "properties": {
        "enabled": {
          "type": "boolean",
          "description": "Enable or disable all inlay hints",
          "default": true
        },
        "parameterHints": {
          "type": "boolean",
          "description": "Show parameter name hints in function calls",
          "default": true
        },
        "typeHints": {
          "type": "boolean",
          "description": "Show inferred type hints for variables",
          "default": true
        },
        "chainedHints": {
          "type": "boolean",
          "description": "Show hints for chained method calls",
          "default": false
        },
        "maxLength": {
          "type": "number",
          "description": "Maximum length of inlay hint text before truncation",
          "minimum": 10,
          "maximum": 100,
          "default": 30
        }
      },
      "additionalProperties": false
    },
    "testRunner": {
      "type": "object",
      "description": "Test runner configuration",
      "properties": {
        "enabled": {
          "type": "boolean",
          "description": "Enable the integrated test runner",
          "default": true
        },
        "command": {
          "type": "string",
          "description": "Command to run tests",
          "default": "perl"
        },
        "args": {
          "type": "array",
          "description": "Additional arguments to pass to the test command",
          "items": {
            "type": "string"
          },
          "default": []
        },
        "timeout": {
          "type": "number",
          "description": "Maximum time (ms) to wait for test execution",
          "minimum": 1000,
          "maximum": 300000,
          "default": 60000
        }
      },
      "additionalProperties": false
    },
    "formatting": {
      "type": "object",
      "description": "Native-first formatter configuration. The default engine is native; external perltidy is explicit compatibility mode.",
      "properties": {
        "enabled": {
          "type": "boolean",
          "description": "Enable LSP formatting requests",
          "default": true
        },
        "engine": {
          "type": "string",
          "description": "Formatter engine: native, compat/perltidy-compat, external-perltidy/external-legacy/perltidy, or off/disabled/none",
          "enum": ["native", "compat", "perltidy-compat", "external-perltidy", "external-legacy", "perltidy", "off", "disabled", "none"],
          "default": "native"
        },
        "perltidy_profile": {
          "type": "string",
          "description": "Path to a .perltidyrc profile used for compatibility reporting or explicit external mode"
        },
        "perltidy_maximum_line_length": {
          "type": "integer",
          "description": "Maximum native formatter line width",
          "minimum": 20,
          "default": 80
        },
        "perltidy_indent_columns": {
          "type": "integer",
          "description": "Indent width in spaces",
          "minimum": 1,
          "default": 4
        },
        "perltidy_tabs": {
          "type": "boolean",
          "description": "Use tabs for indentation where supported",
          "default": false
        },
        "perltidy_opening_brace_on_new_line": {
          "type": "boolean",
          "description": "Place supported opening braces on a new line",
          "default": false
        },
        "perltidy_cuddled_else": {
          "type": "boolean",
          "description": "Use cuddled else/elsif layout for supported blocks",
          "default": true
        },
        "perltidy_space_after_keyword": {
          "type": "boolean",
          "description": "Insert a space between supported control-flow keywords and their condition",
          "default": true
        },
        "perltidy_add_trailing_commas": {
          "type": "boolean",
          "description": "Add trailing commas in supported multi-line lists and hashes",
          "default": false
        }
      },
      "additionalProperties": false
    },
    "perlcritic": {
      "type": "object",
      "description": "Critic diagnostics compatibility settings. Native critic is the default engine; legacy Perl::Critic is explicit compatibility mode.",
      "properties": {
        "enabled": {
          "type": "boolean",
          "description": "Enable critic diagnostics",
          "default": true
        },
        "severity": {
          "type": "integer",
          "description": "Minimum critic severity to report: 1 (least severe, reports everything) to 5 (most severe, reports less)",
          "minimum": 1,
          "maximum": 5,
          "default": 3
        },
        "profile": {
          "type": "string",
          "description": "Path to a .perlcriticrc profile file"
        }
      },
      "additionalProperties": false
    },
    "critic": {
      "type": "object",
      "description": "Native critic engine selection and profile settings",
      "properties": {
        "engine": {
          "type": "string",
          "description": "Critic engine: native, legacy, external, or perlcritic",
          "enum": ["native", "legacy", "external", "perlcritic"],
          "default": "native"
        },
        "profile": {
          "type": "string",
          "description": "Native critic profile",
          "enum": ["recommended", "strict"],
          "default": "recommended"
        }
      },
      "additionalProperties": false
    },
    "telemetry": {
      "type": "object",
      "description": "Telemetry configuration",
      "properties": {
        "enabled": {
          "type": "boolean",
          "description": "Enable telemetry/event notifications to the client",
          "default": false
        }
      },
      "additionalProperties": false
    },
    "limits": {
      "type": "object",
      "description": "Resource limits and performance tuning",
      "properties": {
        "workspaceSymbolCap": {
          "type": "integer",
          "description": "Maximum number of results from workspace/symbol requests",
          "minimum": 10,
          "maximum": 1000,
          "default": 200
        },
        "referencesCap": {
          "type": "integer",
          "description": "Maximum number of results from textDocument/references requests",
          "minimum": 10,
          "maximum": 5000,
          "default": 500
        },
        "completionCap": {
          "type": "integer",
          "description": "Maximum number of completion items to return",
          "minimum": 10,
          "maximum": 500,
          "default": 100
        },
        "documentSymbolCap": {
          "type": "integer",
          "description": "Maximum number of results from textDocument/documentSymbol requests",
          "minimum": 10,
          "maximum": 2000,
          "default": 500
        },
        "codeLensCap": {
          "type": "integer",
          "description": "Maximum code lens items per file",
          "minimum": 10,
          "maximum": 500,
          "default": 100
        },
        "diagnosticsPerFileCap": {
          "type": "integer",
          "description": "Maximum diagnostics per file",
          "minimum": 10,
          "maximum": 1000,
          "default": 200
        },
        "inlayHintsCap": {
          "type": "integer",
          "description": "Maximum inlay hints per file",
          "minimum": 10,
          "maximum": 2000,
          "default": 500
        },
        "astCacheMaxEntries": {
          "type": "integer",
          "description": "Maximum number of AST cache entries (LRU eviction)",
          "minimum": 10,
          "maximum": 500,
          "default": 100
        },
        "astCacheTtlSecs": {
          "type": "integer",
          "description": "AST cache TTL in seconds",
          "minimum": 10,
          "maximum": 3600,
          "default": 300
        },
        "symbolCacheMaxEntries": {
          "type": "integer",
          "description": "Maximum symbol cache entries",
          "minimum": 10,
          "maximum": 10000,
          "default": 1000
        },
        "maxIndexedFiles": {
          "type": "integer",
          "description": "Maximum number of files to index in workspace",
          "minimum": 100,
          "maximum": 100000,
          "default": 10000
        },
        "maxSymbolsPerFile": {
          "type": "integer",
          "description": "Maximum symbols indexed per file",
          "minimum": 100,
          "maximum": 50000,
          "default": 5000
        },
        "maxTotalSymbols": {
          "type": "integer",
          "description": "Maximum total symbols across all indexed files",
          "minimum": 10000,
          "maximum": 1000000,
          "default": 500000
        },
        "parseStormThreshold": {
          "type": "integer",
          "description": "Pending parse count before degradation mode activates",
          "minimum": 1,
          "maximum": 100,
          "default": 10
        },
        "maxFileSizeBytes": {
          "type": "integer",
          "description": "Skip files larger than this in bytes (default: 1 MB)",
          "minimum": 1024,
          "default": 1048576
        },
        "workspaceScanDeadlineMs": {
          "type": "integer",
          "description": "Deadline (ms) for initial workspace folder scan",
          "minimum": 5000,
          "maximum": 120000,
          "default": 30000
        },
        "fileIndexDeadlineMs": {
          "type": "integer",
          "description": "Deadline (ms) for single file indexing",
          "minimum": 100,
          "maximum": 30000,
          "default": 5000
        },
        "referenceSearchDeadlineMs": {
          "type": "integer",
          "description": "Deadline (ms) for reference search",
          "minimum": 500,
          "maximum": 10000,
          "default": 2000
        },
        "regexScanDeadlineMs": {
          "type": "integer",
          "description": "Deadline (ms) for regex scan",
          "minimum": 100,
          "maximum": 5000,
          "default": 1000
        },
        "fsOperationDeadlineMs": {
          "type": "integer",
          "description": "Deadline (ms) for filesystem operations",
          "minimum": 50,
          "maximum": 5000,
          "default": 500
        }
      },
      "additionalProperties": false
    }
  }
}
```

---

## Configuration Options

### Workspace Settings

Configuration for module resolution and workspace behavior.

#### `perl.workspace.includePaths`

| Property | Value |
|----------|-------|
| Type | `string[]` |
| Default | `["lib", ".", "local/lib/perl5"]` |
| Minimum | 1 item |
| Maximum | 50 items |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Directories to search for Perl modules, relative to the workspace root.

**Example:**

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
    }
  }
}
```

**Validation Rules:**
- All paths must be relative to workspace root
- Paths must not contain `..` segments
- Maximum depth: 10 levels

#### `perl.workspace.useSystemInc`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `false` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Whether to include the selected Perl interpreter's startup `@INC` paths
(the result of `perl -e 'print join("\n", @INC)'`) in module resolution.
Disabled by default to avoid slow filesystem probes and surprising matches
from globally installed modules.

This does **not** control `PERL5LIB`; use `usePerl5lib` for that. The two
settings are independent: `useSystemInc` controls interpreter startup `@INC`
only, while `usePerl5lib` controls `PERL5LIB`.

**Security Note:** The current directory (`.`) is filtered from system `@INC` to prevent injection attacks. When `usePerl5lib` is `false`, `PERL5LIB` is also stripped from the probe subprocess environment, so disabling `usePerl5lib` truly disables PERL5LIB even when `useSystemInc=true`.

**Example:**

```json
{
  "perl": {
    "workspace": {
      "useSystemInc": true
    }
  }
}
```

**Validation Rules:**
- Must be boolean
- Cannot be changed while server is running (requires restart)

#### `perl.workspace.usePerl5lib`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Whether `perl-lsp` reads the `PERL5LIB` environment variable and merges its
paths into module resolution. Default is enabled so the LSP behaves like
running `perl` directly from the same shell.

This is separate from `useSystemInc`. `PERL5LIB` is an explicit environment
search path; `useSystemInc` controls probing the selected Perl interpreter's
startup `@INC`. Set `usePerl5lib` to `false` when you want module resolution
to ignore ambient shell environment paths and rely only on project /
workspace configuration.

**Example:**

```json
{
  "perl": {
    "workspace": {
      "usePerl5lib": false
    }
  }
}
```

**Validation Rules:**
- Must be boolean
- Toggling invalidates the lazy startup-`@INC` cache so the next probe
  re-runs with the correct `PERL5LIB` environment.

#### `perl.workspace.perl5libPrecedence`

| Property | Value |
|----------|-------|
| Type | `"prepend"` or `"append"` |
| Default | `"prepend"` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Controls whether `PERL5LIB` entries are searched before or after configured
`includePaths`. `"prepend"` matches Perl's normal runtime behavior:
`PERL5LIB` paths appear before later library paths.

Only takes effect when `usePerl5lib` is `true` and `PERL5LIB` is non-empty.

**Example:**

```json
{
  "perl": {
    "workspace": {
      "perl5libPrecedence": "append"
    }
  }
}
```

**Validation Rules:**
- Must be `"prepend"` or `"append"`. Unknown values are ignored (current
  setting is preserved) rather than rejected, so a typo does not silently
  reset an explicitly-configured `"append"`.

#### `perl.workspace.resolutionTimeout`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `50` |
| Minimum | `10` |
| Maximum | `5000` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Maximum time to spend resolving a module path. Prevents blocking on slow/network filesystems.

**Example:**

```json
{
  "perl": {
    "workspace": {
      "resolutionTimeout": 100
    }
  }
}
```

**Validation Rules:**
- Must be positive integer
- Values below 10ms may cause resolution failures
- Values above 5000ms may cause UI lag

---

### Inlay Hints

Configuration for inlay hints displayed in the editor.

#### `perl.inlayHints.enabled`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Enable or disable all inlay hints.

**Example:**

```json
{
  "perl": {
    "inlayHints": {
      "enabled": false
    }
  }
}
```

#### `perl.inlayHints.parameterHints`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Show parameter name hints in function calls.

**Example:**

```perl
# With parameterHints enabled:
some_function(/* name: */ "value", /* count: */ 42);
```

**Configuration:**

```json
{
  "perl": {
    "inlayHints": {
      "parameterHints": true
    }
  }
}
```

#### `perl.inlayHints.typeHints`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Show inferred type hints for variables.

**Example:**

```perl
# With typeHints enabled:
my $x = 42;  # : Int
my $name = "Hello";  # : Str
```

**Configuration:**

```json
{
  "perl": {
    "inlayHints": {
      "typeHints": true
    }
  }
}
```

#### `perl.inlayHints.chainedHints`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `false` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Show hints for chained method calls.

**Example:**

```perl
# With chainedHints enabled:
$obj->method1()->method2()->method3();
# ^ Shows: /* returns Type1 */ /* returns Type2 */ /* returns Type3 */
```

**Configuration:**

```json
{
  "perl": {
    "inlayHints": {
      "chainedHints": true
    }
  }
}
```

#### `perl.inlayHints.maxLength`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `30` |
| Minimum | `10` |
| Maximum | `100` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Maximum length of inlay hint text before truncation.

**Example:**

```json
{
  "perl": {
    "inlayHints": {
      "maxLength": 50
    }
  }
}
```

**Validation Rules:**
- Must be positive integer
- Values below 10 may truncate useful information
- Values above 100 may clutter the editor

---

### Test Runner

Configuration for integrated test execution.

#### `perl.testRunner.enabled`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Enable the integrated test runner.

**Example:**

```json
{
  "perl": {
    "testRunner": {
      "enabled": true
    }
  }
}
```

#### `perl.testRunner.command`

| Property | Value |
|----------|-------|
| Type | `string` |
| Default | `"perl"` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Command to run tests.

**Example:**

```json
{
  "perl": {
    "testRunner": {
      "command": "prove"
    }
  }
}
```

**Validation Rules:**
- Must be valid executable name or path
- Must be in system PATH

#### `perl.testRunner.args`

| Property | Value |
|----------|-------|
| Type | `string[]` |
| Default | `[]` |
| Maximum | 20 items |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Additional arguments to pass to the test command.

**Example:**

```json
{
  "perl": {
    "testRunner": {
      "command": "prove",
      "args": ["-l", "-v", "--timer"]
    }
  }
}
```

**Validation Rules:**
- Each argument must be a valid string
- Maximum 20 arguments to prevent command injection

#### `perl.testRunner.timeout`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `60000` |
| Minimum | `1000` |
| Maximum | `300000` |
| Source | `crates/perl-lsp-rs-core/src/config/mod.rs` |

Maximum time to wait for test execution.

**Example:**

```json
{
  "perl": {
    "testRunner": {
      "timeout": 120000
    }
  }
}
```

**Validation Rules:**
- Must be positive integer
- Values below 1000ms may timeout on slow tests
- Values above 300000ms (5 minutes) may cause UI freezes

---

### Resource Limits

Configuration for bounded behavior and performance tuning.

#### Result Caps

##### `perl.limits.workspaceSymbolCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `200` |
| Minimum | `10` |
| Maximum | `1000` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Maximum number of results from `workspace/symbol` requests.

**Example:**

```json
{
  "perl": {
    "limits": {
      "workspaceSymbolCap": 100
    }
  }
}
```

##### `perl.limits.referencesCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `500` |
| Minimum | `10` |
| Maximum | `5000` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Maximum number of results from `textDocument/references` requests.

**Example:**

```json
{
  "perl": {
    "limits": {
      "referencesCap": 200
    }
  }
}
```

##### `perl.limits.completionCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `100` |
| Minimum | `10` |
| Maximum | `500` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Maximum number of completion items to return.

**Example:**

```json
{
  "perl": {
    "limits": {
      "completionCap": 50
    }
  }
}
```

#### Index Limits

##### `perl.limits.astCacheMaxEntries`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `100` |
| Minimum | `10` |
| Maximum | `500` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Maximum number of AST cache entries. Uses LRU eviction when exceeded.

**Example:**

```json
{
  "perl": {
    "limits": {
      "astCacheMaxEntries": 50
    }
  }
}
```

##### `perl.limits.maxIndexedFiles`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `10000` |
| Minimum | `100` |
| Maximum | `100000` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Maximum number of files to index in workspace. Skips older/less-used files when exceeded.

**Example:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 5000
    }
  }
}
```

##### `perl.limits.maxTotalSymbols`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `500000` |
| Minimum | `10000` |
| Maximum | `1000000` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Maximum total symbols across all indexed files. Uses LRU eviction when exceeded.

**Example:**

```json
{
  "perl": {
    "limits": {
      "maxTotalSymbols": 250000
    }
  }
}
```

#### Deadline Limits

##### `perl.limits.workspaceScanDeadlineMs`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `30000` |
| Minimum | `5000` |
| Maximum | `120000` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Deadline (ms) for workspace folder scan. Returns partial index when exceeded.

**Example:**

```json
{
  "perl": {
    "limits": {
      "workspaceScanDeadlineMs": 20000
    }
  }
}
```

##### `perl.limits.referenceSearchDeadlineMs`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `2000` |
| Minimum | `500` |
| Maximum | `10000` |
| Source | `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` |

Deadline (ms) for reference search. Returns partial results when exceeded.

**Example:**

```json
{
  "perl": {
    "limits": {
      "referenceSearchDeadlineMs": 1500
    }
  }
}
```

---

### DAP Settings

Debug Adapter Protocol configuration is not exposed as an LSP workspace setting under
`perl.debugAdapter`. DAP sessions are configured via `launch.json` in your editor using
DAP launch and attach configurations. Source: `crates/perl-dap/src/config/mod.rs`.

#### Launch Configuration

Start a new Perl process under the debugger.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `program` | `string` | — | Path to the Perl script to debug |
| `args` | `string[]` | `[]` | Command-line arguments for the script |
| `perlPath` | `string` | `"perl"` | Path to the Perl executable |
| `includePaths` | `string[]` | `[]` | Paths added to `@INC` as `-I` flags |
| `cwd` | `string` | workspace root | Working directory for the debugged process |
| `env` | `object` | `{}` | Environment variables for the debugged process |

**Example `launch.json`:**

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
      "env": { "PERL5LIB": "${workspaceFolder}/lib" }
    }
  ]
}
```

#### Attach Configuration

Attach to a running Perl debugger process over TCP.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `host` | `string` | `"localhost"` | Hostname or IP of the running debugger |
| `port` | `number` | `13603` | TCP port the debugger is listening on |
| `timeout` | `number` (ms) | `5000` | Connection timeout in milliseconds |

**Example:**

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

See [DAP User Guide](../tutorials/DAP_USER_GUIDE.md) for a full walkthrough.

---

### Environment Variables

Environment variables read at startup by the server. Source: `crates/perl-lsp-rs-core/src/runtime/launcher/mod.rs`.

Workspace settings (includePaths, useSystemInc, etc.) are not configurable via environment
variables — use LSP `initializationOptions` or `workspace/didChangeConfiguration` instead.

#### `PERL_LSP_LOG`

Set to any non-empty value to enable logging to stderr. Equivalent to the `--log` CLI flag.

```bash
export PERL_LSP_LOG=1
```

#### `RUST_LOG`

Standard `tracing`/`env_logger` filter directive. Controls log level and per-module filtering.
Takes precedence over the `--log` flag default filter.

```bash
export RUST_LOG=perl_lsp=debug
export RUST_LOG=warn
```

Common log levels: `error`, `warn`, `info`, `debug`, `trace`.

#### `NO_COLOR`

When set, disables ANSI colour in log output. Follows the [no-color.org](https://no-color.org)
convention.

```bash
export NO_COLOR=1
```

#### `PERL5LIB`

Standard Perl library path environment variable. `perl-lsp` reads this for
module resolution when `perl.workspace.usePerl5lib` is `true` (the default).
Its ordering relative to configured `includePaths` is controlled by
`perl.workspace.perl5libPrecedence`.

`PERL5LIB` is **not** the same as interpreter startup `@INC`. Startup `@INC`
probing — the result of `perl -e 'print join("\n", @INC)'` — is controlled
separately by `perl.workspace.useSystemInc`. The two sources are independent:

- Project libraries → `perl.workspace.includePaths`
- Environment libraries → `perl.workspace.usePerl5lib` + `PERL5LIB`
- Globally installed libraries → `perl.workspace.useSystemInc` + interpreter startup `@INC`

```bash
export PERL5LIB="/path/to/lib:/another/path"
```

---

## Example Configurations

### Minimal Configuration

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib"]
    }
  }
}
```

### Typical Project Configuration

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"],
      "useSystemInc": false,
      "resolutionTimeout": 50
    },
    "inlayHints": {
      "enabled": true,
      "parameterHints": true,
      "typeHints": true
    },
    "limits": {
      "workspaceSymbolCap": 200,
      "referencesCap": 500,
      "completionCap": 100
    }
  }
}
```

### Performance-Optimized Configuration

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
      "workspaceSymbolCap": 100,
      "referencesCap": 200,
      "completionCap": 50,
      "astCacheMaxEntries": 50,
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 250000,
      "workspaceScanDeadlineMs": 20000,
      "referenceSearchDeadlineMs": 1500
    }
  }
}
```

### Development Configuration

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", "t/lib", "local/lib/perl5"],
      "useSystemInc": true,
      "resolutionTimeout": 100
    },
    "inlayHints": {
      "enabled": true,
      "parameterHints": true,
      "typeHints": true,
      "chainedHints": true,
      "maxLength": 50
    },
    "testRunner": {
      "enabled": true,
      "command": "prove",
      "args": ["-l", "-v", "--timer"],
      "timeout": 120000
    },
    "limits": {
      "workspaceSymbolCap": 500,
      "referencesCap": 1000,
      "completionCap": 200
    }
  }
}
```

DAP debugging is configured separately via `launch.json` — see [DAP Settings](#dap-settings).

### Editor-Specific Examples

#### VS Code

```json
{
  "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5"],
  "perl-lsp.useSystemInc": false,
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true
}
```

#### Neovim

```lua
settings = {
  perl = {
    workspace = {
      includePaths = { "lib", ".", "local/lib/perl5" },
      useSystemInc = false,
      resolutionTimeout = 50,
    },
    inlayHints = {
      enabled = true,
      parameterHints = true,
      typeHints = true,
    },
  },
}
```

#### Emacs

```elisp
(setq-default eglot-workspace-configuration
  '((perl
     (workspace
      (includePaths . ["lib" "." "local/lib/perl5"])
      (useSystemInc . :json-false)))))
```

#### Helix

```toml
[language-server.perl-lsp.config.perl]
workspace.includePaths = ["lib", ".", "local/lib/perl5"]
workspace.useSystemInc = false
inlayHints.enabled = true
```

#### Sublime Text

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

## Validation

### JSON Schema Validation

Use a JSON Schema validator to validate your configuration. Copy the JSON Schema block
from the [JSON Schema](#json-schema) section above into a file (e.g., `perl-lsp-schema.json`),
then validate:

```bash
# Using ajv-cli
npx ajv validate -s perl-lsp-schema.json -d .vscode/settings.json

# Using python-jsonschema
python -m jsonschema -i .vscode/settings.json perl-lsp-schema.json
```

### Online Validation

Use online JSON Schema validators:
- [JSON Schema Validator](https://www.jsonschemavalidator.net/)
- [JSONLint](https://jsonlint.com/)

### Common Validation Errors

#### Type Mismatch

```json
{
  "perl": {
    "workspace": {
      "includePaths": "lib"  // Error: should be array
    }
  }
}
```

**Fix:**

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib"]
    }
  }
}
```

#### Value Out of Range

```json
{
  "perl": {
    "limits": {
      "completionCap": 1000  // Error: maximum is 500
    }
  }
}
```

**Fix:**

```json
{
  "perl": {
    "limits": {
      "completionCap": 500
    }
  }
}
```

#### Missing Required Property

```json
{
  "perl": {
    // Error: missing workspace configuration
  }
}
```

**Fix:**

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib"]
    }
  }
}
```

---

## See Also

- [Getting Started](../tutorials/GETTING_STARTED.md) - Quick start guide
- [Editor Setup](../how-to/EDITOR_SETUP.md) - Editor-specific configuration
- [Performance Tuning](../how-to/PERFORMANCE_TUNING.md) - Performance optimization guide
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md) - Common issues and solutions
- [DAP User Guide](../tutorials/DAP_USER_GUIDE.md) - Debugging configuration
