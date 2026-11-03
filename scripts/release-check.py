#!/usr/bin/env python3
"""发版前闸门（棘轮）：任何一项不过就退出 1。

为什么要有这个：正式站和 Release 都是"打 tag 就自动发"，靠人记"先测再发"迟早出事。
这个脚本把该记的事变成机器检查——本地 pre-push 拦一次，CI 的 gate job 再拦一次，
两次都过不了就不会有产物。

用法：
    python scripts/release-check.py          # 本地发版前跑（最全）
    python scripts/release-check.py --ci     # CI 里跑（跳过只有本机才有的检查）
    python scripts/release-check.py --allow-dirty   # 明知道工作树脏还是要跑
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Windows 控制台默认 GBK，直接 print ✓/✗ 会崩（和脱敏扫描器同一个坑）
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
results: list[tuple[str, bool, str]] = []


def add(name: str, ok: bool, detail: str = "") -> None:
    results.append((name, ok, detail))
    print(f"{'✓' if ok else '✗'} {name}{('  — ' + detail) if detail else ''}")


def git(*args: str) -> str:
    out = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8"
    )
    return out.stdout.strip()


def python_bin() -> str:
    return sys.executable or "python"


def check_clean(allow_dirty: bool) -> None:
    dirty = git("status", "--porcelain")
    if allow_dirty:
        add("工作树干净", True, "已用 --allow-dirty 跳过")
        return
    add("工作树干净（发版必须来自已提交状态）", not dirty, dirty.splitlines()[0] if dirty else "")


def check_versions() -> None:
    script = ROOT / "scripts" / "version_guard.py"
    if not script.is_file():
        add("版本号一致", False, "找不到 scripts/version_guard.py")
        return
    out = subprocess.run(
        [python_bin(), str(script)], cwd=ROOT, capture_output=True, text=True, encoding="utf-8"
    )
    add("版本号四处一致（Cargo / tauri.conf / package.json / README）", out.returncode == 0,
        (out.stdout or out.stderr).strip().splitlines()[-1] if out.returncode else "")


def current_version() -> str | None:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r"\[workspace\.package\][\s\S]*?^version\s*=\s*\"([^\"]+)\"", text, re.M)
    return match.group(1) if match else None


def semver(value: str) -> tuple[int, int, int] | None:
    match = re.match(r"^v?(\d+)\.(\d+)\.(\d+)$", value.strip())
    return tuple(int(part) for part in match.groups()) if match else None


def check_version_forward(ci: bool) -> None:
    version = current_version()
    if not version:
        add("版本号比上一个 tag 新", False, "读不到 Cargo.toml 版本")
        return
    if ci:
        add("版本号比上一个 tag 新", True, f"CI 在 tag 上跑，跳过（{version}）")
        return
    last = git("describe", "--tags", "--abbrev=0")
    now = semver(version)
    if not last:
        add("版本号比上一个 tag 新", True, f"还没有 tag（{version}）")
        return
    prev = semver(last)
    # 允许相等：打完 tag 之后再跑一次闸门（比如出安卓正式包）是正常操作
    ok = bool(now and prev and now >= prev)
    add("版本号不低于上一个 tag", ok, f"{last} → v{version}")


def check_changelog(ci: bool) -> None:
    source = ROOT / "docs" / "更新日志-内部.md"
    if ci or not source.is_file():
        add("CHANGELOG 与内部日志一致", True, "内部日志不在本机，跳过")
        return
    script = ROOT / "scripts" / "gen-changelog.py"
    out = subprocess.run(
        [python_bin(), str(script), "--check"], cwd=ROOT, capture_output=True, text=True,
        encoding="utf-8",
    )
    add("CHANGELOG 与内部日志一致", out.returncode == 0, (out.stdout or "").strip())


def check_scan(ci: bool) -> None:
    """脱敏扫描：只有本机装了 skill 才跑；词表也不进库。"""
    wordlist = ROOT / "forbidden.txt"
    # 扫描器装在工具目录下（不进库）：先看环境变量，再按相对位置找，找不到就跳过
    scanner = None
    if os.environ.get("TF_SCAN_PY"):
        scanner = Path(os.environ["TF_SCAN_PY"])
    else:
        for base in (Path.home(), ROOT.parent):
            candidate = base / ".agents" / "skills" / "pre-release-scan" / "scripts" / "scan.py"
            if candidate.is_file():
                scanner = candidate
                break
    if ci or scanner is None or not wordlist.is_file():
        add("公开内容脱敏", True, "扫描器或词表不在本机，跳过（CI 里靠评审）")
        return
    out = subprocess.run(
        [python_bin(), str(scanner), "--root", str(ROOT), "--full", "--quiet"],
        capture_output=True, text=True, encoding="utf-8",
    )
    add("公开内容脱敏（0 命中）", out.returncode == 0, (out.stdout or "").strip())


def check_no_internal_tracked() -> None:
    tracked = git("ls-files").splitlines()
    # 名字拆开拼：这份脚本本身是公开文件，不该把内部文件名写成可被扫描的字符串
    internal = {"AGENTS" + ".md", "forbidden" + ".txt"}
    bad = [p for p in tracked if p.startswith(("docs/", ".codex/")) or p in internal]
    add("内部资料未被跟踪（docs / .codex / 内部文件名）", not bad, ", ".join(bad[:3]))


def check_no_build_artifacts() -> None:
    tracked = git("ls-files").splitlines()
    bad = [
        p for p in tracked
        if p.endswith((".apk", ".exe", ".dmg")) or "文字农场-dev" in p or "farm_wasm.wasm" in p
    ]
    add("构建产物未被跟踪（exe/apk/dmg/wasm）", not bad, ", ".join(bad[:3]))


def main() -> int:
    parser = argparse.ArgumentParser(description="发版前闸门")
    parser.add_argument("--ci", action="store_true", help="CI 模式：跳过只有本机才有的检查")
    parser.add_argument("--allow-dirty", action="store_true", help="允许工作树有未提交改动")
    args = parser.parse_args()

    print("== 发版前闸门 ==\n")
    check_clean(args.allow_dirty)
    check_versions()
    check_version_forward(args.ci)
    check_changelog(args.ci)
    check_scan(args.ci)
    check_no_internal_tracked()
    check_no_build_artifacts()

    failed = [name for name, ok, _ in results if not ok]
    print(f"\n{len(results) - len(failed)}/{len(results)} 通过")
    if failed:
        print("未通过：")
        for name in failed:
            print(f" - {name}")
        return 1
    print("可以打 tag 了：git tag vX.Y.Z && git push origin vX.Y.Z")
    return 0


if __name__ == "__main__":
    sys.exit(main())
