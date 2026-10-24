"""生成钓鱼小游戏的轻像素水域背景（creek/river/jiang/sea）。

目标是四种水域一眼能分清：色温、地貌、主体剪影都不同。
母版 230x100，最近邻放大 4x -> 920x400；无文字、无第三方素材。
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

P = {
    # 小溪：暖绿浅水
    "creek_surface": (176, 226, 188),
    "creek_1": (111, 191, 154),
    "creek_2": (63, 154, 130),
    "creek_3": (42, 116, 104),
    "creek_sand": (217, 192, 137),
    "creek_sand_dark": (185, 154, 98),
    "creek_grass": (79, 139, 79),
    "creek_grass_light": (111, 175, 95),
    "creek_rock": (138, 154, 138),
    "creek_rock_dark": (104, 122, 108),
    "creek_soil": (126, 94, 62),
    "creek_soil_dark": (96, 70, 47),
    "creek_wood": (124, 86, 52),
    "creek_wood_light": (158, 112, 68),
    # 河流：冷蓝 + 灰岩
    "river_surface": (168, 216, 224),
    "river_1": (90, 166, 184),
    "river_2": (46, 127, 150),
    "river_3": (29, 92, 116),
    "river_4": (18, 66, 86),
    "river_rock": (125, 138, 140),
    "river_rock_light": (158, 170, 170),
    "river_rock_dark": (90, 103, 107),
    "river_gravel": (154, 143, 117),
    "river_log": (122, 90, 58),
    "river_log_light": (154, 116, 76),
    # 江面：暗蓝灰 + 船/桥
    "jiang_surface": (127, 166, 176),
    "jiang_1": (63, 111, 128),
    "jiang_2": (36, 79, 99),
    "jiang_3": (20, 53, 69),
    "jiang_4": (11, 34, 49),
    "jiang_silt": (95, 90, 74),
    "jiang_silt_dark": (70, 66, 54),
    "jiang_boat": (43, 42, 40),
    "jiang_boat_light": (72, 68, 62),
    "jiang_rust": (138, 90, 58),
    "jiang_rope": (110, 102, 80),
    # 大海：深蓝 + 珊瑚
    "sea_surface": (143, 208, 232),
    "sea_1": (63, 158, 196),
    "sea_2": (30, 111, 158),
    "sea_3": (15, 74, 120),
    "sea_4": (8, 47, 82),
    "sea_sand": (214, 196, 143),
    "sea_sand_dark": (180, 158, 112),
    "coral": (224, 106, 90),
    "coral_light": (242, 154, 122),
    "coral_pink": (201, 79, 109),
    "kelp": (63, 138, 90),
    "kelp_light": (99, 176, 111),
    "fish_yellow": (255, 209, 102),
    "fish_orange": (255, 138, 91),
    "ray": (170, 232, 238),
}

rng = random.Random(20261023)


def rect(d: ImageDraw.ImageDraw, x0: int, y0: int, x1: int, y1: int, color: tuple) -> None:
    d.rectangle([x0, y0, x1, y1], fill=color)


def hband(d: ImageDraw.ImageDraw, y0: int, y1: int, color: tuple) -> None:
    rect(d, 0, y0, W - 1, y1, color)


def water(d: ImageDraw.ImageDraw, colors: tuple[tuple, ...], bands: tuple[int, ...]) -> None:
    start = 0
    for color, end in zip(colors, bands):
        hband(d, start, end, color)
        start = end + 1


def rays(image: Image.Image, count: int, alpha: int = 40) -> Image.Image:
    overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    for i in range(count):
        x = 8 + i * (W // max(1, count)) + rng.randint(-4, 4)
        width = rng.randint(8, 16)
        d.polygon(
            [(x, 6), (x + width, 6), (x + width + 30, 74), (x + 30, 74)],
            fill=(*P["ray"], alpha),
        )
    return Image.alpha_composite(image.convert("RGBA"), overlay).convert("RGB")


def surface_waves(d: ImageDraw.ImageDraw, color: tuple, y: int = 4) -> None:
    for x in range(0, W, 7):
        if rng.random() < 0.7:
            d.rectangle([x, y, x + rng.randint(2, 5), y], fill=color)


def tiny_fish(d: ImageDraw.ImageDraw, x: int, y: int, color: tuple) -> None:
    d.rectangle([x, y, x + 3, y + 1], fill=color)
    d.point((x - 1, y + 1), fill=color)
    d.point((x + 4, y), fill=color)


def creek() -> Image.Image:
    img = Image.new("RGB", (W, H), P["creek_1"])
    d = ImageDraw.Draw(img)
    # 浅水：水面到 62，下面是沙底
    water(d, (P["creek_surface"], P["creek_1"], P["creek_2"], P["creek_3"]), (7, 25, 44, 62))
    surface_waves(d, P["creek_surface"])
    # 河岸用逐列画法，边缘有自然台阶，不是两块大三角
    for y in range(10, 63):
        left = 11 + int((y - 10) * 0.52) + rng.randint(-1, 1)
        right = W - left
        for x in range(0, left):
            if y < 22:
                color = P["creek_grass"]
            elif y < 50:
                color = P["creek_soil"]
            else:
                color = P["creek_sand_dark"]
            d.point((x, y), fill=color)
        for x in range(right, W):
            if y < 22:
                color = P["creek_grass"]
            elif y < 50:
                color = P["creek_soil"]
            else:
                color = P["creek_sand_dark"]
            d.point((x, y), fill=color)
    # 草皮亮边和土粒
    for y in (20, 21):
        for x in range(0, 30):
            if rng.random() < 0.5:
                d.point((x, y), fill=P["creek_grass_light"])
        for x in range(W - 30, W):
            if rng.random() < 0.5:
                d.point((x, y), fill=P["creek_grass_light"])
    for _ in range(70):
        side = rng.choice((0, 1))
        y = rng.randint(24, 58)
        edge = 11 + int((y - 10) * 0.52)
        x = rng.randint(0, edge - 1) if side == 0 else rng.randint(W - edge, W - 1)
        d.point((x, y), fill=P["creek_soil_dark"])
    # 岸边草丛
    for x, base, h in ((4, 22, 12), (9, 22, 16), (15, 23, 10), (21, 25, 13), (214, 23, 11), (220, 22, 15), (225, 23, 9)):
        for i in range(h):
            d.point((x + (i // 5), base - i), fill=P["creek_grass_light"])
    # 沙底
    hband(d, 63, 99, P["creek_sand"])
    for x in range(0, W, 5):
        d.point((x, 64 + rng.randint(0, 2)), fill=P["creek_sand_dark"])
    # 鹅卵石 + 踏脚石
    for _ in range(24):
        x, y = rng.randint(8, W - 10), rng.randint(68, 96)
        d.ellipse([x, y, x + rng.randint(2, 5), y + 1], fill=P["creek_rock"])
        d.point((x, y), fill=P["creek_rock_dark"])
    for x, y, w in ((84, 80, 10), (101, 84, 9), (118, 87, 8), (136, 92, 7)):
        d.ellipse([x, y, x + w, y + 3], fill=P["creek_rock"])
        d.ellipse([x + 1, y + 1, x + w - 1, y + 2], fill=P["creek_rock_dark"])
        d.point((x + w // 2, y + 2), fill=P["creek_grass"])
    # 岸边水线碎石
    for _ in range(18):
        side = rng.choice((0, 1))
        y = rng.randint(20, 40)
        edge = 11 + int((y - 10) * 0.52)
        x = edge - rng.randint(1, 4) if side == 0 else W - edge + rng.randint(1, 4)
        d.point((x, y), fill=P["creek_rock"])
        d.point((x, y + 1), fill=P["creek_rock_dark"])
    # 水草
    for x in (58, 64, 176, 182, 188):
        for i in range(rng.randint(12, 24)):
            d.point((x + (i // 6) % 2, 96 - i), fill=P["creek_grass"])
    # 浮萍 / 水面
    for x in (70, 78, 146, 154, 162):
        d.ellipse([x, 3, x + 7, 7], fill=P["creek_grass"])
        d.point((x + 3, 2), fill=P["creek_grass_light"])
    # 水面反光
    for x in range(30, 200, 13):
        y = 8 + rng.randint(0, 3)
        d.rectangle([x, y, x + rng.randint(2, 6), y], fill=P["creek_surface"])
    for x, y in ((92, 50), (126, 40), (166, 56)):
        tiny_fish(d, x, y, P["creek_3"])
    for _ in range(16):
        x, y = rng.randint(56, 176), rng.randint(24, 58)
        d.point((x, y), fill=P["creek_surface"])
    return rays(img, 3, 30)


def river() -> Image.Image:
    img = Image.new("RGB", (W, H), P["river_1"])
    d = ImageDraw.Draw(img)
    water(
        d,
        (P["river_surface"], P["river_1"], P["river_2"], P["river_3"], P["river_4"]),
        (6, 24, 46, 66, 82),
    )
    surface_waves(d, (226, 244, 244))
    # 强水流线
    for y in (20, 32, 44, 56):
        for x in range(6, W - 6, 24):
            d.rectangle([x, y, x + rng.randint(8, 14), y], fill=P["river_surface"])
    # 底部砾石
    hband(d, 82, 99, P["river_gravel"])
    for _ in range(30):
        x, y = rng.randint(0, W - 5), rng.randint(84, 97)
        d.rectangle([x, y, x + rng.randint(1, 4), y + 1], fill=P["river_rock_dark"])
    # 大块圆石（主体）
    for x, y, w, h in ((10, 70, 38, 22), (62, 76, 30, 18), (186, 68, 38, 26)):
        d.ellipse([x, y, x + w, y + h], fill=P["river_rock"])
        d.ellipse([x + 2, y + 2, x + w - 3, y + h // 2], fill=P["river_rock_light"])
        d.arc([x, y, x + w, y + h], 180, 330, fill=P["river_rock_dark"])
    # 沉木
    for i in range(48):
        x, y = 40 + i, 86 - i // 5
        rect(d, x, y, x + 1, y + 2, P["river_log"])
        if i % 5 == 0:
            d.point((x, y), fill=P["river_log_light"])
    # 水草
    for x in (172, 177, 182):
        for i in range(rng.randint(8, 16)):
            d.point((x, 80 - i), fill=P["kelp"])
    for x, y in ((128, 40), (142, 52), (100, 62)):
        tiny_fish(d, x, y, P["river_4"])
    return rays(img, 3, 30)


def jiang() -> Image.Image:
    img = Image.new("RGB", (W, H), P["jiang_2"])
    d = ImageDraw.Draw(img)
    water(
        d,
        (P["jiang_surface"], P["jiang_1"], P["jiang_2"], P["jiang_3"], P["jiang_4"]),
        (7, 20, 42, 64, 84),
    )
    # 江面船底：大剪影，一眼认出江面
    d.rectangle([42, 0, 190, 7], fill=P["jiang_boat"])
    d.rectangle([58, 0, 174, 1], fill=P["jiang_boat_light"])
    d.rectangle([120, 0, 136, 3], fill=P["jiang_boat_light"])
    d.rectangle([150, 4, 158, 12], fill=P["jiang_boat"])
    for x in (74, 96, 150):
        d.point((x, 4), fill=P["jiang_rust"])
    # 桥墩
    d.rectangle([204, 0, 218, 34], fill=P["jiang_boat"])
    d.rectangle([204, 0, 218, 2], fill=P["jiang_boat_light"])
    d.rectangle([200, 30, 222, 34], fill=P["jiang_boat_light"])
    # 浑浊水平泥带
    for y in (24, 38, 52, 68):
        for x in range(0, W, 12):
            if rng.random() < 0.6:
                d.rectangle([x, y, x + rng.randint(3, 9), y], fill=P["jiang_1"])
    # 江底淤泥
    hband(d, 84, 99, P["jiang_silt"])
    for x in range(0, W, 4):
        d.point((x, 85 + rng.randint(0, 1)), fill=P["jiang_silt_dark"])
    # 沉箱
    d.rectangle([28, 70, 58, 86], fill=P["jiang_silt_dark"])
    d.rectangle([28, 70, 58, 72], fill=P["jiang_silt"])
    d.rectangle([34, 74, 52, 75], fill=P["jiang_rust"])
    d.rectangle([36, 78, 50, 79], fill=P["jiang_rope"])
    # 旧锚/铁链
    d.rectangle([150, 80, 152, 92], fill=P["jiang_boat_light"])
    d.rectangle([146, 90, 158, 92], fill=P["jiang_boat_light"])
    for i in range(8):
        d.point((146 - i, 92 - i // 2), fill=P["jiang_rope"])
    # 少量水草，显得荒
    for x in (70, 76, 188):
        for i in range(rng.randint(6, 12)):
            d.point((x, 84 - i), fill=P["jiang_3"])
    return rays(img, 2, 18)


def sea() -> Image.Image:
    img = Image.new("RGB", (W, H), P["sea_2"])
    d = ImageDraw.Draw(img)
    water(
        d,
        (P["sea_surface"], P["sea_1"], P["sea_2"], P["sea_3"], P["sea_4"]),
        (6, 22, 44, 68, 86),
    )
    surface_waves(d, (222, 246, 250))
    # 沙底
    hband(d, 80, 99, P["sea_sand"])
    for x in range(0, W, 5):
        d.point((x, 81 + rng.randint(0, 2)), fill=P["sea_sand_dark"])
    # 珊瑚丛：粗枝 + 亮色，主体
    for base_x, base_y, height in ((22, 96, 28), (30, 96, 22), (186, 96, 30), (196, 96, 20)):
        for i in range(height):
            x = base_x + (i // 4) * 3
            d.point((x, base_y - i), fill=P["coral"])
            if i % 5 == 0:
                d.point((x - 1, base_y - i), fill=P["coral_light"])
                d.point((x + 1, base_y - i), fill=P["coral_pink"])
        for b in range(4):
            bx = base_x + b * 4
            by = base_y - 10 - b * 3
            d.rectangle([bx, by, bx + 2, by + 2], fill=P["coral_light"])
    # 海葵
    for x in (54, 60, 66):
        for i in range(10):
            d.point((x + i % 3 - 1, 90 - i), fill=P["coral_pink"])
    # 海草森林
    for x in (82, 88, 94, 152, 158, 164, 170):
        for i in range(rng.randint(20, 42)):
            d.point((x + (i // 8) % 2, 96 - i), fill=P["kelp"])
            if i % 7 == 0:
                d.point((x + 1, 96 - i), fill=P["kelp_light"])
    # 海星
    sx, sy = 120, 92
    d.rectangle([sx, sy, sx + 5, sy + 2], fill=P["coral"])
    d.rectangle([sx + 1, sy - 2, sx + 4, sy + 5], fill=P["coral"])
    d.point((sx + 2, sy + 1), fill=P["coral_light"])
    # 热带鱼群（亮色，和蓝色强对比）
    for x, y in ((100, 30), (108, 36), (116, 30), (124, 40), (132, 34), (140, 42)):
        d.rectangle([x, y, x + 4, y + 2], fill=P["fish_yellow"])
        d.point((x - 1, y + 2), fill=P["fish_orange"])
        d.point((x + 5, y + 1), fill=P["fish_orange"])
    # 一只海龟剪影
    d.ellipse([64, 44, 80, 54], fill=P["sea_4"])
    d.rectangle([60, 47, 64, 49], fill=P["sea_4"])
    d.rectangle([80, 47, 84, 49], fill=P["sea_4"])
    d.point((72, 46), fill=P["kelp_light"])
    return rays(img, 5, 46)


def build(name: str, image: Image.Image) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    big = image.resize((W * SCALE, H * SCALE), Image.Resampling.NEAREST)
    path = OUT / f"bg_{name}.png"
    big.save(path, optimize=True)
    print(path, big.size, path.stat().st_size)


def main() -> None:
    for name, image in (
        ("creek", creek()),
        ("river", river()),
        ("jiang", jiang()),
        ("sea", sea()),
    ):
        build(name, image)
    manifest = {
        "version": 2,
        "source": "generated/pil",
        "license": "in-house",
        "style": "light pixel, underwater cross-section, distinct per water body",
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
