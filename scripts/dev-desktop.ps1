# 开发版桌面壳：热重载 + devtools + 前端 DEV 钩子（window.__tfTilt 这类）
#
# 正式版只从 tag 出（cargo tauri build 走 release.yml），这个是本地用的：
#   .\scripts\dev-desktop.ps1
#
# 它会先起 Vite（1420），再让 Tauri 窗口连过去；改前端立刻生效。

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host '== 开发版：cargo tauri dev ==' -ForegroundColor Cyan
cargo tauri dev
