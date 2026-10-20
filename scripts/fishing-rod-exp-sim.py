#!/usr/bin/env python3
"""高级鱼竿经验加成模拟（只读分析，不改游戏数值）。

从钓鱼 9 级（碳素已解锁）起，河流、保养状态、普通饵、主动钓获；
比较竹竿 / 玻璃钢 / 碳素，以及玻璃钢+碳素不同经验加成下，
升到 15 级（大海）和 20 级要多少条/天。

注意：成功率沿用鱼竿经济模拟的假设模型，不是真实 2D 追鱼。
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FISHING_RS = ROOT / "crates/farm-core/src/fishing.rs"
OUT = ROOT / "docs/高级竿经验加成模拟.md"
START_LEVEL = 9
CATCHES_PER_DAY = (30, 60)

RODS = {
    "竹竿": {"frame": 18, "q": (45, 85), "w": 1, "success": 0.625},
    "玻璃钢": {"frame": 15, "q": (30, 75), "w": 2, "success": 0.506},
    "碳素": {"frame": 13, "q": (15, 60), "w": 3, "success": 0.426},
}

PLANS = {
    "B0 无加成": {"玻璃钢": 0, "碳素": 0},
    "B1 温和": {"玻璃钢": 10, "碳素": 20},
    "B2 中": {"玻璃钢": 20, "碳素": 40},
    "B3 强": {"玻璃钢": 30, "碳素": 60},
    "B4 很猛": {"玻璃钢": 50, "碳素": 100},
}

PERSONALITY = [
    ("Lazy", 0.25, 1.0),
    ("Normal", 0.40, 0.92),
    ("Active", 0.25, 0.78),
    ("Fierce", 0.10, 0.60),
]


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
        if fields["spot"] == "FishingSpot::River":
            fields["weight"] = int(fields["weight"])
            fields["difficulty"] = int(fields["difficulty"])
            fish.append(fields)
    return fish


def exp_need(level: int) -> int:
    return 100 + level * 100


def level_casts(start: int, target: int, exp_per_cast: float) -> int:
    if target <= start:
        return 0
    lv = start
    exp = 0.0
    casts = 0
    while lv < target:
        exp += exp_per_cast
        casts += 1
        while lv < target and exp >= exp_need(lv):
            exp -= exp_need(lv)
            lv += 1
    return casts


def exp_catch(fish, rod, q: int, w: int) -> int:
    base = {
        "FishRarity::Common": 3,
        "FishRarity::Uncommon": 5,
        "FishRarity::Rare": 8,
        "FishRarity::King": 40,
    }[fish["rarity"]]
    return base + max(0, q - 1) + w // 10


def evaluate(fish, name: str, bonus_pct: int) -> dict:
    rod = RODS[name]
    total_w = sum(f["weight"] for f in fish)
    weighted_success = 0.0
    weighted_exp = 0.0
    for f in fish:
        for _, p_weight, p_factor in PERSONALITY:
            diff = 1.0 - 0.08 * (f["difficulty"] - 1)
            control = (rod["frame"] / 20.0) ** 1.2
            king = 0.60 if f["rarity"] == "FishRarity::King" else 1.0
            success = 0.95 * control * diff * p_factor * king
            success = max(0.03, min(0.95, success))
            # 品质/重量按钓获后的分布取期望；不同 roll 均匀
            for q_roll in range(100):
                q = 1 if q_roll < rod["q"][0] else (2 if q_roll < rod["q"][1] else 3)
                q = min(3, q + 1)
                for w_roll in range(20):
                    base_w = 10 + w_roll % 6 if f["rarity"] == "FishRarity::King" else 8 + w_roll % 5
                    w = min(20, base_w + rod["w"] + 1 + max(0, q - 1))
                    exp = exp_catch(f, rod, q, w) * (100 + bonus_pct) // 100
                    weighted_exp += f["weight"] * p_weight * success * exp
            weighted_success += f["weight"] * p_weight * success
    # 每个 (fish, personality) 的 q/w 组合是 100*20，weighted_exp 已含重复
    weighted_exp /= 100 * 20
    avg_success = weighted_success / total_w
    exp_per_catch = weighted_exp / weighted_success * total_w / total_w  # 条件钓获经验
    # 上面的 exp 已乘 weight/p/success；除以成功加权得到每条
    exp_per_catch = weighted_exp / weighted_success
    exp_per_cast = avg_success * exp_per_catch
    return {
        "success": avg_success * 100,
        "exp_catch": exp_per_catch,
        "exp_cast": exp_per_cast,
        "to15": level_casts(START_LEVEL, 15, exp_per_cast),
        "to20": level_casts(START_LEVEL, 20, exp_per_cast),
    }


def main() -> None:
    fish = parse_fish()
    print(f"river fish={len(fish)}")
    lines = [
        "# 高级竿经验加成模拟（只读分析）",
        "",
        "> 2026-10-21 · 脚本：`scripts/fishing-rod-exp-sim.py`；**未改游戏数值**。",
        "",
        "## 口径",
        "",
        f"- 从钓鱼 **{START_LEVEL} 级、0 经验**起；河流、保养状态、普通饵、主动钓获。",
        "- 成功率/每条经验沿用鱼竿经济模拟的假设（竹 .625 / 玻 .506 / 碳 .426），"
        "品质/重量按各竿阈值 + 主动 +1 计算；不是真实 2D 追鱼。",
        "- 天数按 30 / 60 次抛竿/天；不含失败、自动钓鱼、鱼饵成本。",
        "",
        "## 结果",
        "",
        "| 方案 | 竿 | 上鱼率 | 每条经验 | 每次经验 | 到15级（条/天） | 到20级（条/天） |",
        "|---|---|---:|---:|---:|---:|---:|",
    ]
    base = evaluate(fish, "竹竿", 0)
    for plan, bonuses in PLANS.items():
        for name in ("竹竿", "玻璃钢", "碳素"):
            bonus = 0 if name == "竹竿" else bonuses[name]
            r = evaluate(fish, name, bonus)
            lines.append(
                f"| {plan} | {name} | {r['success']:.1f}% | {r['exp_catch']:.2f} | "
                f"{r['exp_cast']:.2f} | {r['to15']:,} 条（{max(1, round(r['to15']/30))}/{max(1, round(r['to15']/60))} 天） | "
                f"{r['to20']:,} 条（{max(1, round(r['to20']/30))}/{max(1, round(r['to20']/60))} 天） |"
            )
        lines.append("")

    lines += [
        "## 先看结论",
        "",
        f"- 竹竿从 9 级到 15 级要 **{base['to15']:,} 条**（{max(1, round(base['to15']/30))}~{max(1, round(base['to15']/60))} 天），到 20 级 **{base['to20']:,} 条**。",
        "- 高级竿框小、上鱼率低，但品质/重量更高；经验加成要能补上上鱼率的差距，才值得用。",
        "- 看表里哪一档让玻璃钢/碳素的“到15级”明显快过竹竿，再选；如果加成后仍更慢，说明加成不够或高竿只适合挂机。",
        "- 模拟只算经验，不含卖鱼收益；实际还要看 A2 售价/金色。",
        "",
        "## 复跑",
        "",
        "```powershell",
        "python scripts/fishing-rod-exp-sim.py",
        "```",
        "",
    ]
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
