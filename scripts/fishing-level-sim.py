#!/usr/bin/env python3
"""鱼竿解锁等级模拟（只读分析，不改游戏数值）。

从钓鱼 8 级、0 经验起，按河流平均每条鱼的经验，估算升到
玻璃钢/碳素解锁等级要多少条鱼、多少天。
当前：竹 5 / 玻璃钢 12 / 碳素 20。
候选：竹 4 / 玻璃钢 7 / 碳素 9。
"""
from __future__ import annotations

import random
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FISHING_RS = ROOT / "crates/farm-core/src/fishing.rs"
OUT = ROOT / "docs/鱼竿解锁等级模拟.md"
START_LEVEL = 8
TRIALS = 20_000
CATCHES_PER_DAY = (30, 60)


def parse_fish():
    text = FISHING_RS.read_text(encoding="utf-8")
    starts = [m.start() for m in re.finditer(r'^        id: "([a-z_]+)",$', text, re.M)]
    fish = []
    for pos in starts:
        end = text.find("\n    },", pos)
        block = text[pos:end]
        fields = {}
        for key in ("id", "spot", "rarity", "difficulty", "weight"):
            m = re.search(rf"{key}:\s*([^,\n]+)", block)
            fields[key] = m.group(1).strip().strip('"') if m else ""
        fish.append(fields)
    return [f for f in fish if f["spot"] == "FishingSpot::River"]


def exp_need(level: int) -> int:
    return 100 + level * 100


def exp_for(rarity: str, quality: int, weight: int) -> int:
    base = {
        "FishRarity::Common": 3,
        "FishRarity::Uncommon": 5,
        "FishRarity::Rare": 8,
        "FishRarity::King": 40,
    }[rarity]
    return base + max(0, quality - 1) + weight // 10


def average_exp(pool, rng: random.Random) -> float:
    total = 0
    for _ in range(TRIALS):
        roll = rng.randrange(1000)
        acc = 0
        fish = pool[-1]
        for f in pool:
            acc += int(f["weight"])
            if roll < acc:
                fish = f
                break
        # 竹竿品质阈值 45/85，主动 +1 品质、+1 重量
        q_roll = rng.randrange(100)
        q = 1 if q_roll < 45 else (2 if q_roll < 85 else 3)
        q = min(3, q + 1)
        w = (10 + rng.randrange(6) if fish["rarity"] == "FishRarity::King" else 8 + rng.randrange(5))
        w = min(20, w + 1 + 1 + max(0, q - 1))
        total += exp_for(fish["rarity"], q, w)
    return total / TRIALS


def catches_to(level: int, target: int, avg: float) -> tuple[int, int, int]:
    """从 level 升到 target 所需条数与天数（30/60 条/天）。"""
    if target <= level:
        return 0, 0, 0
    exp = 0
    lv = level
    catches = 0
    while lv < target:
        exp += avg
        catches += 1
        while lv < target and exp >= exp_need(lv):
            exp -= exp_need(lv)
            lv += 1
    return catches, max(1, round(catches / CATCHES_PER_DAY[0])), max(1, round(catches / CATCHES_PER_DAY[1]))


def main() -> None:
    pool = parse_fish()
    rng = random.Random(20261020)
    avg = average_exp(pool, rng)
    print(f"river fish={len(pool)} avg_exp={avg:.2f}")
    current = {"竹竿": 5, "玻璃钢": 12, "碳素": 20}
    proposed = {"竹竿": 4, "玻璃钢": 7, "碳素": 9}
    lines = [
        "# 鱼竿解锁等级模拟（只读分析）",
        "",
        "> 2026-10-20 · 脚本：`scripts/fishing-level-sim.py`；**未改游戏数值**。",
        "",
        "## 口径",
        "",
        f"- 从钓鱼 **{START_LEVEL} 级、0 经验**起，河流、竹竿、普通饵、主动钓获。",
        f"- 平均每条鱼经验 **{avg:.2f}**（{TRIALS:,} 次采样；品质/重量/稀有度按当前公式）。",
        "- 每天按 30 / 60 条鱼估算；不含失败、自动钓鱼、鱼饵/金币。",
        "",
        "## 结果",
        "",
        "| 鱼竿 | 当前解锁 | 当前要多少条（30/60 条/天） | 候选解锁 | 候选要多少条（30/60 条/天） |",
        "|---|---:|---:|---:|---:|",
    ]
    for name in ("竹竿", "玻璃钢", "碳素"):
        c = catches_to(START_LEVEL, current[name], avg)
        p = catches_to(START_LEVEL, proposed[name], avg)
        lines.append(
            f"| {name} | {current[name]} 级 | {c[0]:,} 条（{c[1]}/{c[2]} 天） | "
            f"{proposed[name]} 级 | {p[0]:,} 条（{p[1]}/{p[2]} 天） |"
        )
    lines += [
        "",
        "## 先看结论",
        "",
        f"- 玻璃钢从 12 级降到 7 级：8 级时**已经解锁**，不用再刷。",
        f"- 碳素从 20 级降到 9 级：从 8 级只需 **{catches_to(START_LEVEL, 9, avg)[0]:,} 条**，"
        f"约 {catches_to(START_LEVEL, 9, avg)[1]}~{catches_to(START_LEVEL, 9, avg)[2]} 天；"
        f"当前 20 级要 {catches_to(START_LEVEL, 20, avg)[0]:,} 条，"
        f"约 {catches_to(START_LEVEL, 20, avg)[1]}~{catches_to(START_LEVEL, 20, avg)[2]} 天。",
        "- 这只是等级门槛；玻璃钢 12,000、碳素 40,000 的购买价不变，仍要攒钱。",
        "- 候选值（竹 4 / 玻 7 / 碳 9）只是建议，可以再压/再抬。",
        "",
        "## 复跑",
        "",
        "```powershell",
        "python scripts/fishing-level-sim.py",
        "```",
        "",
    ]
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
