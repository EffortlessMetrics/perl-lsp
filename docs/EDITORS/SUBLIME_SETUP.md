# Sublime Text Setup Guide for perl-lsp

This comprehensive guide helps you set up and configure the Perl Language Server in Sublime Text.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Basic Setup](#basic-setup)
- [Configuration](#configuration)
- [Features](#features)
- [Keybindings](#keybindings)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

---

## Prerequisites

### Required

- **Sublime Text** version 4 or later
- **Package Control** (for installing plugins)
- **perl-lsp** server installed (see [Installation](#installation))

### Optional but Recommended

- **LSP** package (LSP client)
- **LSP-pyright** or similar LSP package
- **Perl** 5.10 or later (for syntax validation)
- **perltidy** (for code formatting)
- **perlcritic** (for linting)

---

## Installation

### Install Package Control

If you don't have Package Control installed:

1. Open Sublime Text
2. Press `Ctrl+`` (or `Cmd+`` on macOS) to open the console
3. Paste the following command and press Enter:

```python
import urllib.request,os,hashlib; h = '6f4c264a24d933ce70df5dedcf1dcaee' + 'ebe013ee18cced0ef93d5f746d80ef60'; pf = 'Package Control.sublime-package'; ipp = sublime.installed_packages_path(); urllib.request.install_opener( urllib.request.build_opener( urllib.request.ProxyHandler()) ); by = urllib.request.urlopen( 'https://packagecontrol.io/' + pf.replace(' ', '%20')).read(); dh = hashlib.sha256(by).hexdigest(); print('Error validating download (got %s instead of %s), please try manual install' % (dh, h)) if dh != h else open(os.path.join( ipp, pf), 'wb' ).write(by)
```

4. Restart Sublime Text

### Install the Server

Choose one of the following methods:

#### Option 1: Install from crates.io (Recommended)

```bash
cargo install perl-lsp
```

#### Option 2: Download Pre-built Binary

Download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases):

```bash
# Linux (x86_64)
curl -LO https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-linux-x86_64.tar.gz
tar xzf perl-lsp-linux-x86_64.tar.gz
sudo mv perl-lsp /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-darwin-aarch64.tar.gz
tar xzf perl-lsp-darwin-aarch64.tar.gz
sudo mv perl-lsp /usr/local/bin/
```

#### Option 3: Build from Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp
```

### Verify Installation

```bash
# Check version
perl-lsp --version

# Quick health check
perl-lsp --health
# Should output: ok 0.9.1
```

---

## Basic Setup

### Install LSP Package

1. Open Sublime Text
2. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS)
3. Type "Package Control: Install Package"
4. Press Enter
5. Search for "LSP"
6. Press Enter to install

### Configure perl-lsp

1. Open Sublime Text
2. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS)
3. Type "Preferences: LSP Settings"
4. Press Enter
5. Add the following configuration:

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl",
      "initializationOptions": {
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
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100
          }
        }
      }
    }
  }
}
```

### Verify Setup

1. Restart Sublime Text
2. Open a `.pl` or `.pm` file
3. Check if LSP is attached:
   - Open Command Palette: `Ctrl+Shift+P` (or `Cmd+Shift+P`)
   - Type "LSP: Enable Language Server"
   - Look for perl-lsp in the list

---

## Configuration

### Full Configuration

Open LSP settings (`Preferences: LSP Settings`) and add:

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl",
      "priority": 1,
      "initializationOptions": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"],
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
            "command": "perl",
            "args": [],
            "timeout": 60000
          },
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100,
            "astCacheMaxEntries": 100,
            "maxIndexedFiles": 10000,
            "maxTotalSymbols": 500000,
            "workspaceScanDeadlineMs": 30000,
            "referenceSearchDeadlineMs": 2000
          }
        }
      },
      "settings": {
        "LSP-lsp-perl-lsp.log_stderr": true,
        "LSP-lsp-perl-lsp.log_stdout": true
      },
      "env": {
        "PERL5LIB": "${folder}/lib",
        "PERL_MB_OPT": "--install_base ${folder}/local"
      }
    }
  },
  "log_server": [
    "perl-lsp"
  ]
}
```

### Project-Specific Configuration

Create `.sublime-project` file in your project root:

```json
{
  "folders": [
    {
      "path": "."
    }
  ],
  "settings": {
    "LSP": {
      "perl-lsp": {
        "initializationOptions": {
          "perl": {
            "workspace": {
              "includePaths": ["lib", "local/lib/perl5", "vendor/lib"]
            }
          }
        }
      }
    }
  }
}
```

---

## Features

### Syntax Diagnostics

Real-time syntax error detection and reporting:

```perl
# Errors are highlighted as you type
my $x = 1
# Missing semicolon - error shown immediately
```

View diagnostics:
- Errors are shown in the gutter
- Hover over error markers for details
- View all diagnostics: `Ctrl+Shift+P` → "LSP: Diagnostics"

### Go to Definition

Navigate to symbol definitions:

- **Keybinding**: `F12`
- **Command**: `Ctrl+Shift+P` → "LSP: Go to Definition"

```perl
use MyModule;

