# Walkthrough Assets

This directory is the current home for the launch-demo visuals called out in issue #2336.

Status:
- The SVG files in this folder are storyboard previews, not recorded GIFs.
- The final GIF pass still needs a manual screen-recording capture in VS Code.
- Once a recording exists, use `scripts/marketing/render-walkthrough-gif.py` to produce the compressed GIF.

## Planned GIFs

| Target GIF | Storyboard | Source material |
| --- | --- | --- |
| `install-health.gif` | [`install-health.svg`](install-health.svg) | Fresh install, extension auto-download, and `perl-lsp --health` |
| `find-references.gif` | [`find-references.svg`](find-references.svg) | Go to definition and find references over `demo_workspace/main.pl` and `demo_workspace/lib/Utils.pm` |
| `extract-variable.gif` | [`extract-variable.svg`](extract-variable.svg) | Code action refactor flow in `demo_workspace/main.pl` |

## Manual Capture Notes

Use the sample files in [`../../../demo_workspace/`](../../../demo_workspace/) for a reproducible demo workspace:

- `main.pl`
- `lib/Utils.pm`
- `lib/Database.pm`

Capture the interactions in a clean editor window, then render the recording with the helper script. Keep the final artifact small enough for GitHub README usage and preserve the on-screen text at readable size.

## Render Helper

```bash
python scripts/marketing/render-walkthrough-gif.py --help
```

The helper expects a recorded input video and produces a palette-optimized GIF. It does not generate the recording itself.
