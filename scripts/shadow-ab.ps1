param(
    [switch]$Refresh
)

$root = Split-Path -Parent $PSScriptRoot
$realSave = Join-Path $env:APPDATA "app.textfarm.desktop\save.json"
$abRoot = Join-Path $root "shadow\ab"
$base = Join-Path $abRoot "base.json"

New-Item -ItemType Directory -Path $abRoot -Force | Out-Null
if ($Refresh -or -not (Test-Path -LiteralPath $base)) {
    if (-not (Test-Path -LiteralPath $realSave)) {
        throw "找不到真机存档：$realSave"
    }
    Copy-Item -LiteralPath $realSave -Destination $base -Force
    Write-Host "A/B 基线来自真机档副本：$base（真机档不动）"
}

foreach ($profile in @("A", "B")) {
    $dir = Join-Path $abRoot $profile
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Copy-Item -LiteralPath $base -Destination (Join-Path $dir "save.json") -Force
    Write-Host "影子 $profile：$dir"
}

Write-Host ""
Write-Host "A 组： powershell -File scripts\shadow-run.ps1 -Profile ab\A"
Write-Host "B 组： powershell -File scripts\shadow-run.ps1 -Profile ab\B"
Write-Host "两组各自读写自己的 save.json，不会碰真机档。"
