#!/usr/bin/env python3
"""生成 GitHub Release 的说明文字。

结构：安装方式（Win / macOS / 网页）+ 本次更新 + 校验提示。
「本次更新」优先读 CHANGELOG.md 里对应版本那一行；
CHANGELOG 还没记这一版时，退回 git log（上一个 tag 到 HEAD），
并过滤掉纯工程提交，只留玩家能感知的。

用法：
    python scripts/release-notes.py            # 打到 stdout
    python scripts/release-notes.py --out FILE # 写文件（CI 用）
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

# 纯工程提交，不进玩家面向的更新说明
NOISE = (
    "CI：", "闸门", "gitignore", "重构", "测试：", "chore", "docs:", "changelog",
    "脱敏", "棘轮", "开发链路", "版本号", "发版", "workflow",
)


def read_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"', text, re.M)
    return match.group(1) if match else "0.0.0"


def changelog_line(version: str) -> str | None:
    path = ROOT / "CHANGELOG.md"
    if not path.is_file():
        return None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith(f"| {version} |"):
            cells = [cell.strip() for cell in line.strip("|").split("|")]
            if len(cells) >= 3 and cells[2]:
                return cells[2]
    return None


def git(*args: str) -> str:
    out = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    return out.stdout.strip()


def commits_since_last_tag(version: str) -> list[str]:
    last = git("describe", "--tags", "--abbrev=0", f"v{version}^")
    if not last:
        return []
    log = git("log", f"{last}..HEAD", "--no-merges", "--pretty=%s")
    picked: list[str] = []
    for subject in log.splitlines():
        subject = subject.strip()
        if not subject or any(noise in subject for noise in NOISE):
            continue
        picked.append(subject)
    return picked[:12]


def build(version: str) -> str:
    line = changelog_line(version)
    body = [
        f"**Windows 免安装版**：下载 `text-farm-{version}-win64.exe`，双击就能跑。",
        "首次运行 Windows 会提示「已保护你的电脑」——exe 没买代码签名证书，",
        "点「更多信息 → 仍要运行」就行。",
        "",
        f"**macOS**：下载 `text-farm-{version}-macos-universal.dmg`（Intel / Apple 芯片通用）。",
        "没买 Apple 开发者账号、没做公证，首次打开会被拦：",
        "右键 →「打开」，或者终端跑一句",
        "`xattr -dr com.apple.quarantine /Applications/文字农场.app`。",
        "",
        "**网页版**（不用下载）：https://ksiukee.github.io/text-farm/",
        "",
        "---",
        "",
        "### 本次更新",
        "",
    ]
    if line:
        body.append(f"- {line}")
    else:
        picked = commits_since_last_tag(version)
        if picked:
            body.extend(f"- {item}" for item in picked)
        else:
            body.append("- 见 [CHANGELOG](CHANGELOG.md)")
    body.extend(
        [
            "",
            "### 说明",
            "",
            "- 存档三端通用（Windows / macOS / 安卓 / 网页），设置里可以导出导入。",
            "- 安卓包不在这里，见网盘或仓库的安卓构建说明。",
            "- 有问题请开 [Issue](https://github.com/KSiukee/text-farm/issues)。",
        ]
    )
    return "\n".join(body) + "\n"


def main() -> int:
    version = read_version()
    text = build(version)
    if "--out" in sys.argv:
        out = Path(sys.argv[sys.argv.index("--out") + 1])
        out.write_text(text, encoding="utf-8")
        print(f"写好 {out}")
    else:
        print(text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
