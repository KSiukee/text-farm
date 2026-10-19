$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
node (Join-Path $PSScriptRoot "ui-smoke.mjs")
exit $LASTEXITCODE
