# Crush Setup

This guide shows how to wire `perllsp` into [Crush](https://github.com/charmbracelet/crush) so Crush can use Perl language-server context during agent runs.

## Prerequisites

- `crush` installed and working (`crush --version`)
- `perllsp` installed and on `PATH` (`perllsp --version`)
- your Perl project opened from its repository root

## Configure LSP in Crush

Crush reads configuration from (highest to lowest priority):

1. `.crush.json`
2. `crush.json`
3. `$HOME/.config/crush/crush.json`

For project-local setup, create `.crush.json` at your repo root:

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

## Verify

1. Start Crush from your project root:

   ```bash
   crush
   ```

2. Ask Crush to inspect a Perl symbol in your workspace (for example, “find references for `foo`”).
3. If needed, enable LSP debug logging in config:

   ```json
   {
     "$schema": "https://charm.land/crush.json",
     "options": {
       "debug_lsp": true
     }
   }
   ```

4. Review logs with:

   ```bash
   crush logs --tail 200
   ```

## Troubleshooting

- If Crush cannot launch the server, run `perllsp --health` and fix the reported environment issues first.
- If no Perl context appears, confirm Crush was started from the project root containing your Perl files.
- If `perllsp` is not found, use an absolute path for `command`, or fix `PATH` for the shell that launches Crush.

For general server diagnostics, see [Troubleshooting](../how-to/TROUBLESHOOTING.md).
