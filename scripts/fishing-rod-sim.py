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
from dataclasses import dataclass, replace
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FISHING_RS = ROOT / "crates/farm-core/src/fishing.rs"
OUT = ROOT / "docs/鱼竿长期收益模拟.md"
CASTS = 5400
SEEDS = 20
BAIT_PRICE = 10


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
    durability: int
    weight_bonus: int
    quality_thresholds: tuple[int, int]
    sell_bonus_pct: int = 0
    bait_save_pct: int = 0


RODS = {
    "wood": Rod("wood", "木竿", 20, 300, 30, 0, (60, 95)),
    "bamboo": Rod("bamboo", "竹竿", 18, 900, 30, 1, (45, 85)),
    "glass": Rod("glass", "玻璃钢", 15, 3000, 30, 2, (30, 75)),
    "carbon": Rod("carbon", "碳素", 13, 9000, 30, 3, (15, 60)),
}

# A 路线候选：售价加成 / 省饵；经验、金色率不进入利润。
A_PLANS = {
    "A0 无加成": {"wood": (0, 0), "bamboo": (0, 0), "glass": (0, 0), "carbon": (0, 0)},
    "A1 温和": {"wood": (0, 0), "bamboo": (5, 0), "glass": (10, 5), "carbon": (15, 20)},
    "A2 中强": {"wood": (0, 0), "bamboo": (10, 0), "glass": (25, 10), "carbon": (40, 20)},
    "A3 强力": {"wood": (0, 0), "bamboo": (20, 0), "glass": (50, 15), "carbon": (80, 25)},
}

