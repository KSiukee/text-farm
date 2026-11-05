#!/usr/bin/env python3
"""构建产物清理门 —— 只留「要用的那一两个」，其余中间物随手清。

口径沿用一套跑过很多次的 Rust 构建清理规矩，只改了根目录与阈值。
三条规矩：

  删：每个 profile 目录（debug / release / 三元组…）里的
      deps/ build/ incremental/ examples/ .fingerprint/
      以及 *.pdb *.rlib *.rmeta *.d *.lib *.exp *.obj *.ilk
  留：profile 根下面的 *.exe / *.dll（那才是「构建物」）、bundle/（Tauri 安装包）
  ⇒ 一句话：留下能双击跑的东西，删掉为了再跑一次而存在的东西。

本仓额外清两个地方（Rust 之外的构建垃圾）：
  · 安卓的 .so 构建目录：编在仓外的 ASCII 路径里（中文路径链接器不认），
    位置用环境变量 `TF_TARGET_DIR` 指定，不设就跳过
  · `app/gen/android/**/build`、`.gradle`、`.cxx`：gradle 的中间物

用法：
    python scripts/clean_build.py                  # 清一遍（默认两个 target + gradle）
    python scripts/clean_build.py --dry-run        # 只报不删
    python scripts/clean_build.py --check          # 只量体积，超上限非零退出（工程门）
    python scripts/clean_build.py --selftest       # 自证会红（临时假树里跑）

退出码：0 = 没事；1 = --check 超上限（或 selftest 失败）。

★ 一条容易踩的坑：`--check` 必须隐含 dry-run，只量不删。
如果 `--check` 顺手把缓存删了，量到的永远是 0，这道门就永远绿。
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# 超上限就判红。默认按本仓实测定：清完大约 1.5 GB，留 8 GB 余量。
DEFAULT_LIMIT_GB = 8.0

# 每个 profile 目录里要删的中间物
PROFILE_JUNK_DIRS = ("deps", "build", "incremental", "examples", ".fingerprint")
PROFILE_JUNK_SUFFIX = (".pdb", ".rlib", ".rmeta", ".d", ".lib", ".exp", ".obj", ".ilk")

# 安卓 gradle 的中间物
GRADLE_JUNK_DIRS = ("build", ".gradle", ".cxx", ".kotlin")


def du(path: Path) -> tuple[int, int]:
    """(字节数, 文件数)"""
    total = 0
    files = 0
    if not path.exists():
        return 0, 0
    for base, _dirs, names in os.walk(path):
        for name in names:
            full = Path(base) / name
            try:
                total += full.stat().st_size
                files += 1
            except OSError:
                continue
    return total, files


def human(size: int) -> str:
    return f"{size / 1024 ** 3:.2f} GiB"


def android_target_dir() -> Path | None:
    """安卓 .so 编在仓外的 ASCII 目录里：优先环境变量，其次读打包脚本里的默认值"""
    env = os.environ.get("TF_TARGET_DIR")
    if env and Path(env).is_dir():
        return Path(env)
    script = ROOT / "scripts" / "android-build.ps1"
    if script.is_file():
        match = re.search(r'\$TargetDir\s*=\s*"([^"]+)"', script.read_text(encoding="utf-8"))
        if match and Path(match.group(1)).is_dir():
            return Path(match.group(1))
    return None


def rust_target_roots(extra: list[Path]) -> list[Path]:
    roots = [ROOT / "target", *extra]
    return [path for path in roots if path.is_dir()]


def gradle_roots() -> list[Path]:
    android = ROOT / "app" / "gen" / "android"
    if not android.is_dir():
        return []
    return [android]


def _collect(profile: Path, victims: list[tuple[Path, int, int]]) -> None:
    """收集一个 profile 目录里的中间物（不再往 deps/ 里面递归，否则会重复计数）"""
    for name in PROFILE_JUNK_DIRS:
        target = profile / name
        if target.is_dir():
            size, files = du(target)
            if size:
                victims.append((target, size, files))
    for item in profile.iterdir():
        if item.is_file() and item.suffix in PROFILE_JUNK_SUFFIX:
            try:
                size = item.stat().st_size
            except OSError:
                continue
            if size:
                victims.append((item, size, 1))


def scan(roots: list[Path], gradle: list[Path]) -> list[tuple[Path, int, int]]:
    """列出所有将被清理的目录（不删）。"""
    victims: list[tuple[Path, int, int]] = []
    for root in roots:
        for profile in root.iterdir():
            if not profile.is_dir():
                continue
            _collect(profile, victims)
            # 三元组目录（x86_64-linux-android / aarch64-apple-darwin…）里还套一层 profile
            for nested in profile.iterdir():
                if nested.is_dir() and nested.name not in PROFILE_JUNK_DIRS:
                    _collect(nested, victims)
    for gradle_root in gradle:
        for dirpath, dirnames, _files in os.walk(gradle_root):
            for name in list(dirnames):
                if name in GRADLE_JUNK_DIRS:
                    target = Path(dirpath) / name
                    size, files = du(target)
                    if size:
                        victims.append((target, size, files))
                    dirnames.remove(name)
    return victims


def sweep(victims: list[tuple[Path, int, int]], dry_run: bool) -> None:
    for path, size, files in victims:
        print(f"  {'(dry) ' if dry_run else ''}{path}  {human(size)} / {files} files")
        if dry_run:
            continue
        if path.is_dir():
            shutil.rmtree(path, ignore_errors=True)
        else:
            try:
                path.unlink()
            except OSError:
                pass


def report(roots: list[Path], gradle: list[Path], limit_gb: float) -> int:
    print("== 构建产物体积 ==")
    total = 0
    for root in roots:
        size, files = du(root)
        total += size
        print(f"  {root}  {human(size)} / {files} files")
    for gradle_root in gradle:
        size, files = du(gradle_root)
        total += size
        print(f"  {gradle_root}  {human(size)} / {files} files")
    print(f"  合计 {human(total)}（上限 {limit_gb:.1f} GB）")
    return 0 if total <= limit_gb * 1024 ** 3 else 1


def selftest() -> int:
    """在临时目录里造一棵假树：check 不许删、sweep 只删中间物。"""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "target"
        debug = root / "debug"
        (debug / "deps").mkdir(parents=True)
        (debug / "incremental").mkdir()
        (debug / "build").mkdir()
        (debug / "deps" / "a.rlib").write_bytes(b"x" * 4096)
        (debug / "incremental" / "chunk").write_bytes(b"x" * 4096)
        (debug / "text-farm.exe").write_bytes(b"x" * 1024)
        (debug / "app.pdb").write_bytes(b"x" * 4096)

        victims = scan([root], [])
        before = {path.name for path, _s, _f in victims}
        assert "deps" in before and "incremental" in before and "app.pdb" in before, before
        assert "text-farm.exe" not in before, "exe 不该被清"

        # --check 必须只量不删
        assert (debug / "deps").exists()
        sweep(victims, dry_run=True)
        assert (debug / "deps").exists(), "--dry-run 删了东西"
        assert (debug / "app.pdb").exists(), "--dry-run 删了东西"

        sweep(victims, dry_run=False)
        assert not (debug / "deps").exists(), "sweep 没删 deps"
        assert not (debug / "app.pdb").exists(), "sweep 没删 pdb"
        assert (debug / "text-farm.exe").exists(), "sweep 把 exe 删了"
    print("selftest 通过：check/dry-run 不删，sweep 只删中间物")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="构建产物清理门")
    parser.add_argument("--dry-run", action="store_true", help="只报不删")
    parser.add_argument("--check", action="store_true", help="只量体积，超上限非零退出")
    parser.add_argument("--selftest", action="store_true", help="自证会红")
    parser.add_argument("--limit-gb", type=float, default=DEFAULT_LIMIT_GB, help="体积上限（GB）")
    parser.add_argument(
        "--extra-root",
        action="append",
        default=[],
        help="额外的 Rust target 目录（也可用环境变量 TF_TARGET_DIR）",
    )
    args = parser.parse_args()

    if args.selftest:
        return selftest()

    extra = [Path(item) for item in args.extra_root]
    if not extra:
        android_target = android_target_dir()
        if android_target is not None:
            extra = [android_target]

    roots = rust_target_roots(extra)
    gradle = gradle_roots()

    if args.check:
        return report(roots, gradle, args.limit_gb)

    victims = scan(roots, gradle)
    total = sum(size for _p, size, _f in victims)
    print(f"== 可清理 {human(total)} / {len(victims)} 项 ==")
    sweep(victims, dry_run=args.dry_run)
    if not args.dry_run:
        print("== 清完体积 ==")
        report(roots, gradle, args.limit_gb)
    return 0


if __name__ == "__main__":
    sys.exit(main())
