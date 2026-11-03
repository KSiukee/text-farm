# 贡献指南

先谢谢你愿意花时间。这个项目是一个人做的单机游戏，规则不复杂，但有几条线要先说清楚。

## 先开 Issue 再动手

**除了错别字和一眼就能看出来的 bug，动手前先开个 Issue。**
尤其是新玩法、重构、依赖升级——先聊两句，能省掉你写一天然后被拒的麻烦。

## 收什么样的 PR

| 类型 | 收不收 | 说明 |
|---|---|---|
| Bug 修复 | ✅ | 带复现步骤最好 |
| 平台兼容（Windows / Android / 浏览器） | ✅ | 说明系统和版本 |
| 文档、错别字、注释 | ✅ | |
| 性能问题 | ✅ | 请带上「怎么量的」 |
| 数值 / 平衡调整 | ❌ | 这游戏的数值是几十次 90 天模拟调出来的，单个 PR 没法验证 |
| 新玩法 / 新系统 | ⚠️ | 先开 Issue 谈设计，别直接写 |
| 美术素材 | ❌ | 来源和授权没法核实 |
| 大规模重构 | ⚠️ | 先谈，风格分歧很常见 |

## 提交前

```bash
cargo test --workspace        # 全量测试
cd app/frontend && npm run test && npm run typecheck
```

- Rust 侧：`cargo fmt`，别引入新的 `unwrap()`（测试里可以）。
- 注释用中文，写「为什么」，不写「做了什么」。
- 一个 PR 只做一件事。

## 发版（机器管，不靠记性）

正式包只在打 tag 时产出，而且要先过**发布前闸门**：

```bash
python scripts/release-check.py     # 手动跑
# 或者直接 push tag，pre-push hook 会替你先跑一遍
git tag vX.Y.Z && git push origin vX.Y.Z
```

闸门不过，tag 推不出去（本地 hook）+ 出了包也没产物（CI 的 `gate` job 卡在构建前面）。

检查项：

| 项 | 为什么 |
|---|---|
| 工作树干净 | 正式包必须来自已提交的状态 |
| 版本号四处一致 | Cargo / tauri.conf / package.json / README 写岔过 |
| 版本号不低于上个 tag | 防止把旧版本号再发一次 |
| CHANGELOG 与内部日志一致 | 手改公开的 CHANGELOG 会漂 |
| 公开内容脱敏 0 命中 | 内部路径/项目名不进公开仓库 |
| 内部资料、构建产物没被跟踪 | `.exe`/`.apk`/内部文档不该进库 |

装 hook（每台机器一次）：

```powershell
.\scripts\install-hooks.ps1
```

安卓正式包同样过闸门：

```powershell
.\scripts\android-build.ps1 -Arch aarch64 -Release    # 会先跑 release-check
```

只想要测试包就别带 `-Release`，debug 包不受闸门约束。

## 许可

提交即表示你同意以 **GPL-3.0-or-later** 授权你的贡献（inbound = outbound，不需要签 CLA）。

## 别做的事

- 别把存档格式改坏。玩家手里的档比代码重要。
- 别为了「更优雅」动 `farm-core` 的时间注入和文件系统隔离——那是可测试性的地基。
