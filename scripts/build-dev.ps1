# 调试版桌面包：debug 构建（带 devtools），产出 文字农场-dev.exe
#
# 和正式版的区别：
#   - debug 构建，体积大、性能差，但能开 devtools（F12）
#   - 文件名带 -dev，不会和正式版混淆
#   - 存档目录相同（app.textfarm.desktop），测试时注意别把正式档改坏
#
# 注意：这是"生产前端 + debug 壳"，没有 import.meta.env.DEV，
# 所以 window.__tfTilt 这类开发钩子只在 `cargo tauri dev` 里存在。

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host '== 构建 debug 版 ==' -ForegroundColor Cyan
cargo tauri build --debug --no-bundle
if ($LASTEXITCODE -ne 0) { throw 'debug 构建失败' }

# 目标文件可能正在运行（占用中），那就换一个带时间戳的名字，别让构建白跑
$target = '文字农场-dev.exe'
try {
    Copy-Item 'target\debug\text-farm.exe' $target -Force -ErrorAction Stop
} catch {
    $target = "文字农场-dev-$(Get-Date -Format 'HHmm').exe"
    Copy-Item 'target\debug\text-farm.exe' $target -Force
    Write-Host "原 dev 版正在运行，新构建写到 $target（关掉旧的再启动它）" -ForegroundColor Yellow
}
$size = [math]::Round((Get-Item $target).Length / 1MB, 1)
Write-Host "好了：$target（$size MB，带 devtools）" -ForegroundColor Green
