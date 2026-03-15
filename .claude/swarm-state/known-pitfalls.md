# Known Pitfalls

Accumulated lessons from fixer agents and failed builds. Scouts and builders should read this before starting work to avoid repeating known mistakes.

This file is append-only during swarm operation. The janitor consolidates it periodically.

## Format

Each entry:
```
### <date> — <category>
**Source**: <branch or PR that discovered this>
**Pitfall**: <what went wrong>
**Fix**: <what the correct approach is>
**Affected crates**: <list>
```

## Entries

<!-- Agents append new entries below this line -->
