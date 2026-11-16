# /// script
# requires-python = ">=3.10"
# dependencies = ["pillow>=10.0"]
# ///
"""Render the README's assets from the app itself: a still and an animated GIF.

    uv run scripts/make-assets.py            # assets/screenshot.png + assets/clock.gif
    uv run scripts/make-assets.py --still    # only the still

Everything comes out of Rune's headless renderer, so the pictures are exactly what the
window draws — same shapes, same frame times — with no desktop, capture card or window
manager in the loop. `--scene hero` pins the displayed time to 10:09:36, which is why the
still is reproducible frame for frame.
"""

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets"
FRAMES = ROOT / "target" / "frames"

WINDOW = "820x900"
SCALE = 2


def run(*args: str) -> None:
    cmd = ["cargo", "run", "--quiet"]
    if shutil.which("cargo") is None:
        sys.exit("cargo not found")
    subprocess.run([*cmd, "--", *args], cwd=ROOT, check=True)


def crop_clock(im: Image.Image) -> Image.Image:
    """Crop to the case plus a little breathing room, dropping the caption."""
    w, h = im.size
    side = int(min(w - 24, h - 176) * 0.99)
    left = (w - side) // 2
    top = int((h - 176 - side) / 2)
    return im.crop((left, top, left + side, top + side))


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--still", action="store_true")
    ap.add_argument("--fps", type=int, default=30)
    ap.add_argument("--frames", type=int, default=60)
    ap.add_argument("--gif-width", type=int, default=560)
    ap.add_argument("--colors", type=int, default=160)
    args = ap.parse_args()

    OUT.mkdir(exist_ok=True)

    print("rendering the still ...")
    run(
        "--scene",
        "hero",
        "--at",
        "0",
        "--snap",
        str(OUT / "screenshot.png"),
        "--size",
        WINDOW,
        "--scale",
        str(SCALE),
    )

    if args.still:
        return

    print(f"rendering {args.frames} frames at {args.fps} fps ...")
    if FRAMES.exists():
        shutil.rmtree(FRAMES)
    FRAMES.mkdir(parents=True)
    run(
        "--scene",
        "hero",
        "--out",
        str(FRAMES),
        "--frames",
        str(args.frames),
        "--fps",
        str(args.fps),
        "--size",
        WINDOW,
        "--scale",
        "1",
    )

    print("assembling the gif ...")
    frames = sorted(FRAMES.glob("frame_*.png"))
    if not frames:
        sys.exit("no frames were dumped")
    ims = []
    for f in frames:
        im = crop_clock(Image.open(f).convert("RGB"))
        im = im.resize((args.gif_width, args.gif_width), Image.LANCZOS)
        # Quantise up front, with one palette for the lot: the alternative is Pillow inventing
        # a palette per frame, which triples the file.
        ims.append(
            im.quantize(
                args.colors, method=Image.MEDIANCUT, dither=Image.FLOYDSTEINBERG
            )
        )

    duration = round(1000 / args.fps)
    ims[0].save(
        OUT / "clock.gif",
        format="GIF",
        save_all=True,
        append_images=ims[1:],
        duration=duration,
        loop=0,
        optimize=True,
        disposal=1,
    )
    kb = (OUT / "clock.gif").stat().st_size // 1024
    still_kb = (OUT / "screenshot.png").stat().st_size // 1024
    print(f"assets/screenshot.png  {still_kb} KB")
    print(f"assets/clock.gif       {kb} KB, {len(ims)} frames at {args.fps} fps")


if __name__ == "__main__":
    main()
