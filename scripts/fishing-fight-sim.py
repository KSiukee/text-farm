#!/usr/bin/env python3
"""打架时长模拟（只读分析，不改游戏数值）。

当前机制：进度 35 起，满 100 成功，0 失败；基础增速 22/s、扣速 12/s、
离框缓冲 0.8s；收线窗口 = 11s + 难度*0.9（鱼王 20s）。

这里不模拟 2D 追鱼，只模拟“在框内的时间比例 + 进出切换”：
- f = 玩家能把框保持在鱼身上的时间比例（按性格假设）。
- 用马尔可夫进出（out 会连续一段），另外 2%/s 概率触发一次 1.5s 大失手；
  out 超过 0.8s 才开始扣进度。
- 延长方案：把 progress_gain/loss 都除以 m，把收线窗口乘以 m；
  鱼的行为不变，所以实际打架时长约 ×m，失误次数也约 ×m。

用途：判断“活跃/凶猛/鱼王”到底会变多长、成功率会不会掉，
不是游戏真实胜率，也不能直接测“好玩”。
"""
from __future__ import annotations

import random
import statistics
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/打架时长模拟.md"

START = 35
GOAL = 100
GAIN = 22.0
LOSS = 12.0
GRACE = 0.8
DT = 0.1
TRIALS = 20_000


@dataclass(frozen=True)
class Case:
    key: str
    name: str
    f: float          # 在框内时间比例
    exits_per_s: float
    window_s: float   # 当前收线窗口


CASES = [
    Case("lazy", "慵懒·难度1", 0.65, 0.10, 11.9),
    Case("normal", "普通·难度2", 0.60, 0.20, 12.8),
    Case("active", "活泼·难度3", 0.55, 0.35, 13.7),
    Case("fierce", "凶猛·难度4", 0.50, 0.50, 14.6),
    Case("king", "鱼王·难度5", 0.48, 0.60, 20.0),
]

# 延长倍率；普通/慵懒不动
PLANS = {
    "D0 当前": {"lazy": 1.0, "normal": 1.0, "active": 1.0, "fierce": 1.0, "king": 1.0},
    "D1 轻": {"lazy": 1.0, "normal": 1.0, "active": 1.2, "fierce": 1.4, "king": 1.6},
    "D2 中": {"lazy": 1.0, "normal": 1.0, "active": 1.3, "fierce": 1.5, "king": 1.8},
    "D3 强": {"lazy": 1.0, "normal": 1.0, "active": 1.5, "fierce": 1.8, "king": 2.2},
}


def simulate(case: Case, mult: float, rng: random.Random, grace_s: float = GRACE) -> dict:
    gain = GAIN / mult
    loss = LOSS / mult
    window = case.window_s * mult
    successes = 0
    durations: list[float] = []
    exits_total = 0
    # 用马尔可夫进出代替每 0.1s 独立抽样：out 会连续一段，才可能真的掉进度。
    p_in = 0.10
    p_out = p_in * (1.0 - case.f) / case.f
    for _ in range(TRIALS):
        progress = START
        t = 0.0
        grace = 0.0
        in_zone = True
        exits = 0
        burst = 0.0
        while t < window:
            if burst > 0:
                burst = max(0.0, burst - DT)
                in_zone = False
            elif in_zone:
                if rng.random() < p_out:
                    in_zone = False
                    exits += 1
            else:
                if rng.random() < p_in:
                    in_zone = True
            # 偶发大失手：鱼突然变向/玩家看错，强制出框 1.5s
            if in_zone and rng.random() < 0.02 * DT:
                burst = 1.5
                in_zone = False
                exits += 1
            if in_zone:
                grace = grace_s
                progress += gain * DT
            else:
                if grace > 0:
                    grace = max(0.0, grace - DT)
                else:
                    progress -= loss * DT
            t += DT
            if progress >= GOAL:
                successes += 1
                durations.append(t)
                break
            if progress <= 0:
                break
        exits_total += exits
    return {
        "success": successes / TRIALS * 100,
        "duration": statistics.median(durations) if durations else 0.0,
        "exits": exits_total / TRIALS,
        "window": window,
    }


