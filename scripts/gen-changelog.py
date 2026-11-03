#!/usr/bin/env python3
"""把内部更新日志压成公开版 CHANGELOG.md。

内部日志不进仓库（`docs/` 被 .gitignore 排除），只在本机存在；
公开版是**每个版本一行**：剥掉任务号、`docs/` 指针、绝对路径、exe 体积、
schema 号、测试库之类的内部记号，只留玩家看得懂的一句话。

用法：
    python scripts/gen-changelog.py
    python scripts/gen-changelog.py --source docs/更新日志-内部.md --out CHANGELOG.md
    python scripts/gen-changelog.py --check        # 只比对，不写文件（CI/发版前用）
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SOURCE = ROOT / "docs" / "更新日志-内部.md"
DEFAULT_OUT = ROOT / "CHANGELOG.md"

# 整条都是内部记号的版本，直接跳过（工具/调参/重构这类对玩家没意义）
SKIP_PREFIX = ("工具：", "工具:", "调参：", "重构：", "文档：", "脚本：", "守卫：")

JUNK = [
    (re.compile(r"^A\d+\s*(final)?\s*[：:\s]+", re.I), ""),
    (re.compile(r"（A\d+[^）]*）"), ""),
    (re.compile(r"\(A\d+[^)]*\)"), ""),
    (re.compile(r"（#\d+[^）]*）"), ""),
    (re.compile(r"\(#\d+[^)]*\)"), ""),
    (re.compile(r"（详见[^）]*）"), ""),
    (re.compile(r"详见\s*docs/[^\s，；。）]*"), ""),
    (re.compile(r"docs/[^\s，；。）]+"), ""),
    (re.compile(r"[A-Za-z]:\\[^\s，；。）]*"), ""),
    (re.compile(r"exe\s*\d+\.\d+\.\d+\s*[\d.]+\s*MB[。，；]?"), ""),
    (re.compile(r"schema\s*v?\d+"), ""),
    (re.compile(r"\d+\s*例\s*(Rust|前端|UI)[^；。]*"), ""),
    (re.compile(r"\d+\s*项\s*UI[^；。]*"), ""),
    (re.compile(r"（[^）]{0,40}(MB|字节|毫秒|ms)[^）]{0,20}）"), ""),
    (re.compile(r"\d+\.\d+\.\d+\s*实测[。，；]?"), ""),
]


def parse_source(text: str) -> list[tuple[str, str, str]]:
    """按 `## 版本 · 日期` 切块，取每条的第一个短句。"""
    rows: list[tuple[str, str, str]] = []
    for block in re.split(r"^##\s+", text, flags=re.M)[1:]:
        head, _, body = block.partition("\n")
        match = re.match(
            r"([0-9]+\.[0-9]+\.[0-9]+)\s*[·\-]\s*(\d{4}-\d{2}-\d{2})", head.strip()
        )
        if not match:
            continue
        bullets = [line[2:].strip() for line in body.splitlines() if line.startswith("- ")]
        if not bullets:
            continue
        heads = [bullet.split("——")[0].strip() for bullet in bullets]
        summary = "；".join(head for head in heads if head)
        rows.append((match.group(1), match.group(2), summary))
    return rows


def clean(value: str) -> str:
    text = (
        value.replace("影子库", "测试库")
        .replace("影子档", "测试档")
        .replace("真机档", "存档")
        .replace("release exe", "")
    )
    for rx, repl in JUNK:
        text = rx.sub(repl, text)
    text = re.sub(r"[、，]\s*迁移", "", text)
    text = re.sub(r"\s+", " ", text)

    parts: list[str] = []
    for segment in re.split(r"[；;]", text):
        segment = re.sub(r"^A\d+(\s*\+\s*A\d+)*\s*[：:]?\s*", "", segment).strip(" ·，、：")
        if not segment:
            continue
        if segment in {"迁移", "自动迁移", "存档迁移"}:
            continue
        if re.search(r"\bexe\b|测试档|测试库|schema|MB", segment):
            continue
        parts.append(segment)

    text = re.sub(r"；{2,}", "；", "；".join(parts)).strip(" ·，；、：")
    if len(text) > 80:
        cut = max(text.rfind(point, 0, 80) for point in "，；、")
        text = text[: cut if cut > 30 else 80].rstrip("，；、") + "…"
    return text


def render(rows: list[tuple[str, str, str]]) -> str:
    lines = [
        "# 更新日志",
        "",
        "> 从 0.1 到 0.138 的每个版本，一句话摘要（按时间倒序）。",
        "> 早期是边写边记的，措辞比较随意；每个版本的详细说明见 [Releases](../../releases)。",
        "> 这份表由 `scripts/gen-changelog.py` 生成，别手改。",
        "",
        "| 版本 | 日期 | 这一版做了什么 |",
        "|---|---|---|",
    ]
    for version, date, summary in rows:
        if summary.startswith(SKIP_PREFIX):
            continue
        cleaned = clean(summary)
        if cleaned:
            lines.append(f"| {version} | {date} | {cleaned} |")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="生成公开版 CHANGELOG.md")
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE, help="内部更新日志")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help="输出的公开日志")
    parser.add_argument("--check", action="store_true", help="只比对，不写文件")
    args = parser.parse_args()

    if not args.source.is_file():
        print(
            f"找不到内部日志：{args.source}\n"
            "它是本机文件、不进仓库；从备份里恢复后再跑。",
            file=sys.stderr,
        )
        return 1

    text = render(parse_source(args.source.read_text(encoding="utf-8")))

    if args.check:
        current = args.out.read_text(encoding="utf-8") if args.out.is_file() else ""
        if current.strip() == text.strip():
            print(f"{args.out.name} 已是最新")
            return 0
        print(f"{args.out.name} 和内部日志对不上，跑一次不带 --check 的")
        return 1

    args.out.write_text(text, encoding="utf-8")
    rows = text.count("\n| 0.")
    print(f"写好 {args.out}（{rows} 个版本）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
