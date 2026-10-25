#!/usr/bin/env python3
"""版本号守卫（圈记版设计，按文字农场 7 处副本裁剪）。

圈记这版真正值钱的两件事，不是“多查几处”：
  1. `--selftest`：造一份临时树，逐处故意改坏，断言守卫**真的会红**。
     没有自检的一致性检查，等于一个永远绿灯的装饰品。
  2. `MIN_SEEN` 防空转：少读到一处就判失败，避免路径写错时静默放过。

用法：
    python scripts/version_guard.py                 # 只查一致性与不跳号
    python scripts/version_guard.py --kind fix      # 只允许第三位 +1
    python scripts/version_guard.py --kind feat     # 中位 +1、第三位归零
    python scripts/version_guard.py --allow-downgrade
    python scripts/version_guard.py --selftest      # 自证会红
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# (相对路径, 说明, 提取方式)；这 7 处必须全部读到。
SOURCES: list[tuple[str, str, str]] = [
    ("Cargo.toml", "[workspace.package] version", "toml_workspace"),
    ("Cargo.lock", "text-farm 包版本", "lock"),
    ("app/tauri.conf.json", "Tauri 应用版本（安装包自报的就是它）", "json"),
    ("app/frontend/package.json", "前端工程版本", "json"),
    ("README.md", "README 当前版本", "readme"),
    (".codex/state.json", "交接系统里的项目版本", "json"),
    ("CHANGELOG.md", "更新日志首条标题", "changelog"),
]
MIN_SEEN = len(SOURCES)


def read(path: Path) -> str:
    # Windows 检出可能是 CRLF；统一成 LF 再匹配多行模式。
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def extract(path: Path, kind: str) -> str | None:
    if not path.exists():
        return None
    text = read(path)

    if kind == "toml_workspace":
        match = re.search(
            r"\[workspace\.package\][\s\S]*?^version\s*=\s*\"([^\"]+)\"",
            text,
            re.M,
        )
        return match.group(1) if match else None

    if kind == "lock":
        match = re.search(
            r'\[\[package\]\]\nname = "text-farm"\nversion = "([^"]+)"',
            text,
        )
        return match.group(1) if match else None

    if kind == "json":
        try:
            return json.loads(text).get("version")
        except json.JSONDecodeError:
            return None

    if kind == "readme":
        match = re.search(r"^v(\d+\.\d+\.\d+)：", text, re.M)
        return match.group(1) if match else None

    if kind == "changelog":
        match = re.search(r"^##\s*\[?(\d+\.\d+\.\d+)\]?", text, re.M)
        return match.group(1) if match else None

    raise ValueError(f"unknown kind: {kind}")


def collect(root: Path) -> tuple[list[str], dict[str, str], int]:
    """返回 (问题清单, 各文件版本, 实际读到几处)。"""
    problems: list[str] = []
    values: dict[str, str] = {}
    seen = 0

    for relative, description, kind in SOURCES:
        version = extract(root / relative, kind)
        if version is None:
            problems.append(f"读不到版本号：{relative}（{description}）")
            continue
        seen += 1
        values[relative] = version

    if seen < MIN_SEEN:
        problems.append(f"只读到 {seen} 处版本号（底线 {MIN_SEEN}）—— 守卫可能在空转")

    distinct = set(values.values())
    if len(distinct) > 1:
        detail = "；".join(f"{name} = {version}" for name, version in sorted(values.items()))
        problems.append(f"版本号不一致（{len(distinct)} 种）：{detail}")

    return problems, values, seen


def parse(version: str | None) -> tuple[int, int, int] | None:
    if not version:
        return None
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", version)
    return tuple(int(part) for part in match.groups()) if match else None


def head_version(root: Path) -> str | None:
    try:
        out = subprocess.run(
            ["git", "show", "HEAD:Cargo.toml"],
            cwd=root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            encoding="utf-8",
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    match = re.search(
        r"\[workspace\.package\][\s\S]*?^version\s*=\s*\"([^\"]+)\"",
        out,
        re.M,
    )
    return match.group(1) if match else None


def corrupt(text: str, kind: str) -> str:
    """把一处版本改成 9.9.9；只用于 selftest 的临时树。"""
    if kind == "toml_workspace":
        return re.sub(
            r"(\[workspace\.package\][\s\S]*?^version\s*=\s*\")[^\"]+",
            r"\g<1>9.9.9",
            text,
            count=1,
            flags=re.M,
        )
    if kind == "lock":
        return re.sub(
            r'(name = "text-farm"\nversion = ")[^"]+',
            r"\g<1>9.9.9",
            text,
            count=1,
        )
    if kind == "json":
        data = json.loads(text)
        data["version"] = "9.9.9"
        return json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if kind == "readme":
        return re.sub(r"^v\d+\.\d+\.\d+：", "v9.9.9：", text, count=1, flags=re.M)
    if kind == "changelog":
        return re.sub(
            r"^##\s*\[?\d+\.\d+\.\d+\]?",
            "## 9.9.9",
            text,
            count=1,
            flags=re.M,
        )
    raise ValueError(f"unknown kind: {kind}")


def selftest() -> int:
    """造临时树：基线应绿；逐处改坏，每处都必须红。"""
    with tempfile.TemporaryDirectory(prefix="textfarm-version-selftest-") as temp:
        root = Path(temp)
        for relative, _, _ in SOURCES:
            source = ROOT / relative
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)

        problems, _, seen = collect(root)
        if problems:
            print(f"[selftest] ✗ 基线就红了：{problems[0]}")
            return 1
        if seen < MIN_SEEN:
            print(f"[selftest] ✗ 只读到 {seen} 处（底线 {MIN_SEEN}）")
            return 1

        for relative, _, kind in SOURCES:
            path = root / relative
            original = path.read_text(encoding="utf-8")
            try:
                path.write_text(corrupt(original, kind), encoding="utf-8", newline="")
                broken, _, _ = collect(root)
            finally:
                path.write_text(original, encoding="utf-8", newline="")
            if not broken:
                print(f"[selftest] ✗ 改坏 {relative} 后居然还是绿的 —— 这道门是装饰品")
                return 1
            print(f"[selftest] ✓ 改坏 {relative} 后如期变红")

    print(f"[selftest] ✓ {MIN_SEEN} 处逐处改坏都会红，基线通过")
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")

    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--kind", choices=["none", "fix", "feat"], default="none")
    parser.add_argument("--allow-downgrade", action="store_true")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        return selftest()

    root = args.root.resolve()
    values: dict[str, str] = {}
    failures: list[str] = []

    for relative, description, kind in SOURCES:
        version = extract(root / relative, kind)
        if version is None:
            print(f"✗ {relative:32} 未找到  （{description}）")
        else:
            print(f"✓ {relative:32} {version}")
            values[relative] = version

    problems, _, seen = collect(root)
    failures.extend(problems)

    current_text = values.get("Cargo.toml")
    current = parse(current_text)
    previous_text = head_version(root)
    previous = parse(previous_text)
    print(f"\n当前 {current_text or '?'}，HEAD {previous_text or '（无 git 基线）'}")

    if not current:
        failures.append(f"Cargo.toml 版本不是 x.y.z：{current_text}")
    elif not previous:
        print("· 没有 HEAD 基线，只校验一致性")
    elif current_text == previous_text:
        if args.kind != "none":
            failures.append(f"--kind {args.kind} 要求升版本，但仍是 {current_text}")
        else:
            print("· 未升版本（一致性通过）")
    else:
        lower = current < previous
        if lower:
            if not args.allow_downgrade:
                failures.append(
                    f"版本回退：{previous_text} → {current_text}；一次性纠正请加 --allow-downgrade"
                )
            else:
                print("⚠ 允许一次性版本回退（纠正旧号）")
        else:
            same_major = current[0] == previous[0]
            same_minor = same_major and current[1] == previous[1]
            patch_step = current[2] - previous[2] if same_minor else 0
            minor_step = current[1] - previous[1] if same_major else 0
            if args.kind == "fix" and not (same_minor and patch_step == 1):
                failures.append(f"--kind fix 只允许第三位 +1：{previous_text} → {current_text}")
            elif args.kind == "feat" and not (
                same_major and minor_step == 1 and current[2] == 0
            ):
                failures.append(
                    f"--kind feat 只允许中位 +1、第三位归零：{previous_text} → {current_text}"
                )
            elif args.kind == "none":
                if same_minor and patch_step > 1:
                    failures.append(
                        f"跳号：第三位一次 +{patch_step}（{previous_text} → {current_text}）"
                    )
                if same_major and minor_step > 1:
                    failures.append(
                        f"跳号：中位一次 +{minor_step}（{previous_text} → {current_text}）"
                    )

    if failures:
        print("\n版本门卫未通过：")
        for message in failures:
            print(f" - {message}")
        return 1

    print(f"\n版本门卫通过（{seen}/{MIN_SEEN} 处）。")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
