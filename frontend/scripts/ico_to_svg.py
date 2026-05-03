"""Raster ICO -> SVG wrapper (embedded PNG). Run: python scripts/ico_to_svg.py"""
from __future__ import annotations

import base64
from io import BytesIO
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ICO = ROOT / "public" / "favicon.ico"
OUT_PUBLIC = ROOT / "public" / "favicon.svg"
OUT_SRC = ROOT / "src" / "assets" / "favicon.svg"


def main() -> None:
    im = Image.open(ICO)
    if getattr(im, "n_frames", 1) > 1:
        sizes: list[tuple[int, int, tuple[int, int]]] = []
        for i in range(im.n_frames):
            im.seek(i)
            w, h = im.size
            sizes.append((w * h, i, (w, h)))
        sizes.sort(reverse=True)
        im.seek(sizes[0][1])
    else:
        im.seek(0)

    im = im.convert("RGBA")
    max_side = 64
    w0, h0 = im.size
    if max(w0, h0) > max_side:
        if w0 >= h0:
            nw, nh = max_side, max(1, round(h0 * max_side / w0))
        else:
            nh, nw = max_side, max(1, round(w0 * max_side / h0))
        im = im.resize((nw, nh), Image.Resampling.LANCZOS)

    buf = BytesIO()
    im.save(buf, format="PNG")
    b64 = base64.b64encode(buf.getvalue()).decode("ascii")
    w, h = im.size

    svg = f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <image width="{w}" height="{h}" xlink:href="data:image/png;base64,{b64}"/>
</svg>
"""
    for path in (OUT_PUBLIC, OUT_SRC):
        path.write_text(svg, encoding="utf-8")
        print(f"Wrote {path} ({w}x{h}, {len(svg)} chars)")


if __name__ == "__main__":
    main()