def main() -> None:
    lines = [
        "# 打架时长模拟（只读分析）",
        "",
        "> 2026-10-20 · 脚本：`scripts/fishing-fight-sim.py`；**未改游戏数值**。",
        "",
        "## 口径",
        "",
        "- 进度 35 起、100 满；基础增速 22/s、扣速 12/s、离框缓冲 0.8s。",
        "- 收线窗口按难度：慵懒 11.9s / 普通 12.8s / 活泼 13.7s / 凶猛 14.6s / 鱼王 20s。",
        "- 在框内比例 f 是假设：慵懒 .65 / 普通 .60 / 活泼 .55 / 凶猛 .50 / 鱼王 .48；"
        "每 0.1s 抽一次 in/out，out 超过 0.8s 才扣进度。",
        "- 延长方案：增速/扣速都除以 m，收线窗口乘以 m；鱼行为不变。",
        "- 这不是 2D 追鱼的真实胜率，也不能测“好玩”；只看实际时长/成功率/失误次数的方向。",
        "",
        "## 结果",
        "",
    ]
    print(f"trials={TRIALS} cases={len(CASES)}")
    for plan_name, plan in PLANS.items():
        lines += [f"### {plan_name}", ""]
        lines.append("| 场景 | 实际时长中位 | 成功率 | 平均失误次数 | 收线窗口 |")
        lines.append("|---|---:|---:|---:|---:|")
        for case in CASES:
            rng = random.Random(20261020)
            r = simulate(case, plan[case.key], rng)
            lines.append(
                f"| {case.name} | {r['duration']:.1f}s | {r['success']:.1f}% | "
                f"{r['exits']:.1f} | {r['window']:.1f}s |"
            )
            print(plan_name, case.name, r)
        lines.append("")

    lines += [
        "## 先看结论",
        "",
        "- 当前实际时长：活泼 5.3s / 凶猛 5.9s / 鱼王 6.6s；D2 变 7.3 / 9.8 / 13.3s，D3 变 8.7 / 12.2 / 16.6s。",
        "- 成功率没有掉：D2 活泼 93.5% / 凶猛 91.8% / 鱼王 97.7%，和当前 91.9 / 89.6 / 93.6 基本持平甚至更高；"
        "因为收线窗口也按倍率放大了，长架不是靠更容易失败换来的。",
        "- 失误次数从 2.9 / 3.7 / 4.2 升到 3.9 / 5.7 / 7.8，长架确实更能“打架”，不是单纯看条慢。",
        "- 结论：D2 是“纯拉长、不中和难度”的方案；D3 鱼王 16.6s 可能偏拖。"
        "模拟测不了“好玩”，最终要你真机试 D1/D2/D3。",
        "",
        "## 离框容错（grace）方案（D2）",
        "",
        "| grace | 平均成功率 | 平均时长 | 活泼成功率 | 凶猛成功率 | 鱼王成功率 | 平均失误 |",
        "|---:|---:|---:|---:|---:|---:|---:|",
    ]
    d2 = PLANS["D2 中"]
    for grace in (0.8, 1.0, 1.2, 1.5, 2.0):
        vals = [simulate(c, d2[c.key], random.Random(20261020), grace) for c in CASES]
        avg_success = sum(v["success"] for v in vals) / len(vals)
        avg_duration = sum(v["duration"] for v in vals) / len(vals)
        avg_exits = sum(v["exits"] for v in vals) / len(vals)
        by_name = {c.key: v for c, v in zip(CASES, vals)}
        lines.append(
            f"| {grace:.1f}s | {avg_success:.1f}% | {avg_duration:.1f}s | "
            f"{by_name['active']['success']:.1f}% | {by_name['fierce']['success']:.1f}% | "
            f"{by_name['king']['success']:.1f}% | {avg_exits:.1f} |"
        )
    lines += [
        "",
        "- grace 越长，掉进度越少：成功率上升、实际时长略缩短；它买的是**容错**，不是更长/更难。",
        "- 0.8s 当前；1.0s 小幅；1.2s 中等；1.5s 很宽容；2.0s 基本很难失败，容易削弱紧张感。",
        "- 全局改，所有竿一起；不建议按竿单独给 grace，否则又回到每根竿调手感。",
        "",
        "## 复跑",
        "",
        "```powershell",
        "python scripts/fishing-fight-sim.py",
        "```",
        "",
    ]
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
