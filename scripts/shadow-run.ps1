param(
    [switch]$CopyReal,
    [switch]$Fresh,
    [string]$Profile = "data"
)

$root = Split-Path -Parent $PSScriptRoot
$shadow = Join-Path $root "shadow\$Profile"
$exe = Join-Path $root "文字农场.exe"
$realDir = Join-Path $env:APPDATA "app.textfarm.desktop"
$realSave = Join-Path $realDir "save.json"

if ($Fresh -and (Test-Path -LiteralPath $shadow)) {
    $resolved = (Resolve-Path -LiteralPath $shadow).Path
    $expected = [System.IO.Path]::GetFullPath($shadow)
    if ($resolved -ne $expected -or -not $resolved.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "拒绝清理非项目影子目录：$resolved"
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
New-Item -ItemType Directory -Path $shadow -Force | Out-Null

$shadowSave = Join-Path $shadow "save.json"
if ($CopyReal -and (Test-Path -LiteralPath $realSave)) {
    Copy-Item -LiteralPath $realSave -Destination $shadowSave -Force
    Write-Host "已把真机档复制到影子库（真机档不动）：$shadowSave"
} elseif (-not (Test-Path -LiteralPath $shadowSave)) {
    Write-Host "影子库没有存档；用 -CopyReal 复制真机档，或直接启动会新建。"
}

$env:TEXT_FARM_DATA_DIR = $shadow
Write-Host "影子数据目录：$shadow"
Write-Host "启动：$exe"
& $exe
