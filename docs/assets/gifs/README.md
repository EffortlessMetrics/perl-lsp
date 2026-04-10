# Demo GIFs

This directory holds the animated GIFs used in README.md and other marketing
surfaces. Each GIF is produced from a manual screen-recording session using the
`scripts/marketing/render-walkthrough-gif.py` helper.

## Planned GIF Files

| Filename | Feature | Max Size | Duration |
|----------|---------|---------|---------|
| `install-health.gif` | VS Code install, extension auto-download, `perllsp --health` output | 3 MB | 15 s |
| `find-references.gif` | Ctrl+Click go to definition, Find All References panel | 3 MB | 15 s |
| `extract-variable.gif` | Select expression, light-bulb, Extract Variable code action | 3 MB | 12 s |

None of these files can be created by an automated agent because each requires a
live editor session with a running LSP server. See the [recording guide](#recording-guide)
below for instructions.

The canonical P0 tracker lives in [`../demo-asset-plan.toml`](../demo-asset-plan.toml)
and the reporting helper is [`../../../scripts/marketing/check-demo-assets.py`](../../../scripts/marketing/check-demo-assets.py).

## Storyboard Reference

SVG storyboards for each planned GIF live in
[`vscode-extension/media/walkthrough/`](../../../vscode-extension/media/walkthrough/).
Use them to rehearse the flow before recording.

| GIF | Storyboard |
|-----|-----------|
| `install-health.gif` | [`install-health.svg`](../../../vscode-extension/media/walkthrough/install-health.svg) |
| `find-references.gif` | [`find-references.svg`](../../../vscode-extension/media/walkthrough/find-references.svg) |
| `extract-variable.gif` | [`extract-variable.svg`](../../../vscode-extension/media/walkthrough/extract-variable.svg) |

## Demo Workspace

Record all GIFs using the files in [`demo_workspace/`](../../../demo_workspace/):

```
demo_workspace/
  main.pl           -- entry point (calls Utils and Database)
  lib/Utils.pm      -- process_data and load_data subs
  lib/Database.pm   -- save sub
```

Open this workspace in VS Code before recording. The workspace provides
realistic Perl code with cross-file symbol references so that go-to-definition
and find-references produce visible jumps.

## Recording Guide

### Before You Start

1. Install the VS Code extension from the Marketplace:
   ```
   code --install-extension EffortlessMetrics.perl-lsp-rs
   ```
2. Open `demo_workspace/` as a VS Code workspace folder.
3. Verify the server is running: the status bar shows "perl-lsp" and there are
   no red error indicators on `main.pl`.
4. Set VS Code to a light or high-contrast dark theme with a large font size
   (16 pt or larger) so the GIF is readable at 960 px wide.
5. Hide panels you will not use (terminal, output, source control) to minimise
   visual noise.

### Recommended Capture Settings

| Setting | Value |
|---------|-------|
| Resolution | 1280 x 720 or 1920 x 1080 |
| Frame rate | 30 fps |
| Format | MP4 (H.264) or WebM |
| Audio | Off |
| Cursor | Large, highlighted |

**Linux:** `peek`, `simplescreenrecorder`, or:
```bash
ffmpeg -video_size 1280x720 -framerate 30 -f x11grab -i :0.0 recording.mp4
```

**macOS:** QuickTime Player (File > New Screen Recording) or `ScreenFlow`.

**Windows:** Xbox Game Bar (`Win+G`), OBS Studio, or ShareX.

### GIF #1 — Install and Health Check (`install-health.gif`)

Goal: show that installing the extension is one command and the server comes up
healthy.

Steps to record:
1. Open a fresh VS Code window (no Perl files open).
2. Open the integrated terminal.
3. Run `code --install-extension EffortlessMetrics.perl-lsp-rs` and let it
   complete.
4. Open `demo_workspace/main.pl`.
5. Wait for the status bar to show the LSP is active (1–3 seconds).
6. Open the terminal again and run `perllsp --health`, then wait for the output.
7. Stop recording.

Keep the terminal text large enough to read in the final GIF.

### GIF #2 — Go to Definition and Find References (`find-references.gif`)

Goal: show instant cross-file navigation.

Steps to record:
1. Open `demo_workspace/main.pl`.
2. Hover over `Utils::process_data` so the hover tooltip appears briefly.
3. Ctrl+Click (macOS: Cmd+Click) `Utils::process_data` to jump to the
   definition in `lib/Utils.pm`.
4. Pause one second at the definition.
5. Right-click the sub name and choose "Find All References". The references
   panel opens listing the call site in `main.pl`.
6. Click the reference to jump back to `main.pl`.
7. Stop recording.

### GIF #3 — Extract Variable Code Action (`extract-variable.gif`)

Goal: show the refactoring light-bulb workflow.

Steps to record:
1. Open `demo_workspace/main.pl`.
2. Select the expression `Utils::process_data($data)` on the assignment line.
3. Wait for the light-bulb icon to appear (or press `Ctrl+.`).
4. Choose "Extract Variable" from the code action menu.
5. Type a name for the new variable (for example `$result`) and press Enter.
6. Pause one second on the refactored result.
7. Stop recording.

## Rendering the GIFs

After capturing a recording, convert it with the render helper:

```bash
python scripts/marketing/render-walkthrough-gif.py \
  --input recordings/install-health.mp4 \
  --output docs/assets/gifs/install-health.gif \
  --fps 12 \
  --width 960 \
  --max-bytes 3145728
```

Trim dead time at the start or end with `--start` and `--duration`:

```bash
python scripts/marketing/render-walkthrough-gif.py \
  --input recordings/find-references.mp4 \
  --output docs/assets/gifs/find-references.gif \
  --start 00:00:01.5 \
  --duration 00:00:14.0 \
  --fps 12 \
  --width 960 \
  --max-bytes 3145728
```

The helper requires `ffmpeg`. If `gifsicle` is also on your PATH it will be used
for an additional lossy compression pass.

If the output exceeds the byte limit the script exits with an error. Common
fixes:
- Lower `--fps` to 10.
- Lower `--width` to 800.
- Shorten the clip with `--duration`.
- Install `gifsicle` for a free extra compression pass.

## Integrating into README.md

Once all three GIFs are committed, add a Demo section to `README.md`. Place it
between the "Why Teams Pick It" and "Quick Start" sections:

```markdown
## Demo

| Install and Health Check | Go to Definition | Extract Variable |
|:---:|:---:|:---:|
| ![Install](docs/assets/gifs/install-health.gif) | ![Go to Def](docs/assets/gifs/find-references.gif) | ![Extract](docs/assets/gifs/extract-variable.gif) |
```

GitHub renders GIFs inline in Markdown so no special hosting is required.

## File Naming and Versioning

- Name GIF files after the feature, not the version: `find-references.gif` not
  `find-references-v1.gif`.
- When a workflow changes enough to require a re-record, replace the file in
  place and commit with a message like:
  `docs: re-record find-references gif for v0.13 navigation changes`
- Raw recordings are large and should not be committed. Add them to
  `docs/assets/recordings/` which is gitignored.

## Checklist for Each Release

- [ ] Verify that each GIF still matches the current UI (menu labels, status
  bar text, key bindings).
- [ ] Re-record any GIF where the workflow has changed.
- [ ] Confirm each GIF is under 3 MB after rendering.
- [ ] Check that the README table links resolve correctly.
