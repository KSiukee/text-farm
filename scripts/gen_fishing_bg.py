"""生成钓鱼小游戏的轻像素水域背景（creek/river/jiang/sea）。

母版 230x100，最近邻放大 4x -> 920x400；无文字、无第三方素材、可重复生成。
运行：python scripts/gen_fishing_bg.py
"""

from __future__ import annotations

import json
import random
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "app" / "frontend" / "public" / "fishing"
W, H, SCALE = 230, 100, 4

PALETTE = {
    "surface": (137, 210, 196),
    "surface2": (99, 180, 174),
    "shallow": (58, 140, 140),
    "mid": (35, 104, 116),
    "deep": (20, 73, 88),
    "abyss": (13, 48, 63),
    "sand": (196, 174, 120),
    "sand_dark": (158, 133, 88),
    "stone": (104, 118, 120),
    "stone_dark": (76, 88, 92),
    "silt": (96, 91, 76),
    "plant": (74, 128, 86),
    "plant_dark": (52, 96, 66),
    "coral": (196, 104, 92),
    "coral_light": (224, 144, 126),
    "kelp": (67, 116, 72),
    "kelp_light": (94, 146, 90),
    "ray": (120, 196, 194),
    "boat": (54, 50, 47),
    "boat_light": (84, 78, 68),
}

rng = random.Random(20261023)


def rect(d: ImageDraw.ImageDraw, x0: int, y0: int, x1: int, y1: int, color: str) -> None:
    d.rectangle([x0, y0, x1, y1], fill=PALETTE[color])