MyModule::some_function();
# ^ F12 here jumps to the definition
```

### Find References

Find all usages of a symbol:

- **Keybinding**: `Shift+F12`
- **Command**: `Ctrl+Shift+P` → "LSP: Find References"

```perl
sub my_function {
    return 42;
}

# ^ Find references here shows all calls to my_function
```

### Hover Information

View documentation and type information:

- **Keybinding**: `Ctrl+I` (or hover with mouse)
- **Command**: `Ctrl+Shift+P` → "LSP: Hover"

### Code Completion

Intelligent code completion:

- **Keybinding**: `Ctrl+Space` (or type to trigger)
- **Command**: `Ctrl+Shift+P` → "LSP: Complete"

```perl
use MyModule;

MyModule::  # Type here for completion
```

### Document Symbols

Navigate symbols in the current file:

- **Command**: `Ctrl+Shift+P` → "LSP: Document Symbols"

### Workspace Symbols

Search symbols across the entire workspace:

- **Command**: `Ctrl+Shift+P` → "LSP: Workspace Symbols"

### Rename Symbol

Rename symbols across the workspace:

- **Keybinding**: `F2`
- **Command**: `Ctrl+Shift+P` → "LSP: Rename"

### Formatting

Format Perl code using perltidy:

- **Keybinding**: `Ctrl+Shift+F`
- **Command**: `Ctrl+Shift+P` → "LSP: Format Document"

### Code Actions

Quick fixes and refactorings:

- **Command**: `Ctrl+Shift+P` → "LSP: Code Actions"

Available actions:
- Extract variable
- Extract subroutine
- Optimize imports
- Add missing pragmas

### Inlay Hints

Inline type and parameter hints:

```perl
sub my_function($name, $count) {
    return "Hello, $name x$count";
}

