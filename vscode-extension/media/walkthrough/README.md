# Walkthrough Assets

This directory is the current home for the launch-demo visuals called out in issue #3302.

Status:
- The SVG files in this folder are storyboard previews, not recorded GIFs.
- The final GIFs come from captured editor sessions, not from the storyboards themselves.
- Once a recording exists, use `scripts/marketing/render-walkthrough-gif.py` to produce the compressed GIF and enforce a size cap.

## Planned GIFs

| Target GIF | Storyboard | Source material |
| --- | --- | --- |
| `install-health.gif` | [`install-health.svg`](install-health.svg) | Fresh install, extension auto-download, and `perllsp --health` |
| `find-references.gif` | [`find-references.svg`](find-references.svg) | Go to definition and find references over `demo_workspace/main.pl` and `demo_workspace/lib/Utils.pm` |
| `extract-variable.gif` | [`extract-variable.svg`](extract-variable.svg) | Code action refactor flow in `demo_workspace/main.pl` |

For the canonical capture checklist and asset names, see
[`../../../docs/assets/demo-asset-plan.toml`](../../../docs/assets/demo-asset-plan.toml).

## Manual Capture Notes

Use the sample files in [`../../../demo_workspace/`](../../../demo_workspace/) for a reproducible demo workspace:

- `main.pl`
- `lib/Utils.pm`
- `lib/Database.pm`

Capture the interactions in a clean editor window, then render the recording with the helper script. Keep the final artifact small enough for GitHub README usage and preserve the on-screen text at readable size.

Recommended baseline:

- Record short clips only; trim dead time before rendering.
- Start with `--fps 12` and `--width 960`.
- Pass `--max-bytes 8000000` so oversized exports fail fast instead of silently bloating the repo.
- Use `--keep-temp` when you want to inspect the generated palette.

Example:

```bash
python scripts/marketing/render-walkthrough-gif.py \
  --input recordings/install-health.mp4 \
  --output vscode-extension/media/walkthrough/install-health.gif \
  --max-bytes 8000000
```

## Render Helper

```bash
python scripts/marketing/render-walkthrough-gif.py --help
```

The helper expects a recorded input video and produces a palette-optimized GIF. It validates the input path, rejects non-GIF outputs, and can fail if the rendered asset exceeds the configured size limit. It does not generate the recording itself.
