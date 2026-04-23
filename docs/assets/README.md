# docs/assets

Static assets used in project documentation and marketing surfaces.

| Directory | Contents |
|-----------|---------|
| [`gifs/`](gifs/) | Animated GIFs for README.md — recording guide and inventory |
| [`recordings/`](recordings/) | Raw screen-capture files (gitignored, not committed) |
| [`demo-asset-plan.toml`](demo-asset-plan.toml) | Canonical P0 walkthrough asset plan and filenames |

## Adding Assets

- GIF files belong in `gifs/` and must stay under 3 MB each.
- Raw recordings belong in `recordings/` and are never committed (see
  `.gitignore`).
- Other static images (diagrams, screenshots) can go in subdirectories here.
  Follow the naming convention of the nearest existing asset.
- Use `python scripts/marketing/check-demo-assets.py --check` to validate the
  canonical P0 walkthrough plan before recording.
