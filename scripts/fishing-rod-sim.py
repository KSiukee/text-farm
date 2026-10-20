#!/usr/bin/env python3
"""鱼竿长期收益模拟（只读分析，不改游戏数值）。

对比：竹竿不保养 / 竹竿保养 / 玻璃钢保养 / 碳素保养 / 木竿不保养。
假设：
- 河流，60 次主动抛竿/天，90 天 = 5400 次；每档跑多个随机种子取平均。
- 成功率 = 0.95 × 框控制力 × 磨损系数 × 难度系数 × 性格系数 × 鱼王系数。
  框控制力 = (frame_pct/20)^1.2；磨损 0.85；难度 1-0.08*(d-1)；
  性格 lazy 1.0 / normal .92 / active .78 / fierce .60；鱼王额外 ×0.60。
  这只是模型假设，不是游戏真实成功率；用于比较相对差异。
- 品质/重量/价格用 farm-core 的公式；主动品质+1/重量+1 生效。
- 收入只算卖鱼；经验/金色图鉴不算钱（金色没有售价加成）。
- 保养按每 30 条鱼一次，费用 300/900/3000/9000；不保养一直顶着 62% 加速度。
- 不含鱼竿购买价、自动钓鱼、宠物/技能/鱼饵品质加成；普通饵。
"""
from __future__ import annotations

import math
import random
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FISHING_RS = ROOT / "crates/farm-core/src/fishing.rs"
OUT = ROOT / "docs/鱼竿长期收益模拟.md"
CASTS = 5400
SEEDS = 20
BAIT_PRICE = 10
WEAR_MAX = 30


@dataclass(frozen=True)
class Fish:
    id: str
    name: str
    spot: str
    rarity: str
    price: int
    difficulty: int
    weight: int


@dataclass(frozen=True)
class Rod:
    key: str
    name: str
    frame: int
    maintenance: int
    weight_bonus: int
    quality_thresholds: tuple[int, int]
    sell_bonus_pct: int = 0
    bait_save_pct: int = 0


RODS = {
    "wood": Rod("wood", "木竿", 20, 300, 0, (60, 95)),
    "bamboo": Rod("bamboo", "竹竿", 18, 900, 1, (45, 85)),
    "glass": Rod("glass", "玻璃钢", 15, 3000, 2, (30, 75)),
    "carbon": Rod("carbon", "碳素", 13, 9000, 3, (15, 60)),
}

# A 路线候选：售价加成 / 省饵；经验、金色率不进入利润。
A_PLANS = {
    "A0 无加成": {"wood": (0, 0), "bamboo": (0, 0), "glass": (0, 0), "carbon": (0, 0)},
    "A1 温和": {"wood": (0, 0), "bamboo": (5, 0), "glass": (10, 5), "carbon": (15, 20)},
    "A2 中强": {"wood": (0, 0), "bamboo": (10, 0), "glass": (25, 10), "carbon": (40, 20)},
    "A3 强力": {"wood": (0, 0), "bamboo": (20, 0), "glass": (50, 15), "carbon": (80, 25)},
}


def parse_fish() -> list[Fish]:
    text = FISHING_RS.read_text(encoding="utf-8")
    starts = [m.start() for m in re.finditer(r'^        id: "([a-z_]+)",$', text, re.M)]
    fish: list[Fish] = []
    for pos in starts:
        end = text.find("\n    },", pos)
        block = text[pos:end]
        fields: dict[str, str] = {}
        for key in ("id", "name", "spot", "rarity", "price", "difficulty", "weight"):
            m = re.search(rf"{key}:\s*([^,\n]+)", block)
            if not m:
                raise SystemExit(f"parse {key} failed near {block[:80]}")
            fields[key] = m.group(1).strip().strip('"')
        fish.append(
            Fish(
                id=fields["id"],
                name=fields["name"],
                spot=fields["spot"].replace("FishingSpot::", ""),
                rarity=fields["rarity"].replace("FishRarity::", ""),
                price=int(fields["price"]),
                difficulty=int(fields["difficulty"]),
                weight=int(fields["weight"]),
            )
        )
    if len(fish) != 35:
        raise SystemExit(f"expected 35 fish, got {len(fish)}")
    for spot in ("Creek", "River", "Jiang", "Sea"):
        total = sum(f.weight for f in fish if f.spot == spot)
        if total != 1000:
            raise SystemExit(f"{spot} weight sum {total}")
    return fish


