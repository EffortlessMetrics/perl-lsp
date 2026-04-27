# Zed Setup Guide for perl-lsp

Use this guide to run `perllsp` in Zed through Zed's built-in LSP client.

> **Current status:** Zed requires a language extension to register a language
> server for each language. The public Zed Perl extension currently registers
> `perlnavigator-server`, not `perllsp`. The configuration below works only with
> a Perl extension that registers a `perl-lsp` language server and launches
> `perllsp --stdio`.

## Prerequisites

- A current stable Zed release
- Zed 0.152.0 or later if you rely on launching Zed from a shell so `PATH`
  changes are inherited reliably
- `perllsp` installed and available on your `PATH`, unless your Zed extension
  downloads or bundles it
- A Perl project opened in Zed
- A Zed Perl extension that registers `perl-lsp` as a language server

If you rely on shell `PATH` lookup, start Zed from the same shell:

```bash
zed .
```

Verify `perllsp` before changing editor settings:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## How Zed Loads Language Servers

Zed discovers language servers through language extensions. The `lsp` block in
`settings.json` configures language servers that Zed already knows about; it
does not register a new language server by itself.

For `perl-lsp`, the installed Zed extension must register a language server ID
such as `perl-lsp` for the `Perl` language.

If Zed logs `no language server found matching 'perl-lsp'`, the extension is
missing or registered the server under a different ID.

## Configure the `perllsp` Binary

Once a Zed extension has registered `perl-lsp`, you can override the executable
path in `settings.json`:

```json
{
  "lsp": {
    "perl-lsp": {
      "binary": {
        "path": "/usr/local/bin/perllsp",
        "arguments": ["--stdio"]
      }
    }
  }
}
```

On macOS or Linux, find the path with:

```bash
command -v perllsp
```

On Windows PowerShell:

```powershell
where perllsp
```

The key name, `perl-lsp`, must match the language server ID registered by the
installed Zed extension.

## Optional: Perl File Associations

If your project uses Perl-bearing files beyond `.pl`, `.pm`, and `.t`, add file
associations:

```json
{
  "file_types": {
    "Perl": ["pl", "PL", "pm", "t", "pod", "psgi", "cgi", "fcgi"]
  }
}
```

## Optional: Server Initialization Options

Prefer `.perl-lsp.toml` for settings that should apply across all editors. Use
Zed `initialization_options` for Zed-specific startup settings.

```json
{
  "lsp": {
    "perl-lsp": {
      "initialization_options": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"]
          },
          "inlayHints": {
            "enabled": true
          }
        }
      }
    }
  }
}
```

## Optional: Semantic Tokens

Zed does not enable LSP semantic tokens by default. If your `perllsp` build and
extension support semantic tokens, enable them globally or for Perl:

```json
{
  "languages": {
    "Perl": {
      "semantic_tokens": "combined"
    }
  }
}
```

## Verify It Is Running

1. Open a Perl file such as `lib/My/Module.pm` or `t/basic.t`.
2. Confirm Zed shows the file language as `Perl`.
3. Introduce a temporary syntax error.
4. Confirm diagnostics appear.
5. Remove the syntax error after testing.

You can also try LSP-backed navigation:

- `editor: Go to Definition`
- `editor: Find All References`
- `editor: Hover`

These features depend on what `perllsp` advertises and what the Zed extension
wires through.

## Troubleshooting

- If Perl files do not activate the server, confirm that a Zed Perl extension
  is installed and that it registers the `perl-lsp` server ID.
- If Zed reports `no language server found matching 'perl-lsp'`, the `lsp` key
  does not match any registered server.
- If the server fails to launch, run `perllsp --health` and `perllsp --info` in
  a terminal first.
- If Zed cannot find `perllsp`, start Zed from a shell with `zed .`, or set an
  absolute `binary.path`.
- If `perllsp --stdio` appears to hang when run manually, that is expected: it
  is waiting for framed LSP JSON-RPC input.
- Check Zed logs with `zed: open log`.
- For more verbose startup logs, close Zed and relaunch it from a terminal
  with:

  ```bash
  zed --foreground .
  ```

For server-side behavior and configuration details, see
[docs/reference/CONFIG.md](../reference/CONFIG.md) and
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