# 维修费方案：金额 / 耐久（多少条鱼一次保养）
MAINT_PLANS = {
    "M1 现行": {"wood": (300, 30), "bamboo": (900, 30), "glass": (3000, 30), "carbon": (9000, 30)},
    "M2 只降费": {"wood": (300, 30), "bamboo": (600, 30), "glass": (1200, 30), "carbon": (1800, 30)},
    "M3 降费+耐久": {"wood": (300, 30), "bamboo": (600, 40), "glass": (1200, 60), "carbon": (1800, 90)},
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


def success_rate(rod: Rod, worn: bool, fish: Fish, personality: str, grace_s: float = 0.8) -> float:
    control = (rod.frame / 20.0) ** 1.2 * (0.85 if worn else 1.0)
    diff = 1.0 - 0.08 * (fish.difficulty - 1)
    pers = {"Lazy": 1.0, "Normal": 0.92, "Active": 0.78, "Fierce": 0.60}[personality]
    king = 0.60 if fish.rarity == "King" else 1.0
    # 离框容错：框越小收益越大；只是粗糙模型，用来判断会不会“中和”。
    grace = 1.0 + 0.35 * max(0.0, grace_s - 0.8) / 0.8 * (1.0 - control)
    return max(0.03, min(0.95, 0.95 * control * diff * pers * king * grace))


def run(
    pool: list[Fish],
    rod: Rod,
    maintain: bool,
    plan: dict[str, tuple[int, int]],
    golden_floor: int = 0,
    golden_bonus: float = 0.0,
    golden_mult: float = 1.0,
    grace_s: float = 0.8,
) -> dict:
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
            worn = not maintain and wear >= rod.durability
            if rng.random() >= success_rate(rod, worn, fish, personality, grace_s):
                continue
            quality = quality_for(rod, rng.randrange(100)) + 1
            quality = min(3, quality)
            weight = weight_for(rod, fish, rng.randrange(100), quality) + 1
            weight = min(20, weight)
            market = market_pct(rng.randrange(100))
            price = price_for(fish, weight, quality, market)
            price = price * (100 + sell_bonus) // 100
            golden = rng.random() < 0.015 + golden_bonus
            if golden:
                totals["golden"] += 1
                if golden_mult > 1.0:
                    price = int(price * golden_mult)
                if golden_floor > 0:
                    price = max(price, golden_floor)
            totals["catches"] += 1
            totals["gross"] += price
            wear += 1
            if wear >= rod.durability:
                if maintain:
                    totals["maint"] += rod.maintenance
                    wear = 0
                else:
                    wear = rod.durability
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
    # 维修费方案对比（A2）
    lines += [
        "## 维修费方案对比（A2 中强）",
        "",
        "| 方案 | 竹不保养 | 竹保养 | 玻璃钢保养 | 碳素保养 |",
        "|---|---:|---:|---:|---:|",
    ]
    for mname, spec in MAINT_PLANS.items():
        rods = {
            key: replace(RODS[key], maintenance=spec[key][0], durability=spec[key][1])
            for key in RODS
        }
        vals = []
        for key, maintain in (("bamboo", False), ("bamboo", True), ("glass", True), ("carbon", True)):
            r = run(pool, rods[key], maintain, A_PLANS["A2 中强"])
            vals.append(r["net"])
        lines.append(
            f"| {mname} | {fmt_int(vals[0])} | {fmt_int(vals[1])} | "
            f"{fmt_int(vals[2])} | {fmt_int(vals[3])} |"
        )
    lines.append("")

    # A2 + M3 + 金色 / 离框容错
    m3 = {
        key: replace(
            RODS[key],
            maintenance=MAINT_PLANS["M3 降费+耐久"][key][0],
            durability=MAINT_PLANS["M3 降费+耐久"][key][1],
        )
        for key in RODS
    }
    king_price = next(f.price for f in pool if f.rarity == "King")
    golden_bonus = {"bamboo": 0.005, "glass": 0.01, "carbon": 0.02}
    golden_plans = [
        ("G0 当前：只进图鉴", 0, 0.0, 1.0),
        ("G1 半鱼王价（河王 800→400）", king_price // 2, 1.0, 1.0),
        ("G2 四分之一鱼王价（200）", king_price // 4, 1.0, 1.0),
        ("G3 普通鱼价×2", 0, 1.0, 2.0),
    ]
    lines += [
        "## A2 + M3：金色售价方案",
        "",
        "金色概率 = 1.5% + A2 竿加成（竹 +0.5%、玻 +1%、碳 +2%）；G1 把金色鱼售价设成半条河王价 400。",
        "",
        "| 金色方案 | 竹不保养 | 玻璃钢保养 | 碳素保养 |",
        "|---|---:|---:|---:|",
    ]
    for gname, floor, use_bonus, mult in golden_plans:
        vals = []
        for key, maintain in (("bamboo", False), ("glass", True), ("carbon", True)):
            bonus = golden_bonus[key] if use_bonus else 0.0
            r = run(
                pool,
                m3[key],
                maintain,
                A_PLANS["A2 中强"],
                golden_floor=floor,
                golden_bonus=bonus,
                golden_mult=mult,
            )
            vals.append(r["net"])
        lines.append(f"| {gname} | {fmt_int(vals[0])} | {fmt_int(vals[1])} | {fmt_int(vals[2])} |")
    lines += [
        "",
        "## A2 + M3 + G1：离框容错（grace）",
        "",
        "grace 0.8 → 1.2 → 1.6s；粗糙模型：框越小，延长容错带来的成功率提升越大。",
        "",
        "| grace | 竿 | 上鱼率 | 净收入 |",
        "|---:|---|---:|---:|",
    ]
    for grace in (0.8, 1.2, 1.6):
        for key, maintain in (("bamboo", False), ("glass", True), ("carbon", True)):
            r = run(
                pool,
                m3[key],
                maintain,
                A_PLANS["A2 中强"],
                golden_floor=king_price // 2,
                golden_bonus=golden_bonus[key],
                grace_s=grace,
            )
            lines.append(
                f"| {grace:.1f}s | {m3[key].name}{'不保养' if not maintain else '保养'} | "
                f"{r['catch_pct']:.1f}% | {fmt_int(r['net'])} |"
            )
    lines += [
        "",
        "- 金色：半条河王价（400）是当前最直观的方案；它只在 1.5%~3.5% 的渔获上生效，不会冲垮普通鱼价。",
        "- 离框容错：延长 grace 会让小框的上鱼率明显回升，但它是“少扣进度”，会让打架更轻松、更短；"
        "它能把高竿从“难到不想用”拉回“可用”，不是让打架更有趣/更长。要更长仍得动鱼的进度/行为（#89）。",
        "",
    ]

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
        "- 结论：**A 路线单独上不够，必须先把维修费改合理；但不做抽成/税**。",
        "- M3 方案（木 300/30、竹 600/40、玻 1200/60、碳 1800/90）在 A2 下："
        "竹保养 235,813 ≈ 竹不保养 236,417，玻璃钢保养 249,707、碳素保养 281,915，高杆保养终于有正收益。",
        "",
        "## 下一步（未写代码）",
        "",
        "1. 拍板 M3 维修费：木 300/30、竹 600/40、玻璃钢 1,200/60、碳素 1,800/90；不抽成。",
        "2. 再定 A2：竹 +10% 售价；玻璃钢 +25% 售价 +10% 省饵；碳素 +40% 售价 +20% 省饵。",
        "3. 打架时长（活泼/凶猛/鱼王）另开 #89，不跟竿绑定。",
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