my_function("World", 5);
# ^ Shows: my_function(/* name: */ "World", /* count: */ 5)
```

Enable inlay hints:
- **Command**: `Ctrl+Shift+P` → "LSP: Toggle Inlay Hints"

---

## Keybindings

### Default LSP Keybindings

| Action | Windows/Linux | macOS | Command |
|--------|---------------|-------|---------|
| Go to Definition | `F12` | `F12` | LSP: Go to Definition |
| Find References | `Shift+F12` | `Shift+F12` | LSP: Find References |
| Rename Symbol | `F2` | `F2` | LSP: Rename |
| Format Document | `Ctrl+Shift+F` | `Cmd+Shift+F` | LSP: Format Document |
| Hover | `Ctrl+I` | `Ctrl+I` | LSP: Hover |
| Completion | `Ctrl+Space` | `Ctrl+Space` | LSP: Complete |
| Code Actions | - | - | LSP: Code Actions |
| Document Symbols | - | - | LSP: Document Symbols |
| Workspace Symbols | - | - | LSP: Workspace Symbols |
| Diagnostics | - | - | LSP: Diagnostics |

### Custom Keybindings

To customize keybindings, edit `Preferences: Key Bindings`:

```json
[
  {
    "keys": ["ctrl+shift+r"],
    "command": "lsp_rename",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["ctrl+shift+f"],
    "command": "lsp_format_document",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["ctrl+shift+a"],
    "command": "lsp_code_actions",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["ctrl+shift+d"],
    "command": "lsp_diagnostics",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  }
]
```

---

## Troubleshooting

### Server Not Starting

**Symptoms**: No diagnostics, no completion, error in console

**Solutions**:

1. **Verify binary is in PATH**:
   - Open terminal
   - Run: `which perl-lsp`
   - Should output: `/usr/local/bin/perl-lsp` or similar

2. **Check LSP status**:
   - Open Command Palette: `Ctrl+Shift+P`
   - Type "LSP: Status"
   - Look for perl-lsp in the list

3. **Check logs**:
   - Open Command Palette: `Ctrl+Shift+P`
   - Type "LSP: Toggle Log Panel"
   - Look for error messages

4. **Test server manually**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

**Symptoms**: No errors shown for invalid code

**Solutions**:

1. **Check file type**:
   - Look at the bottom-right corner
   - Should show "Perl" as the language

2. **Set file type manually**:
   - Open Command Palette: `Ctrl+Shift+P`
   - Type "Set Syntax: Perl"

3. **Verify diagnostics enabled**:
   - Open Command Palette: `Ctrl+Shift+P`
   - Type "LSP: Diagnostics"

### Slow Performance

**Symptoms**: Lag when typing, slow completions

**Solutions**:

1. **Reduce result caps**:
   ```json
   {
     "clients": {
       "perl-lsp": {
         "initializationOptions": {
           "perl": {
             "limits": {
               "workspaceSymbolCap": 100,
               "referencesCap": 200,
               "completionCap": 50
             }
           }
         }
       }
     }
   }
   ```

2. **Disable system @INC**:
   ```json
   {
     "clients": {
       "perl-lsp": {
         "initializationOptions": {
           "perl": {
             "workspace": {
               "useSystemInc": false
             }
           }
         }
       }
     }
   }
   ```

3. **Reduce resolution timeout**:
   ```json
   {
     "clients": {
       "perl-lsp": {
         "initializationOptions": {
           "perl": {
             "workspace": {
               "resolutionTimeout": 25
             }
           }
         }
       }
     }
   }
   ```

### Module Resolution Issues

**Symptoms**: Can't find modules, go-to-definition fails

**Solutions**:

1. **Check include paths**:
   ```json
   {
     "clients": {
       "perl-lsp": {
         "initializationOptions": {
           "perl": {
             "workspace": {
               "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
             }
           }
         }
       }
     }
   }
   ```

2. **Verify module exists**:
   ```bash
   perl -e 'use Module::Name;'
   ```

3. **Check workspace root**:
   - Open Command Palette: `Ctrl+Shift+P`
   - Type "LSP: Status"

### Formatting Not Working

**Symptoms**: Format command does nothing or errors

**Solutions**:

1. **Install perltidy**:
   ```bash
   # macOS
   brew install perltidy

   # Ubuntu/Debian
   sudo apt-get install perltidy

   # CentOS/RHEL
   sudo yum install perl-Perl-Tidy
   ```

2. **Check perltidy works**:
   ```bash
   perltidy --version
   ```

3. **Verify formatting enabled**:
   - Open Command Palette: `Ctrl+Shift+P`
   - Type "LSP: Format Document"

---

## Advanced Configuration

### Multi-Root Workspace

For workspaces with multiple folders, create a `.sublime-workspace` file:

```json
{
  "folders": [
    {
      "path": "./project1"
    },
    {
      "path": "./project2"
    }
  ],
  "settings": {
    "LSP": {
      "perl-lsp": {
        "initializationOptions": {
          "perl": {
            "workspace": {
              "includePaths": ["lib", "local/lib/perl5"]
            }
          }
        }
      }
    }
  }
}
```

### Environment Variables

Set environment variables for the LSP server:

```json
{
  "clients": {
    "perl-lsp": {
      "env": {
        "PERL5LIB": "${folder}/lib",
        "PERL_MB_OPT": "--install_base ${folder}/local",
        "PATH": "${env:PATH}:/usr/local/bin"
      }
    }
  }
}
```

### Custom Formatter

Use a custom formatter instead of perltidy:

```json
{
  "clients": {
    "perl-lsp": {
      "command": ["custom-formatter", "--lsp"],
      "selector": "source.perl"
    }
  }
}
```

### Debug Adapter Protocol (DAP)

Enable debugging support:

```json
{
  "clients": {
    "perl-lsp": {
      "initializationOptions": {
        "perl": {
          "debugAdapter": {
            "enabled": true,
            "port": 9257
          }
        }
      }
    }
  }
}
```

See [DAP User Guide](../DAP_USER_GUIDE.md) for more details.

### Performance Tuning

For large workspaces, adjust performance settings:

```json
{
  "clients": {
    "perl-lsp": {
      "initializationOptions": {
        "perl": {
          "limits": {
            "workspaceSymbolCap": 100,
            "referencesCap": 200,
            "completionCap": 50,
            "astCacheMaxEntries": 50,
            "maxIndexedFiles": 5000,
            "maxTotalSymbols": 250000,
            "workspaceScanDeadlineMs": 20000,
            "referenceSearchDeadlineMs": 1500
          },
          "workspace": {
            "resolutionTimeout": 25
          }
        }
      }
    }
  }
}
```

### Logging Configuration

Enable detailed logging for troubleshooting:

```json
{
  "clients": {
    "perl-lsp": {
      "settings": {
        "LSP-lsp-perl-lsp.log_stderr": true,
        "LSP-lsp-perl-lsp.log_stdout": true
      }
    }
  },
  "log_server": ["perl-lsp"]
}
```

---

## Complete Example Configuration

### LSP Settings (`Preferences: LSP Settings`)

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl",
      "priority": 1,
      "initializationOptions": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"],
            "useSystemInc": false,
            "resolutionTimeout": 50
          },
          "inlayHints": {
            "enabled": true,
            "parameterHints": true,
            "typeHints": true,
            "maxLength": 30
          },
          "testRunner": {
            "enabled": true,
            "command": "perl",
            "args": [],
            "timeout": 60000
          },
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100,
            "astCacheMaxEntries": 100,
            "maxIndexedFiles": 10000,
            "maxTotalSymbols": 500000,
            "workspaceScanDeadlineMs": 30000,
            "referenceSearchDeadlineMs": 2000
          }
        }
      },
      "settings": {
        "LSP-lsp-perl-lsp.log_stderr": true,
        "LSP-lsp-perl-lsp.log_stdout": true
      },
      "env": {
        "PERL5LIB": "${folder}/lib",
        "PERL_MB_OPT": "--install_base ${folder}/local"
      }
    }
  },
  "log_server": ["perl-lsp"]
}
```

