# 第三方声明

## 运行时依赖

| 依赖 | 许可 | 用途 |
|---|---|---|
| [Tauri](https://tauri.app/) | MIT / Apache-2.0 | 桌面与安卓外壳 |
| [tauri-plugin-dialog](https://github.com/tauri-apps/plugins-workspace) | MIT / Apache-2.0 | 文件选择框 |
| [tauri-plugin-fs](https://github.com/tauri-apps/plugins-workspace) | MIT / Apache-2.0 | 安卓 content:// 读写 |
| [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json) | MIT / Apache-2.0 | 存档与内容包序列化 |
| [thiserror](https://github.com/dtolnay/thiserror) | MIT / Apache-2.0 | 错误类型 |
| [chrono](https://github.com/chronotope/chrono) | MIT / Apache-2.0 | 桌面壳的本地日期 |
| [parking_lot](https://github.com/Amanieu/parking_lot) | MIT / Apache-2.0 | 壳内互斥 |
| [base64](https://github.com/marshallpierce/rust-base64) | MIT / Apache-2.0 | 壳内编码工具 |
| [jni](https://github.com/jni-rs/jni-rs) | MIT / Apache-2.0 | 安卓全屏横屏（仅 Android 目标） |
| [Vue 3](https://vuejs.org/) | MIT | 前端框架 |
| [@tauri-apps/api](https://github.com/tauri-apps/tauri) | MIT / Apache-2.0 | 前端调壳 |
| [Vite](https://vite.dev/) | MIT | 构建 |

浏览器版把 Rust 内核编成 WebAssembly：`crates/farm-wasm` 只用 C ABI 传指针，**不依赖 wasm-bindgen**。

## 美术与文案

| 内容 | 来源 | 说明 |
|---|---|---|
| 13 张宠物图（`app/frontend/public/pets/`） | 本地 ComfyUI 生成（FLUX.2 Klein 4B） | 非手绘；模型为 Apache-2.0 |
| 四张钓鱼水域背景（`.../backgrounds/`） | `scripts/gen_fishing_bg.py` 用 Pillow 按固定种子生成 | 可复现，仓库内脚本即来源 |
| 鱼、家具、房间精灵（`.../fish/`、`.../sprites/`、`.../windows/`） | 自制 | 像素管线见 `docs/`（不随仓库公开） |
| 字体 | 系统字体 + 玩家自选本机字体 | **不内置任何商业字体**；上传的字体只存在玩家本机 IndexedDB |

## 许可

- **代码**：GPL-3.0-or-later（见 `LICENSE`）
- **美术素材与文案**：CC BY-NC-SA 4.0 —— 可以转载、修改，但**不得商用**，且需署名并以相同方式共享。

商用授权请开 Issue 联系。