def edge_dither(
    d: ImageDraw.ImageDraw, y0: int, y1: int, top: str, bottom: str, step: int = 2
) -> None:
    for y in range(y0, y1 + 1):
        for x in range(0, W, step):
            if (x // step + y // step) % 2 == 0:
                d.point((x, y), fill=PALETTE[bottom])
            else:
                d.point((x, y), fill=PALETTE[top])


def water_base(d: ImageDraw.ImageDraw, shallow: str, mid: str, deep: str) -> None:
    rect(d, 0, 0, W - 1, 6, "surface2")
    rect(d, 0, 7, W - 1, 17, shallow)
    rect(d, 0, 18, W - 1, 30, mid)
    rect(d, 0, 31, W - 1, 44, mid)
    rect(d, 0, 45, W - 1, 62, deep)
    rect(d, 0, 63, W - 1, 79, "abyss")

    # 水中悬浮微粒
    for _ in range(42):
        x = rng.randint(2, W - 3)
        y = rng.randint(10, 72)
        d.point((x, y), fill=PALETTE["surface2"])


def light_rays(image: Image.Image, count: int, alpha: int = 34) -> Image.Image:
    """低透明度斜光柱；只在背景上叠一层，不盖住鱼和框。"""
    overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    for i in range(count):
        x = 14 + i * 28 + rng.randint(-4, 4)
        width = rng.randint(7, 13)
        d.polygon(
            [
                (x, 8),
                (x + width, 8),
                (x + width + 26, 66),
                (x + 26, 66),
            ],
            fill=(*PALETTE["ray"], alpha),
        )
    return Image.alpha_composite(image.convert("RGBA"), overlay).convert("RGB")


def floor(d: ImageDraw.ImageDraw, color: str = "sand", top: int = 78) -> None:
    rect(d, 0, top, W - 1, H - 1, color)
    for x in range(0, W, 3):
        y = top + rng.randint(0, 2)
        d.point((x, y), fill=PALETTE["sand_dark"])
    edge_dither(d, H - 9, H - 1, color, "sand_dark")


def pebbles(d: ImageDraw.ImageDraw, count: int, base: str = "stone") -> None:
    for _ in range(count):
        x = rng.randint(4, W - 8)
        y = rng.randint(80, H - 5)
        w = rng.randint(2, 5)
        h = rng.randint(1, 2)
        d.ellipse([x, y, x + w, y + h], fill=PALETTE[base])
        d.point((x, y), fill=PALETTE["stone_dark"])


def reed(d: ImageDraw.ImageDraw, x: int, height: int) -> None:
    for i in range(height):
        d.point((x, H - 8 - i), fill=PALETTE["plant"])
    d.rectangle([x - 1, H - 10 - height, x + 1, H - 7 - height], fill=PALETTE["plant_dark"])


def creek() -> Image.Image:
    img = Image.new("RGB", (W, H), PALETTE["shallow"])
    d = ImageDraw.Draw(img)
    water_base(d, "shallow", "mid", "deep")
    img = light_rays(img, 4)
    d = ImageDraw.Draw(img)
    floor(d, "sand", 79)
    pebbles(d, 22, "stone")
    # 左右浅滩和水草
    for x in (10, 16, 21, 204, 211, 218):
        reed(d, x, rng.randint(12, 24))
    # 水面浮萍
    for x in (38, 44, 150, 157, 163):
        d.ellipse([x, 4, x + 6, 7], fill=PALETTE["plant"])
        d.point((x + 2, 3), fill=PALETTE["plant_dark"])
    # 几条小鱼影（不抢主体）
    for x, y in ((82, 56), (128, 44), (174, 63)):
        d.rectangle([x, y, x + 4, y + 1], fill=PALETTE["deep"])
        d.point((x - 1, y + 1), fill=PALETTE["deep"])
    return img


def river() -> Image.Image:
    img = Image.new("RGB", (W, H), PALETTE["mid"])
    d = ImageDraw.Draw(img)
    water_base(d, "shallow", "mid", "deep")
    img = light_rays(img, 3)
    d = ImageDraw.Draw(img)
    floor(d, "stone_dark", 81)
    pebbles(d, 18, "stone")
    # 沉木
    for i in range(38):
        x = 46 + i
        y = 84 - i // 4
        d.rectangle([x, y, x + 1, y + 2], fill=PALETTE["silt"])
        d.point((x, y), fill=PALETTE["sand_dark"])
    # 水流线
    for y in (26, 38, 52):
        for x in range(12, W - 12, 18):
            d.rectangle([x, y, x + 9, y], fill=PALETTE["surface2"])
    # 右岸石块
    for x, y, w, h in ((196, 72, 14, 9), (210, 76, 12, 8), (188, 80, 9, 6)):
        d.rectangle([x, y, x + w, y + h], fill=PALETTE["stone"])
        d.rectangle([x, y, x + w, y + 1], fill=PALETTE["stone_dark"])
    return img


def jiang() -> Image.Image:
    img = Image.new("RGB", (W, H), PALETTE["deep"])
    d = ImageDraw.Draw(img)
    water_base(d, "mid", "deep", "abyss")
    img = light_rays(img, 2)
    d = ImageDraw.Draw(img)
    # 江面船底剪影
    d.rectangle([62, 1, 168, 5], fill=PALETTE["boat"])
    d.rectangle([74, 0, 156, 1], fill=PALETTE["boat_light"])
    d.rectangle([150, 3, 154, 8], fill=PALETTE["boat"])
    floor(d, "silt", 84)
    pebbles(d, 12, "stone_dark")
    # 沉箱
    d.rectangle([34, 72, 58, 84], fill=PALETTE["silt"])
    d.rectangle([34, 72, 58, 74], fill=PALETTE["stone"])
    d.rectangle([40, 76, 52, 77], fill=PALETTE["stone_dark"])
    # 水草
    for x in (26, 30, 196, 201, 206):
        reed(d, x, rng.randint(10, 20))
    # 远处暗涌
    for y in (40, 54, 68):
        for x in range(0, W, 26):
            d.rectangle([x + 6, y, x + 18, y], fill=PALETTE["abyss"])
    return img


def sea() -> Image.Image:
    img = Image.new("RGB", (W, H), PALETTE["deep"])
    d = ImageDraw.Draw(img)
    water_base(d, "mid", "deep", "abyss")
    img = light_rays(img, 4)
    d = ImageDraw.Draw(img)
    floor(d, "sand", 82)
    pebbles(d, 16, "stone")
    # 珊瑚
    for base_x, base_y, height in ((32, 92, 18), (42, 92, 12), (190, 92, 16), (200, 92, 10)):
        for i in range(height):
            x = base_x + (i // 3) * 2
            d.point((x, base_y - i), fill=PALETTE["coral"])
            if i % 4 == 0:
                d.point((x - 1, base_y - i), fill=PALETTE["coral_light"])
                d.point((x + 1, base_y - i), fill=PALETTE["coral_light"])
    # 海草
    for x in (66, 72, 150, 158, 166):
        for i in range(rng.randint(12, 26)):
            d.point((x + (i // 6) % 2, H - 6 - i), fill=PALETTE["kelp"])
    # 海星
    sx, sy = 112, 92
    d.rectangle([sx, sy, sx + 4, sy + 2], fill=PALETTE["coral"])
    d.rectangle([sx + 1, sy - 2, sx + 3, sy + 4], fill=PALETTE["coral"])
    # 鱼群剪影
    for x, y in ((92, 36), (98, 40), (104, 36), (170, 52), (176, 56)):
        d.rectangle([x, y, x + 3, y + 1], fill=PALETTE["abyss"])
    return img


def build(name: str, image: Image.Image) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    big = image.resize((W * SCALE, H * SCALE), Image.Resampling.NEAREST)
    path = OUT / f"bg_{name}.png"
    big.save(path, optimize=True)
    print(path, big.size)


def main() -> None:
    for name, image in (
        ("creek", creek()),
        ("river", river()),
        ("jiang", jiang()),
        ("sea", sea()),
    ):
        build(name, image)
    manifest = {
        "version": 1,
        "source": "generated/pil",
        "license": "in-house",
        "style": "light pixel, underwater cross-section",
        "master": [W, H],
        "scale": SCALE,
        "assets": [
            {"id": f"fishing_bg_{name}", "file": f"bg_{name}.png", "size": [W * SCALE, H * SCALE]}
            for name in ("creek", "river", "jiang", "sea")
        ],
    }
    (OUT / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8"
    )


if __name__ == "__main__":
    main()
