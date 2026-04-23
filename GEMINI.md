# Gemini CLI Instructions

This repository already maintains the canonical implementation-agent guide in [`AGENTS.md`](AGENTS.md).

If you are using Gemini CLI in this repo:

1. Read [`AGENTS.md`](AGENTS.md) before making changes.
2. Keep each PR scoped to one concern.
3. Run the local verification gate before opening a PR:

```bash
just pr-fast
```

Before pushing or requesting merge readiness, run:

```bash
nix develop -c just ci-gate
```

For contributor workflow details, see [`CONTRIBUTING.md`](CONTRIBUTING.md).
