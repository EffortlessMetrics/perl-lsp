# Badge endpoints

This directory contains generated Shields endpoint JSON used by README badges.

Regenerate:

```bash
cargo xtask badges
```

Check drift:

```bash
cargo xtask badges --check
```

Only committed `*.json` endpoint files are public badge surfaces. Detailed reports stay in CI artifacts and `target/`.
