#!/usr/bin/env python3
"""Render a compressed demo GIF from a manually captured screen recording."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def run(command: list[str]) -> None:
    subprocess.run(command, check=True)


def build_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render a palette-optimized GIF from a manual screen recording."
    )
    parser.add_argument("--input", required=True, help="Path to the recorded video file.")
    parser.add_argument("--output", required=True, help="Path to the output GIF file.")
    parser.add_argument(
        "--fps",
        type=int,
        default=12,
        help="Output frame rate for the GIF (default: 12).",
    )
    parser.add_argument(
        "--width",
        type=int,
        default=960,
        help="Target width for the GIF; height is computed automatically (default: 960).",
    )
    parser.add_argument(
        "--start",
        default=None,
        help="Optional ffmpeg start offset (for example 00:00:02.0).",
    )
    parser.add_argument(
        "--duration",
        default=None,
        help="Optional ffmpeg duration cap (for example 00:00:08.0).",
    )
    parser.add_argument(
        "--keep-temp",
        action="store_true",
        help="Keep the temporary palette file for debugging.",
    )
    return parser.parse_args()


def main() -> int:
    args = build_args()

    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        print("error: ffmpeg is required but was not found on PATH", file=sys.stderr)
        return 2

    input_video = Path(args.input)
    if not input_video.exists():
        print(f"error: input video not found: {input_video}", file=sys.stderr)
        return 2

    output_gif = Path(args.output)
    output_gif.parent.mkdir(parents=True, exist_ok=True)

    start_args: list[str] = []
    if args.start is not None:
        start_args.extend(["-ss", args.start])

    duration_args: list[str] = []
    if args.duration is not None:
        duration_args.extend(["-t", args.duration])

    scale_filter = f"fps={args.fps},scale={args.width}:-1:flags=lanczos"
    palette_filter = f"{scale_filter},palettegen=stats_mode=diff"
    use_filter = f"{scale_filter}[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        palette = temp_path / "palette.png"

        run(
            [
                ffmpeg,
                "-y",
                *start_args,
                "-i",
                str(input_video),
                *duration_args,
                "-vf",
                palette_filter,
                str(palette),
            ]
        )

        run(
            [
                ffmpeg,
                "-y",
                *start_args,
                "-i",
                str(input_video),
                *duration_args,
                "-i",
                str(palette),
                "-lavfi",
                use_filter,
                str(output_gif),
            ]
        )

        gifsicle = shutil.which("gifsicle")
        if gifsicle is not None:
            optimized = temp_path / "optimized.gif"
            run([gifsicle, "-O3", str(output_gif), "-o", str(optimized)])
            optimized.replace(output_gif)

        if args.keep_temp:
            temp_copy = output_gif.with_suffix(".palette.png")
            shutil.copy2(palette, temp_copy)

    print(f"wrote {output_gif}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
