#!/usr/bin/env python3
"""Render a compressed demo GIF from a captured screen recording."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def run(command: list[str]) -> None:
    subprocess.run(command, check=True)


def positive_int(value: str) -> int:
    try:
        parsed = int(value, 10)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"expected a positive integer, got {value!r}") from error
    if parsed <= 0:
        raise argparse.ArgumentTypeError(f"expected a positive integer, got {parsed}")
    return parsed


def format_size(num_bytes: int) -> str:
    if num_bytes < 1024:
        return f"{num_bytes} B"
    units = ["KiB", "MiB", "GiB"]
    value = float(num_bytes)
    for unit in units:
        value /= 1024.0
        if value < 1024.0 or unit == units[-1]:
            return f"{value:.1f} {unit}"
    return f"{num_bytes} B"


def build_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render a palette-optimized GIF from a captured screen recording.",
        epilog=(
            "Typical flow: capture a short recording in VS Code, then run this helper "
            "with --max-bytes to keep the final asset README-friendly."
        ),
    )
    parser.add_argument("--input", required=True, help="Path to the recorded video file.")
    parser.add_argument("--output", required=True, help="Path to the output GIF file.")
    parser.add_argument(
        "--fps",
        type=positive_int,
        default=12,
        help="Output frame rate for the GIF (default: 12).",
    )
    parser.add_argument(
        "--width",
        type=positive_int,
        default=960,
        help="Target width for the GIF; height is computed automatically (default: 960).",
    )
    parser.add_argument(
        "--max-bytes",
        type=positive_int,
        default=None,
        help="Fail if the rendered GIF exceeds this many bytes.",
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
    if not input_video.is_file():
        print(f"error: input video is not a file: {input_video}", file=sys.stderr)
        return 2

    output_gif = Path(args.output)
    if output_gif.suffix.lower() != ".gif":
        print(f"error: output file must end in .gif: {output_gif}", file=sys.stderr)
        return 2
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
                "-update",
                "1",
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
            shutil.copy2(optimized, output_gif)
            optimized.unlink()

        output_size = output_gif.stat().st_size
        if args.max_bytes is not None and output_size > args.max_bytes:
            print(
                "error: output GIF is "
                f"{format_size(output_size)}, which exceeds the configured limit of "
                f"{format_size(args.max_bytes)}",
                file=sys.stderr,
            )
            return 2

        if args.keep_temp:
            temp_copy = output_gif.with_suffix(".palette.png")
            shutil.copy2(palette, temp_copy)

    print(f"wrote {output_gif} ({format_size(output_gif.stat().st_size)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