def pick_weighted(rng: random.Random, pool: list[Fish]) -> Fish:
    roll = rng.randrange(1000)
    acc = 0
    for f in pool:
        acc += f.weight
        if roll < acc:
            return f
    return pool[-1]


def quality_for(rod: Rod, roll: int, bait_bonus: int = 0) -> int:
    p = max(0, roll % 100 - bait_bonus)
    lo, hi = rod.quality_thresholds
    if p < lo:
        return 1
    if p < hi:
        return 2
    return 3


def weight_for(rod: Rod, fish: Fish, roll: int, quality: int) -> int:
    base = 10 + roll % 6 if fish.rarity == "King" else 8 + roll % 5
    return min(20, base + rod.weight_bonus + max(0, quality - 1))


def market_pct(roll: int) -> int:
    return 90 + roll % 21


def price_for(fish: Fish, weight: int, quality: int, market: int) -> int:
    factor = {3: 160, 2: 125}.get(quality, 100)
    return max(1, fish.price * weight * factor * market // 100_000)


def success_rate(rod: Rod, worn: bool, fish: Fish, personality: str) -> float:
    control = (rod.frame / 20.0) ** 1.2 * (0.85 if worn else 1.0)
    diff = 1.0 - 0.08 * (fish.difficulty - 1)
    pers = {"Lazy": 1.0, "Normal": 0.92, "Active": 0.78, "Fierce": 0.60}[personality]
    king = 0.60 if fish.rarity == "King" else 1.0
    return max(0.03, min(0.95, 0.95 * control * diff * pers * king))


def run(pool: list[Fish], rod: Rod, maintain: bool, plan: dict[str, tuple[int, int]]) -> dict:
    sell_bonus, bait_save = plan[rod.key]
    totals = {
        "casts": 0,
        "catches": 0,
        "gross": 0,
        "bait": 0,
        "maint": 0,
        "golden": 0,
    }
    for seed in range(SEEDS):
        rng = random.Random(10_000 + seed)
        wear = 0
        for _ in range(CASTS):
            totals["casts"] += 1
            if rng.randrange(100) >= bait_save:
                totals["bait"] += BAIT_PRICE
            fish = pick_weighted(rng, pool)
            personality = "Fierce" if fish.rarity == "King" else rng.choices(
                ["Lazy", "Normal", "Active", "Fierce"], weights=[25, 40, 25, 10]
            )[0]
            worn = not maintain and wear >= WEAR_MAX
            if rng.random() >= success_rate(rod, worn, fish, personality):
                continue
            quality = quality_for(rod, rng.randrange(100)) + 1
            quality = min(3, quality)
            weight = weight_for(rod, fish, rng.randrange(100), quality) + 1
            weight = min(20, weight)
            market = market_pct(rng.randrange(100))
            price = price_for(fish, weight, quality, market)
            price = price * (100 + sell_bonus) // 100
            totals["catches"] += 1
            totals["gross"] += price
            totals["golden"] += 1 if rng.random() < 0.015 else 0
            wear += 1
            if wear >= WEAR_MAX:
                if maintain:
                    totals["maint"] += rod.maintenance
                    wear = 0
                else:
                    wear = WEAR_MAX
    n = SEEDS
    for key in ("casts", "catches", "gross", "bait", "maint", "golden"):
        totals[key] /= n
    totals["net"] = totals["gross"] - totals["bait"] - totals["maint"]
    totals["catch_pct"] = totals["catches"] / totals["casts"] * 100
    return totals


def fmt_int(v: float) -> str:
    return f"{v:,.0f}"


def main() -> None:
    fish = parse_fish()
    pool = [f for f in fish if f.spot == "River"]
    print(f"fish={len(fish)} river={len(pool)} casts={CASTS} seeds={SEEDS}")
    policies = [
        ("木竿不保养", RODS["wood"], False),
        ("竹竿不保养", RODS["bamboo"], False),
        ("竹竿保养", RODS["bamboo"], True),
        ("玻璃钢不保养", RODS["glass"], False),
        ("玻璃钢保养", RODS["glass"], True),
        ("碳素不保养", RODS["carbon"], False),
        ("碳素保养", RODS["carbon"], True),
    ]
    lines = [
        "# 鱼竿长期收益模拟（只读分析）",
        "",
        "> 2026-10-20 · 脚本：`scripts/fishing-rod-sim.py`；**尚未改游戏数值**。",
        "",
        "## 口径",
        "",
        "- 河流，90 天 × 每天 60 次主动抛竿 = **5,400 次/档**；20 个随机种子平均。",
        "- 成功率是模型假设：`0.95 × 框控制力 × 磨损 × 难度 × 性格 × 鱼王`；"
        "框控制力 `(frame/20)^1.2`，磨损 ×0.85，难度 `1-0.08*(d-1)`，"
        "性格 lazy 1.0 / normal .92 / active .78 / fierce .60，鱼王 ×0.60。",
        "- 收入只算卖鱼；经验/金色图鉴不计钱（金色目前没有售价加成）。",
        "- 保养每 30 条鱼一次：木 300 / 竹 900 / 玻璃钢 3,000 / 碳素 9,000；不保养一直顶着 62% 加速度。",
        "- 不含鱼竿购买价、自动钓鱼、技能/鱼饵品质加成；普通饵。",
        "",
        "## 结果",
        "",
    ]
    rows = []
    for plan_name, plan in A_PLANS.items():
        lines.append(f"### {plan_name}")
        lines.append("")
        lines.append("| 方案 | 上鱼率 | 渔获 | 毛收入 | 鱼饵 | 保养 | 净收入 | 净收入/天 |")
        lines.append("|---|---:|---:|---:|---:|---:|---:|---:|")
        for label, rod, maintain in policies:
            r = run(pool, rod, maintain, plan)
            rows.append((plan_name, label, r))
            lines.append(
                f"| {label} | {r['catch_pct']:.1f}% | {fmt_int(r['catches'])} | "
                f"{fmt_int(r['gross'])} | {fmt_int(r['bait'])} | {fmt_int(r['maint'])} | "
                f"**{fmt_int(r['net'])}** | {fmt_int(r['net'] / 90)} |"
            )
        lines.append("")
    # 简易结论
    a1 = {label: r for plan, label, r in rows if plan == "A1 温和"}
    a2 = {label: r for plan, label, r in rows if plan == "A2 中强"}
    lines += [
        "## 先看结论",
        "",
        f"- A1 温和方案：竹竿不保养净收入 **{fmt_int(a1['竹竿不保养']['net'])}**，"
        f"玻璃钢保养 **{fmt_int(a1['玻璃钢保养']['net'])}**，碳素保养 **{fmt_int(a1['碳素保养']['net'])}**。",
        f"- A2 中强方案：竹竿不保养 **{fmt_int(a2['竹竿不保养']['net'])}**，"
        f"玻璃钢保养 **{fmt_int(a2['玻璃钢保养']['net'])}**，碳素保养 **{fmt_int(a2['碳素保养']['net'])}**。",
        "- 高级竿保养目前被维修费拖死：碳素 9,000/30 次 = 300/条，而每条鱼毛收入只有 100~200；"
        "A 的售价加成不够填这个洞。",
        "",
        "## 维修费红线（打平竹竿不保养所需）",
        "",
        "| 方案 | 竿 | 当前保养/次 | 打平所需保养/次 |",
        "|---|---|---:|---:|",
    ]
    for plan_name in ("A2 中强", "A3 强力"):
        base = next(r for p, label, r in rows if p == plan_name and label == "竹竿不保养")["net"]
        for label in ("玻璃钢保养", "碳素保养"):
            r = next(r for p, lab, r in rows if p == plan_name and lab == label)
            intervals = r["catches"] / 30
            cap = (r["gross"] - r["bait"] - base) / intervals if intervals else 0
            current = RODS["glass" if label.startswith("玻璃") else "carbon"].maintenance
            lines.append(f"| {plan_name} | {label.replace('保养','')} | {current:,} | **{cap:,.0f}** |")
    lines += [
        "",
        "- 也就是说：A2 下玻璃钢保养要从 3,000 降到约 700~800、碳素从 9,000 降到约 1,200，"
        "才刚打平“竹竿不保养”；A3 也需要大幅降。",
        "- 结论：**A 路线单独上不够，必须先重做维修费**（固定大降或改成按收入 20~30% 抽成），否则高级竿保养永远是负收益。",
        "",
        "## 下一步（未写代码）",
        "",
        "1. 先拍板维修费方案：固定降低，还是按最近渔获收入抽成。",
        "2. 再定 A 的售价/经验/省饵档位；当前模拟里 A1 太弱，A2 接近但被维修费吃掉。",
        "3. 打架时长（活泼/凶猛/鱼王）另开任务，不跟竿绑定。",
        "",
        "## 复跑",
        "",
        "```powershell",
        "python scripts/fishing-rod-sim.py",
        "```",
        "",
    ]
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    print(f"\nwrote {OUT}")


if __name__ == "__main__":
    main()