### Key Bindings (`Preferences: Key Bindings`)

```json
[
  {
    "keys": ["f12"],
    "command": "lsp_symbol_definition",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["shift+f12"],
    "command": "lsp_symbol_references",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["f2"],
    "command": "lsp_rename",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["ctrl+shift+f"],
    "command": "lsp_format_document",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["ctrl+i"],
    "command": "lsp_hover",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["ctrl+shift+a"],
    "command": "lsp_code_actions",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  },
  {
    "keys": ["ctrl+shift+d"],
    "command": "lsp_diagnostics",
    "context": [
      { "key": "lsp.client_name", "operator": "equal", "operand": "perl-lsp" }
    ]
  }
]
```

### Project Settings (`.sublime-project`)

```json
{
  "folders": [
    {
      "path": "."
    }
  ],
  "settings": {
    "LSP": {
      "perl-lsp": {
        "initializationOptions": {
          "perl": {
            "workspace": {
              "includePaths": ["lib", "local/lib/perl5", "vendor/lib"]
            }
          }
        }
      }
    }
  }
}
```

---

## See Also

- [Getting Started](../GETTING_STARTED.md) - Quick start guide
- [Configuration Reference](../CONFIG.md) - Complete configuration options
- [Troubleshooting Guide](../TROUBLESHOOTING.md) - Common issues and solutions
- [Performance Tuning](../PERFORMANCE_TUNING.md) - Performance optimization guide
- [Editor Setup](../EDITOR_SETUP.md) - Other editor configurations
- [LSP Package Documentation](https://packagecontrol.io/packages/LSP)
