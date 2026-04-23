# Crush Setup (charmbracelet/crush)

Crush can launch `perllsp` as a project LSP and use it as additional context
for code understanding.

## Prerequisites

- `perllsp` installed and available on `PATH`
- `crush` installed and available on `PATH`
- a project root containing Perl files

Verify both CLIs first:

```bash
perllsp --version
perllsp --health
crush --version
```

## Minimal `.crush.json`

Create `.crush.json` in your project root:

```json
{
  "$schema": "https://charm.land/crush.json",
  "lsp": {
    "perl": {
      "command": "perllsp",
      "args": ["--stdio"]
    }
  }
}
```

This tells Crush to start `perllsp` over stdio for Perl files.

## Optional: Pin a Perl runtime for lint-driven features

If your workflow relies on external Perl tools (`perl`, `perlcritic`, etc.), add
an explicit `PATH` in the LSP entry so Crush launches `perllsp` with the right
runtime environment:

```json
{
  "$schema": "https://charm.land/crush.json",
  "lsp": {
    "perl": {
      "command": "perllsp",
      "args": ["--stdio"],
      "env": {
        "PATH": "/custom/perl/bin:${PATH}"
      }
    }
  }
}
```

## Troubleshooting

- If Crush does not appear to use the Perl LSP, run `perllsp --health` in the
  same shell and fix any missing runtime dependencies first.
- Enable Crush debug logs (`"options": { "debug_lsp": true }`) and inspect
  `.crush/logs/crush.log` for LSP startup errors.
- If `perllsp` works manually but fails in Crush, use an absolute command path
  (for example `/home/you/.cargo/bin/perllsp`).
