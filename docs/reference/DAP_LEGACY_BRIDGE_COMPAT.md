# DAP Legacy Bridge Compatibility

This reference is for teams maintaining older debugger setups that proxy DAP
traffic through `Perl::LanguageServer`. It is not required for the native
`perl-dap` product path.

For normal setup, use the native guide:

```text
docs/tutorials/DAP_USER_GUIDE.md
```

## Legacy Dependency

Legacy bridge compatibility requires the CPAN module `Perl::LanguageServer`.

Install it with one of:

```bash
cpan Perl::LanguageServer
cpanm Perl::LanguageServer
```

Verify availability with:

```bash
perl -e "use Perl::LanguageServer::DebuggerInterface; print qq{OK\n};"
```

## Legacy CLI Mode

The native `perl-dap` CLI is the supported product mode. Older automation that
still depends on the bridge path can start that compatibility path with:

```bash
perl-dap --bridge
```

Bridge mode starts the Rust `BridgeAdapter`, which spawns
`Perl::LanguageServer` in DAP mode and proxies messages over stdio. It is kept
only for explicit compatibility or migration work.

Socket transport is native-only. Do not combine `--bridge` with `--socket`.

## Legacy launch.json Shape

Older editor setups usually kept the same `launch.json` request shape while the
adapter process was switched to bridge mode by the extension or wrapper script.

Launch example:

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Perl: Launch Script",
  "program": "${workspaceFolder}/script.pl",
  "stopOnEntry": true,
  "args": [],
  "env": {}
}
```

Attach example:

```json
{
  "type": "perl",
  "request": "attach",
  "name": "Perl: Attach",
  "port": 5000,
  "host": "localhost"
}
```

## Troubleshooting

### Perl::LanguageServer not found

If an older bridge setup reports `Perl::LanguageServer not found`, check that
the module is installed in the same Perl environment used by the debugger:

```bash
perl -MPerl::LanguageServer -e "print qq{OK\n};"
```

If it is missing, install it with `cpan Perl::LanguageServer` or
`cpanm Perl::LanguageServer`.

### Breakpoints not hitting

Path mapping mismatches are the common cause. Confirm the `program` path in
`launch.json` points to the same file the Perl process is executing. On Windows,
also check drive-letter casing.

### Connection refused in attach mode

Confirm the Perl process is running and listening on the configured host and
port before attaching.

## Migration Note

The native `perl-dap` CLI is the supported product path. Prefer migration to
the native guide when updating editor setup, release notes, or user-facing
documentation.
