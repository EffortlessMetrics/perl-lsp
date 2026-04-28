# Neovim Setup Guide for perl-lsp

Use this guide to run `perllsp` in Neovim through Neovim's built-in LSP client.

## Prerequisites

- Neovim 0.11.3 or later (current stable recommended)
- `perllsp` installed and available on your `PATH`
- a Perl project opened from the project root

Optional:

- `nvim-lspconfig`, if you already use it for other language servers
- `nvim-cmp`, if you prefer cmp-based completion
- `telescope.nvim`, if you want Telescope-backed symbol/reference pickers
- `perltidy`, for formatting
- `perlcritic`, only if Perl::Critic diagnostics are enabled

Verify `perllsp` before changing Neovim configuration:

```bash
perllsp --version
perllsp --health
perllsp --info
```

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

Download the archive for your platform from GitHub Releases, extract it, and put
`perllsp` on your `PATH`.

Release assets use the `perllsp-<version>-<target>` naming pattern.

## Basic Setup (Neovim 0.11+)

Create a custom LSP config file:

```vim
:exe 'edit' stdpath('config') .. '/lsp/perllsp.lua'
```

Add:

```lua
return {
  cmd = { 'perllsp', '--stdio' },
  filetypes = { 'perl' },
  root_markers = {
    '.perl-lsp.toml',
    'Makefile.PL',
    'Build.PL',
    'cpanfile',
    'dist.ini',
    '.git',
  },
  init_options = {
    perl = {
      workspace = {
        includePaths = { 'lib', '.', 'local/lib/perl5' },
        useSystemInc = false,
        resolutionTimeout = 50,
      },
    },
  },
}
```

Then enable it from `init.lua`:

```lua
vim.lsp.enable('perllsp')
```

Restart Neovim, open a Perl file, and run:

```vim
:checkhealth vim.lsp
```

## Optional Inline Config

```lua
vim.lsp.config('perllsp', {
  cmd = { 'perllsp', '--stdio' },
  filetypes = { 'perl' },
  root_markers = {
    '.perl-lsp.toml',
    'Makefile.PL',
    'Build.PL',
    'cpanfile',
    'dist.ini',
    '.git',
  },
  init_options = {
    perl = {
      workspace = {
        includePaths = { 'lib', '.', 'local/lib/perl5' },
        useSystemInc = false,
      },
      inlayHints = {
        enabled = true,
        parameterHints = true,
        typeHints = true,
      },
      limits = {
        workspaceSymbolCap = 200,
        referencesCap = 500,
        completionCap = 100,
      },
    },
  },
})

vim.lsp.enable('perllsp')
```

## Optional: Filetype Detection

Neovim starts the server only when filetype is `perl`.

```vim
:set filetype?
```

Add filetype rules if needed:

```lua
vim.filetype.add({
  extension = {
    t = 'perl',
    psgi = 'perl',
    cgi = 'perl',
    fcgi = 'perl',
    PL = 'perl',
  },
})
```

## Optional: Built-in Completion

```lua
vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(ev)
    local client = vim.lsp.get_client_by_id(ev.data.client_id)
    if not client or client.name ~= 'perllsp' then
      return
    end

    vim.lsp.completion.enable(true, client.id, ev.buf, {
      autotrigger = true,
    })

    vim.keymap.set('i', '<C-Space>', function()
      vim.lsp.completion.get()
    end, { buffer = ev.buf, desc = 'Trigger LSP completion' })
  end,
})
```

## Optional: Inlay Hints

```lua
vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(ev)
    local client = vim.lsp.get_client_by_id(ev.data.client_id)
    if not client or client.name ~= 'perllsp' then
      return
    end

    if client:supports_method('textDocument/inlayHint') then
      vim.lsp.inlay_hint.enable(true, { bufnr = ev.buf })
    end
  end,
})

vim.keymap.set('n', '<leader>ih', function()
  vim.lsp.inlay_hint.enable(
    not vim.lsp.inlay_hint.is_enabled({ bufnr = 0 }),
    { bufnr = 0 }
  )
end, { desc = 'Toggle inlay hints' })
```

## Verify It Is Running

1. Restart Neovim.
2. Open a Perl file (`.pl`, `.pm`, `.t`).
3. Confirm filetype: `:set filetype?`.
4. Check state: `:checkhealth vim.lsp`.
5. Introduce and remove a temporary syntax error.

You can also check a file outside Neovim:

```bash
perllsp --check path/to/file.pl
```

## Troubleshooting

### Neovim cannot find `perllsp`

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

PowerShell:

```powershell
where perllsp
```

In Neovim:

```vim
:!command -v perllsp
```

### Server does not attach

Check:

```vim
:set filetype?
:checkhealth vim.lsp
```

Common causes:

- buffer filetype is not `perl`
- workspace has no root marker
- `perllsp` is not on `PATH`
- config was not enabled with `vim.lsp.enable('perllsp')`

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input
from Neovim. Use these for manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

## Legacy Setup (Neovim 0.8–0.10 + nvim-lspconfig)

Use this only if you cannot upgrade.

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.perllsp then
  configs.perllsp = {
    default_config = {
      cmd = { 'perllsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern(
        '.perl-lsp.toml',
        'Makefile.PL',
        'Build.PL',
        'cpanfile',
        'dist.ini',
        '.git'
      ),
      single_file_support = true,
      init_options = {
        perl = {
          workspace = {
            includePaths = { 'lib', '.', 'local/lib/perl5' },
            useSystemInc = false,
          },
        },
      },
    },
  }
end

lspconfig.perllsp.setup({})
```

For additional details:

- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [Editor Setup](../how-to/EDITOR_SETUP.md)
