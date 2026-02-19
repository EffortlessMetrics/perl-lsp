# Neovim Setup Guide for perl-lsp

This comprehensive guide helps you set up and configure the Perl Language Server in Neovim.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Basic Setup](#basic-setup)
- [Configuration](#configuration)
- [Features](#features)
- [Keybindings](#keybindings)
- [Plugins](#plugins)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

---

## Prerequisites

### Required

- **Neovim** version 0.8 or later
- **perl-lsp** server installed (see [Installation](#installation))
- **nvim-lspconfig** plugin
- **nvim-cmp** (optional, for enhanced completion)

### Optional but Recommended

- **LuaSnip** - for snippet support
- **plenary.nvim** - for async utilities
- **telescope.nvim** - for fuzzy finding
- **Perl** 5.10 or later (for syntax validation)
- **perltidy** (for code formatting)
- **perlcritic** (for linting)

---

## Installation

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

### Install Required Plugins

Using [packer.nvim](https://github.com/wbthomason/packer.nvim):

```lua
use {
  'neovim/nvim-lspconfig',
  config = function()
    -- LSP configuration will go here
  end
}
```

Using [lazy.nvim](https://github.com/folke/lazy.nvim):

```lua
{
  'neovim/nvim-lspconfig',
  config = function()
    -- LSP configuration will go here
  end
}
```

---

## Basic Setup

### Minimal Configuration

Add to your `init.lua`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define perl-lsp server
if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern('.git'),
      single_file_support = true,
    },
  }
end

-- Enable the server
lspconfig.perl_lsp.setup({})
```

### Verify Setup

1. Restart Neovim
2. Open a `.pl` or `.pm` file
3. Check if LSP is attached:
   ```vim
   :LspInfo
   ```
4. You should see perl-lsp listed under "Client: perl_lsp"

---

## Configuration

### Full Configuration

Add to your `init.lua`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define perl-lsp server
if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern(
        'Makefile.PL',
        'Build.PL',
        'cpanfile',
        'dist.ini',
        '.git'
      ),
      single_file_support = true,
      settings = {
        perl = {
          workspace = {
            includePaths = { 'lib', '.', 'local/lib/perl5' },
            useSystemInc = false,
            resolutionTimeout = 50,
          },
          inlayHints = {
            enabled = true,
            parameterHints = true,
            typeHints = true,
            chainedHints = false,
            maxLength = 30,
          },
          testRunner = {
            enabled = true,
            command = 'perl',
            args = {},
            timeout = 60000,
          },
          limits = {
            workspaceSymbolCap = 200,
            referencesCap = 500,
            completionCap = 100,
            astCacheMaxEntries = 100,
            maxIndexedFiles = 10000,
            maxTotalSymbols = 500000,
            workspaceScanDeadlineMs = 30000,
            referenceSearchDeadlineMs = 2000,
          },
        },
      },
    },
  }
end

-- Enable the server with custom on_attach
lspconfig.perl_lsp.setup({
  on_attach = function(client, bufnr)
    -- Enable completion triggered by <c-x><c-o>
    vim.bo[bufnr].omnifunc = 'v:lua.vim.lsp.omnifunc'

    -- Buffer local keymaps
    local opts = { buffer = bufnr, noremap = true, silent = true }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'gD', vim.lsp.buf.declaration, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', 'gi', vim.lsp.buf.implementation, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', '<C-k>', vim.lsp.buf.signature_help, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
    vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
    vim.keymap.set('n', '<leader>f', function()
      vim.lsp.buf.format { async = true }
    end, opts)
    vim.keymap.set('n', '[d', vim.diagnostic.goto_prev, opts)
    vim.keymap.set('n', ']d', vim.diagnostic.goto_next, opts)
    vim.keymap.set('n', '<leader>q', vim.diagnostic.setloclist, opts)
  end,
  capabilities = vim.lsp.protocol.make_client_capabilities(),
})
```

### Project-Specific Configuration

Create `.nvimrc` or `init.lua` in your project root:

```lua
-- .nvimrc or lua/init.lua
vim.lsp.start {
  name = 'perl-lsp',
  cmd = { 'perl-lsp', '--stdio' },
  root_dir = vim.fn.getcwd(),
  settings = {
    perl = {
      workspace = {
        includePaths = { 'lib', 'local/lib/perl5', 'vendor/lib' },
      },
    },
  },
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
```vim
:lua vim.diagnostic.open_float()
```

### Go to Definition

Navigate to symbol definitions:

```vim
:lua vim.lsp.buf.definition()
```

Or use keybinding: `gd`

```perl
use MyModule;

MyModule::some_function();
# ^ gd here jumps to the definition
```

### Find References

Find all usages of a symbol:

```vim
:lua vim.lsp.buf.references()
```

Or use keybinding: `gr`

```perl
sub my_function {
    return 42;
}

# ^ Find references here shows all calls to my_function
```

### Hover Information

View documentation and type information:

```vim
:lua vim.lsp.buf.hover()
```

Or use keybinding: `K`

### Code Completion

Intelligent code completion:

```vim
:lua vim.lsp.buf.completion()
```

Or use keybinding: `Ctrl+x Ctrl+o`

```perl
use MyModule;

MyModule::  # Press Ctrl+x Ctrl+o for completion
```

### Document Symbols

Navigate symbols in the current file:

```vim
:lua vim.lsp.buf.document_symbol()
```

### Workspace Symbols

Search symbols across the entire workspace:

```vim
:lua vim.lsp.buf.workspace_symbol()
```

### Rename Symbol

Rename symbols across the workspace:

```vim
:lua vim.lsp.buf.rename()
```

Or use keybinding: `<leader>rn`

### Formatting

Format Perl code using perltidy:

```vim
:lua vim.lsp.buf.format { async = true }
```

Or use keybinding: `<leader>f`

### Code Actions

Quick fixes and refactorings:

```vim
:lua vim.lsp.buf.code_action()
```

Or use keybinding: `<leader>ca`

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

```lua
vim.lsp.inlay_hint.enable(bufnr, true)
```

Or use keybinding:
```lua
vim.keymap.set('n', '<leader>ih', function()
  vim.lsp.inlay_hint.enable(not vim.lsp.inlay_hint.is_enabled())
end, opts)
```

---

## Keybindings

### Default LSP Keybindings

Add these to your `init.lua`:

```lua
local opts = { buffer = bufnr, noremap = true, silent = true }

-- Navigation
vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
vim.keymap.set('n', 'gD', vim.lsp.buf.declaration, opts)
vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
vim.keymap.set('n', 'gi', vim.lsp.buf.implementation, opts)

-- Documentation
vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
vim.keymap.set('n', '<C-k>', vim.lsp.buf.signature_help, opts)

-- Refactoring
vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)

-- Formatting
vim.keymap.set('n', '<leader>f', function()
  vim.lsp.buf.format { async = true }
end, opts)

-- Diagnostics
vim.keymap.set('n', '[d', vim.diagnostic.goto_prev, opts)
vim.keymap.set('n', ']d', vim.diagnostic.goto_next, opts)
vim.keymap.set('n', '<leader>q', vim.diagnostic.setloclist, opts)
vim.keymap.set('n', '<leader>e', vim.diagnostic.open_float, opts)
```

### Telescope Integration

If using [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim):

```lua
local telescope = require('telescope.builtin')

vim.keymap.set('n', '<leader>ss', telescope.lsp_document_symbols, opts)
vim.keymap.set('n', '<leader>sw', telescope.lsp_workspace_symbols, opts)
vim.keymap.set('n', '<leader>sr', telescope.lsp_references, opts)
vim.keymap.set('n', '<leader>sd', telescope.lsp_definitions, opts)
vim.keymap.set('n', '<leader>si', telescope.lsp_implementations, opts)
```

---

## Plugins

### nvim-cmp Integration

For enhanced autocompletion with [nvim-cmp](https://github.com/hrsh7th/nvim-cmp):

```lua
local cmp = require('cmp')
local cmp_lsp = require('cmp_nvim_lsp')

cmp.setup {
  sources = {
    { name = 'nvim_lsp' },
    { name = 'buffer' },
    { name = 'path' },
  },
  mapping = cmp.mapping.preset.insert({
    ['<C-b>'] = cmp.mapping.scroll_docs(-4),
    ['<C-f>'] = cmp.mapping.scroll_docs(4),
    ['<C-Space>'] = cmp.mapping.complete(),
    ['<C-e>'] = cmp.mapping.abort(),
    ['<CR>'] = cmp.mapping.confirm({ select = true }),
  }),
}

lspconfig.perl_lsp.setup({
  capabilities = cmp_lsp.default_capabilities(),
})
```

### LuaSnip Integration

For snippet support with [LuaSnip](https://github.com/L3MON4D3/LuaSnip):

```lua
local luasnip = require('luasnip')

cmp.setup {
  snippet = {
    expand = function(args)
      luasnip.lsp_expand(args.body)
    end,
  },
  mapping = cmp.mapping.preset.insert({
    ['<Tab>'] = cmp.mapping(function(fallback)
      if cmp.visible() then
        cmp.select_next_item()
      elseif luasnip.expand_or_jumpable() then
        luasnip.expand_or_jump()
      else
        fallback()
      end
    end, { 'i', 's' }),
    ['<S-Tab>'] = cmp.mapping(function(fallback)
      if cmp.visible() then
        cmp.select_prev_item()
      elseif luasnip.jumpable(-1) then
        luasnip.jump(-1)
      else
        fallback()
      end
    end, { 'i', 's' }),
  }),
}
```

### null-ls Integration

For additional linting and formatting with [null-ls](https://github.com/jose-elias-alvarez/null-ls.nvim):

```lua
local null_ls = require('null-ls')

null_ls.setup {
  sources = {
    null_ls.builtins.diagnostics.perlcritic.with({
      extra_args = { '--severity', '3' },
    }),
    null_ls.builtins.formatting.perltidy,
  },
}
```

### nvim-lightbulb Integration

For code action hints with [nvim-lightbulb](https://github.com/kosayoda/nvim-lightbulb):

```lua
require('nvim-lightbulb').setup({
  autocmd = { enabled = true },
  sign = { enabled = true },
  virtual_text = { enabled = true },
})
```

---

## Troubleshooting

### Server Not Starting

**Symptoms**: No diagnostics, no completion, error in `:LspInfo`

**Solutions**:

1. **Verify binary is in PATH**:
   ```vim
   :!which perl-lsp
   ```

2. **Check LSP info**:
   ```vim
   :LspInfo
   ```
   Look for error messages

3. **Check LSP logs**:
   ```vim
   :lua vim.cmd('e' .. vim.lsp.get_log_path())
   ```

4. **Test server manually**:
   ```vim
   :!echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

**Symptoms**: No errors shown for invalid code

**Solutions**:

1. **Check file type**:
   ```vim
   :set filetype?
   ```
   Should output: `filetype=perl`

2. **Set file type manually**:
   ```vim
   :set filetype=perl
   ```

3. **Verify diagnostics enabled**:
   ```vim
   :lua print(vim.inspect(vim.diagnostic.config()))
   ```

### Slow Performance

**Symptoms**: Lag when typing, slow completions

**Solutions**:

1. **Reduce result caps**:
   ```lua
   settings = {
     perl = {
       limits = {
         workspaceSymbolCap = 100,
         referencesCap = 200,
         completionCap = 50,
       },
     },
   }
   ```

2. **Disable system @INC**:
   ```lua
   settings = {
     perl = {
       workspace = {
         useSystemInc = false,
       },
     },
   }
   ```

3. **Reduce resolution timeout**:
   ```lua
   settings = {
     perl = {
       workspace = {
         resolutionTimeout = 25,
       },
     },
   }
   ```

### Module Resolution Issues

**Symptoms**: Can't find modules, go-to-definition fails

**Solutions**:

1. **Check include paths**:
   ```lua
   settings = {
     perl = {
       workspace = {
         includePaths = { 'lib', '.', 'local/lib/perl5', 'vendor/lib' },
       },
     },
   }
   ```

2. **Verify module exists**:
   ```vim
   :!perl -e 'use Module::Name;'
   ```

3. **Check workspace root**:
   ```vim
   :lua print(vim.lsp.buf.list_workspace_folders()[1])
   ```

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
   ```vim
   :!perltidy --version
   ```

3. **Verify formatting enabled**:
   ```lua
   :lua vim.lsp.buf.format { async = true }
   ```

---

## Advanced Configuration

### Multi-Root Workspace

For workspaces with multiple folders:

```lua
lspconfig.perl_lsp.setup({
  on_attach = function(client, bufnr)
    local workspace_folders = vim.lsp.buf.list_workspace_folders()
    for _, folder in ipairs(workspace_folders) do
      print('Workspace folder:', folder)
    end
  end,
})
```

### Environment Variables

Set environment variables for the LSP server:

```lua
lspconfig.perl_lsp.setup({
  cmd_env = {
    PERL5LIB = vim.fn.getcwd() .. '/lib',
    PERL_MB_OPT = '--install_base ' .. vim.fn.getcwd() .. '/local',
  },
})
```

### Custom Handlers

Override default LSP handlers:

```lua
lspconfig.perl_lsp.setup({
  handlers = {
    ['textDocument/hover'] = vim.lsp.with(vim.lsp.handlers.hover, {
      border = 'rounded',
    }),
    ['textDocument/signatureHelp'] = vim.lsp.with(vim.lsp.handlers.signature_help, {
      border = 'rounded',
    }),
  },
})
```

### Diagnostic Configuration

Customize diagnostic display:

```lua
vim.diagnostic.config({
  virtual_text = {
    prefix = '■',
    spacing = 4,
  },
  signs = {
    text = {
      [vim.diagnostic.severity.ERROR] = '✗',
      [vim.diagnostic.severity.WARN] = '⚠',
      [vim.diagnostic.severity.INFO] = 'ℹ',
      [vim.diagnostic.severity.HINT] = '➤',
    },
  },
  float = {
    border = 'rounded',
  },
})
```

### Auto-Commands

Set up auto-commands for automatic actions:

```lua
vim.api.nvim_create_autocmd('BufWritePre', {
  pattern = '*.pl',
  callback = function()
    vim.lsp.buf.format { async = false }
  end,
})

vim.api.nvim_create_autocmd('LspAttach', {
  group = vim.api.nvim_create_augroup('UserLspConfig', {}),
  callback = function(ev)
    local opts = { buffer = ev.buf }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    -- ... more keymaps
  end,
})
```

### Performance Tuning

For large workspaces, adjust performance settings:

```lua
lspconfig.perl_lsp.setup({
  settings = {
    perl = {
      limits = {
        workspaceSymbolCap = 100,
        referencesCap = 200,
        completionCap = 50,
        astCacheMaxEntries = 50,
        maxIndexedFiles = 5000,
        maxTotalSymbols = 250000,
        workspaceScanDeadlineMs = 20000,
        referenceSearchDeadlineMs = 1500,
      },
      workspace = {
        resolutionTimeout = 25,
      },
    },
  },
})
```

### Debug Logging

Enable detailed logging for troubleshooting:

```lua
vim.lsp.set_log_level('debug')

-- View logs
vim.cmd('e ' .. vim.lsp.get_log_path())
```

---

## Complete Example Configuration

Here's a comprehensive example configuration:

```lua
-- init.lua

-- Plugin setup (using lazy.nvim)
return {
  {
    'neovim/nvim-lspconfig',
    config = function()
      local lspconfig = require('lspconfig')
      local configs = require('lspconfig.configs')

      -- Define perl-lsp server
      if not configs.perl_lsp then
        configs.perl_lsp = {
          default_config = {
            cmd = { 'perl-lsp', '--stdio' },
            filetypes = { 'perl' },
            root_dir = lspconfig.util.root_pattern(
              'Makefile.PL',
              'Build.PL',
              'cpanfile',
              'dist.ini',
              '.git'
            ),
            single_file_support = true,
            settings = {
              perl = {
                workspace = {
                  includePaths = { 'lib', '.', 'local/lib/perl5' },
                  useSystemInc = false,
                  resolutionTimeout = 50,
                },
                inlayHints = {
                  enabled = true,
                  parameterHints = true,
                  typeHints = true,
                  maxLength = 30,
                },
                limits = {
                  workspaceSymbolCap = 200,
                  referencesCap = 500,
                  completionCap = 100,
                },
              },
            },
          },
        }
      end

      -- Common on_attach function
      local on_attach = function(client, bufnr)
        vim.bo[bufnr].omnifunc = 'v:lua.vim.lsp.omnifunc'

        local opts = { buffer = bufnr, noremap = true, silent = true }

        -- Navigation
        vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
        vim.keymap.set('n', 'gD', vim.lsp.buf.declaration, opts)
        vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
        vim.keymap.set('n', 'gi', vim.lsp.buf.implementation, opts)

        -- Documentation
        vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
        vim.keymap.set('n', '<C-k>', vim.lsp.buf.signature_help, opts)

        -- Refactoring
        vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
        vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)

        -- Formatting
        vim.keymap.set('n', '<leader>f', function()
          vim.lsp.buf.format { async = true }
        end, opts)

        -- Diagnostics
        vim.keymap.set('n', '[d', vim.diagnostic.goto_prev, opts)
        vim.keymap.set('n', ']d', vim.diagnostic.goto_next, opts)
        vim.keymap.set('n', '<leader>q', vim.diagnostic.setloclist, opts)
        vim.keymap.set('n', '<leader>e', vim.diagnostic.open_float, opts)

        -- Inlay hints
        vim.lsp.inlay_hint.enable(bufnr, true)
      end

      -- Enable perl-lsp
      lspconfig.perl_lsp.setup({
        on_attach = on_attach,
        capabilities = vim.lsp.protocol.make_client_capabilities(),
        handlers = {
          ['textDocument/hover'] = vim.lsp.with(vim.lsp.handlers.hover, {
            border = 'rounded',
          }),
        },
      })
    end,
  },
}
```

---

## See Also

- [Getting Started](../GETTING_STARTED.md) - Quick start guide
- [Configuration Reference](../CONFIG.md) - Complete configuration options
- [Troubleshooting Guide](../TROUBLESHOOTING.md) - Common issues and solutions
- [Performance Tuning](../PERFORMANCE_TUNING.md) - Performance optimization guide
- [Editor Setup](../EDITOR_SETUP.md) - Other editor configurations
- [nvim-lspconfig Documentation](https://github.com/neovim/nvim-lspconfig)
