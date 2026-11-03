# 启用仓库里的 git hooks（发布棘轮）
#
#   .\scripts\install-hooks.ps1
#
# 作用：git config core.hooksPath .githooks
# 之后 push tag 会先跑 scripts/release-check.py，不过就推不出去。
# 换机器/重新 clone 之后跑一次即可（配置在本地 .git/config，不进库）。

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

git config core.hooksPath .githooks
if ($LASTEXITCODE -ne 0) { throw 'git config 失败' }

Write-Host '✓ 已启用 .githooks（push tag 会先过发布前闸门）' -ForegroundColor Green
Write-Host '  只想手动跑：python scripts/release-check.py' -ForegroundColor DarkGray
