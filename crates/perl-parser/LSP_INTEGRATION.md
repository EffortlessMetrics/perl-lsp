# LSP Integration Guide

This guide explains how to use the Perl Language Server with various editors.

## Overview

The v3 Perl parser now includes a full Language Server Protocol (LSP) implementation that provides:

- **Syntax checking** - Real-time error detection
- **Code completion** - Context-aware suggestions
- **Hover information** - Quick info about symbols
- **Code actions** - Quick fixes and refactoring
- **Diagnostics** - Linting and warnings

## Building the LSP Server

```bash
# Build the LSP server binary
cargo build --release -p perl-parser --bin perl-lsp

# The binary will be at:
# target/release/perl-lsp
```

## VSCode Integration

### 1. Create Extension Structure

```bash
mkdir perl-lsp-vscode
cd perl-lsp-vscode
npm init -y
```

### 2. Create `package.json`

```json
{
  "name": "perl-lsp",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.74.0"
  },
  "activationEvents": [
    "onLanguage:perl"
  ],
  "main": "./extension.js",
  "contributes": {
    "configuration": {
      "type": "object",
      "title": "Perl LSP",
      "properties": {
        "perl-lsp.serverPath": {
          "type": "string",
          "default": "perl-lsp",
          "description": "Path to perl-lsp executable"
        }
      }
    }
  },
  "dependencies": {
    "vscode-languageclient": "^8.0.0"
  }
}
```

### 3. Create `extension.js`

```javascript
const { LanguageClient } = require('vscode-languageclient/node');

function activate(context) {
    const serverOptions = {
        command: 'perl-lsp',
        args: ['--stdio']
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'perl' }]
    };

    const client = new LanguageClient(
        'perl-lsp',
        'Perl Language Server',
        serverOptions,
        clientOptions
    );

    context.subscriptions.push(client.start());
}

exports.activate = activate;
```

## Neovim Integration

### Using nvim-lspconfig

Add to your Neovim configuration:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define the Perl LSP
if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = {'perl-lsp', '--stdio'},
      filetypes = {'perl'},
      root_dir = lspconfig.util.find_git_ancestor,
      settings = {},
    },
  }
end

-- Enable Perl LSP
lspconfig.perl_lsp.setup{}
```

### Using CoC.nvim

Add to `coc-settings.json`:

```json
{
  "languageserver": {
    "perl": {
      "command": "perl-lsp",
      "args": ["--stdio"],
      "filetypes": ["perl"],
      "trace.server": "verbose"
    }
  }
}
```

## Vim Integration (vim-lsp)

Add to your `.vimrc`:

```vim
if executable('perl-lsp')
    au User lsp_setup call lsp#register_server({
        \ 'name': 'perl-lsp',
        \ 'cmd': {server_info->['perl-lsp', '--stdio']},
        \ 'allowlist': ['perl'],
        \ })
endif
```

## Emacs Integration (lsp-mode)

Add to your Emacs configuration:

```elisp
(require 'lsp-mode)

(defcustom lsp-perl-server-path "perl-lsp"
  "Path to perl-lsp executable."
  :type 'string
  :group 'lsp-perl)

(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection
                                   (lambda () (list lsp-perl-server-path "--stdio")))
                  :major-modes '(perl-mode cperl-mode)
                  :server-id 'perl-lsp))

(add-hook 'perl-mode-hook #'lsp)
```

## Sublime Text Integration

Create `Perl-LSP.sublime-settings`:

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl"
    }
  }
}
```

## Features Provided

### 1. Diagnostics

The LSP server provides real-time diagnostics:
- Syntax errors
- Undefined variables
- Unused variables
- Missing strict/warnings
- Deprecated syntax

### 2. Code Completion

Triggered by `$`, `@`, `%`, or `->`:
- Variable names
- Function names
- Keywords
- Built-in functions

### 3. Code Actions

Quick fixes for common issues:
- Declare undefined variables
- Remove unused variables
- Fix assignment in condition
- Add missing pragmas

### 4. Hover Information

Shows information about symbols under cursor (currently basic implementation).

## Testing the LSP Server

### Manual Testing

```bash
# Run the server in stdio mode
perl-lsp --stdio --log

# Send initialize request
Content-Length: 52

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

### Using a Test Client

```python
#!/usr/bin/env python3
import subprocess
import json

def send_request(proc, request):
    content = json.dumps(request)
    header = f"Content-Length: {len(content)}\r\n\r\n"
    proc.stdin.write(header.encode())
    proc.stdin.write(content.encode())
    proc.stdin.flush()

# Start LSP server
proc = subprocess.Popen(
    ['perl-lsp', '--stdio'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

# Initialize
send_request(proc, {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
})

# Open document
send_request(proc, {
    "jsonrpc": "2.0",
    "method": "textDocument/didOpen",
    "params": {
        "textDocument": {
            "uri": "file:///test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "print $undefined;"
        }
    }
})
```

## Troubleshooting

### Enable Logging

```bash
# Run with logging to stderr
perl-lsp --stdio --log
```

### Common Issues

1. **Server not starting**: Check that the binary is in PATH
2. **No diagnostics**: Ensure document is saved with `.pl` extension
3. **No completions**: Check trigger characters in editor config

## Future Enhancements

- [ ] Go-to-definition support
- [ ] Find references
- [ ] Rename refactoring
- [ ] Signature help
- [ ] Semantic tokens
- [ ] Workspace-wide analysis

## Contributing

The LSP server implementation is in:
- `/crates/perl-parser/src/lsp_server.rs` - Main server
- `/crates/perl-parser/src/bin/perl-lsp.rs` - Binary entry point

To add new features:
1. Implement handler in `lsp_server.rs`
2. Add method to `handle_request` match
3. Update capabilities in `handle_initialize`