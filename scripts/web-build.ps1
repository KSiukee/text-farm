# 网页版一把构建：wasm 内核 → 前端 public → 前端 dist
#
# 用法：.\scripts\web-build.ps1
# 产物：app/frontend/dist（整个目录就是网页版，传到 Pages 即可）

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host '[1/3] 编译 wasm 内核…'
cargo build -p farm-wasm --target wasm32-unknown-unknown --release
if ($LASTEXITCODE -ne 0) { throw 'wasm 编译失败' }

Write-Host '[2/3] 同步 wasm 到前端…'
Copy-Item 'target\wasm32-unknown-unknown\release\farm_wasm.wasm' 'app\frontend\public\farm_wasm.wasm' -Force

Write-Host '[3/3] 构建前端…'
Push-Location app\frontend
try {
    npm run build
    if ($LASTEXITCODE -ne 0) { throw '前端构建失败' }
} finally {
    Pop-Location
}

$dist = 'app\frontend\dist'
$size = [math]::Round(((Get-ChildItem $dist -Recurse -File | Measure-Object Length -Sum).Sum / 1MB), 2)
$wasm = [math]::Round((Get-Item "$dist\farm_wasm.wasm").Length / 1MB, 2)
Write-Host "好了：$dist 共 $size MB（其中 wasm $wasm MB，gzip 后约 0.75 MB）"
