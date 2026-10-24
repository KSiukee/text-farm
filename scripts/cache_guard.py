# -*- coding: utf-8 -*-
"""构建缓存体积门卫（搬自夜航 tools/cache_guard.py，阈值按文字农场调整）。

只量、不删：`--check` 超上限返回 1；`--clean` 才是显式清理。
清理口径与夜航/圈记一致：留下能双击跑的 exe/dll 与 bundle，
删掉 deps/build/incremental/examples/.fingerprint、调试符号/链接中间物。

    python scripts/cache_guard.py --check
    python scripts/cache_guard.py --clean
    python scripts/cache_guard.py --dry-run
    python scripts/cache_guard.py --selftest
"""
from __future__ import annotations

import argparse
import os
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# 文字农场的 workspace 比夜航大（40+ test 二进制），上限放到 8 GB；
# 超过 2 GB 只提示，不拦。
LIMIT_GB = 8.0
NOTICE_GB = 2.0
MARKERS = ("debug", "release")

CACHE_DIRS = ("deps", "build", "incremental", "examples", ".fingerprint")
TARGET_CACHE_DIRS = ("doc", "tmp")
CACHE_EXTS = (".pdb", ".rlib", ".rmeta", ".d", ".lib", ".exp", ".obj", ".ilk")
KEEP_EXTS = (".exe", ".dll", ".so", ".dylib")
KEEP_DIRS = ("bundle", "nsis", "msi", "wix", "apk", "deb", "appimage", "dmg")


def dir_gb(path: Path) -> float:
    total = 0
    for base, _dirs, files in os.walk(path, onerror=lambda _e: None):
        for name in files:
            try:
                total += os.stat(os.path.join(base, name), follow_symlinks=False).st_size
            except OSError:
                pass
    return total / (1024**3)


def find_targets(root: Path, max_depth: int = 3) -> list[Path]:
    targets: list[Path] = []
    root = root.resolve()

    def walk(base: Path, depth: int) -> None:
        if depth > max_depth:
            return
        try:
            entries = list(os.scandir(base))
        except OSError:
            return
        for entry in entries:
            if not entry.is_dir(follow_symlinks=False):
                continue
            path = Path(entry.path)
            if entry.name == "target":
                if any((path / marker).is_dir() for marker in MARKERS):
                    targets.append(path)
                continue
            walk(path, depth + 1)

    walk(root, 0)
    return targets


def clean_target(target: Path, dry_run: bool) -> tuple[float, float]:
    before = dir_gb(target)

    # target 根下的 doc/tmp
    for name in TARGET_CACHE_DIRS:
        path = target / name
        if path.is_dir() and not dry_run:
            shutil.rmtree(path, ignore_errors=True)

    for entry in list(os.scandir(target)):
        path = Path(entry.path)
        if not entry.is_dir(follow_symlinks=False) or entry.name in KEEP_DIRS:
            continue
        for name in CACHE_DIRS:
            cache = path / name
            if cache.is_dir() and not dry_run:
                shutil.rmtree(cache, ignore_errors=True)
        for item in list(os.scandir(path)):
            if item.is_file(follow_symlinks=False) and item.name.endswith(CACHE_EXTS):
                if not dry_run:
                    try:
                        os.unlink(item.path)
                    except OSError:
                        pass

    after = before if dry_run else dir_gb(target)
    return before, after


def check(targets: list[Path]) -> int:
    if not targets:
        print("✓ 没有找到已构建的 target/ —— 没编过就不用清")
        return 0
    total = 0.0
    for target in targets:
        size = dir_gb(target)
        total += size
        print(f"  {size:7.2f} GB  {target}")
    print(f"合计 {total:.2f} GB（上限 {LIMIT_GB:.1f} GB）")
    if total > LIMIT_GB:
        print("✗ 构建缓存超上限：先跑 python scripts/cache_guard.py --clean")
        return 1
    if total > NOTICE_GB:
        print("· 构建缓存还在；要省空间可显式 --clean")
    print("✓ 构建缓存在限内")
    return 0


def selftest() -> int:
    with tempfile.TemporaryDirectory(prefix="tf-cache-selftest-") as tmp:
        root = Path(tmp)
        target = root / "target"
        debug = target / "debug"
        deps = debug / "deps"
        deps.mkdir(parents=True)
        (deps / "cache.bin").write_bytes(b"x" * 2048)
        (debug / "app.exe").write_bytes(b"exe")
        (debug / "app.pdb").write_bytes(b"pdb")
        release = target / "release"
        release.mkdir()
        (release / "app.exe").write_bytes(b"exe")

        found = find_targets(root)
        print(f"selftest: found {len(found)} target(s)")
        if len(found) != 1:
            print("✗ selftest: 没认出 target/")
            return 1
        before, after = clean_target(found[0], dry_run=False)
        checks = [
            not (debug / "deps").exists(),
            (debug / "app.exe").exists(),
            (release / "app.exe").exists(),
            not (debug / "app.pdb").exists(),
        ]
        print(f"selftest: {before:.3f} GB -> {after:.3f} GB；checks={checks}")
        if not all(checks):
            print("✗ selftest: 清理口径不对")
            return 1
        print("✓ selftest 通过")
        return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=ROOT)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--clean", action="store_true")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    root = args.root.resolve()

    if args.selftest:
        return selftest()

    targets = find_targets(root)
    if args.clean or args.dry_run:
        if not targets:
            print("没找到 target/ —— 没编过就不用清")
            return 0
        total_before = 0.0
        total_after = 0.0
        for target in targets:
            before, after = clean_target(target, args.dry_run)
            total_before += before
            total_after += after
            verb = "would clean" if args.dry_run else "cleaned"
            print(f"{verb} {before:.2f} -> {after:.2f} GB  {target}")
        print(f"合计 {total_before:.2f} -> {total_after:.2f} GB")
        return 0

    # 默认/--check：只量，不删
    return check(targets)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
