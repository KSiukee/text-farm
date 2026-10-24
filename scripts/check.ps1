$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

function Invoke-Gate {
    param(
        [string]$Exe,
        [string[]]$Arguments
    )
    & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) {
        $code = $LASTEXITCODE
        Write-Host "FAILED: $Exe $($Arguments -join ' ')" -ForegroundColor Red
        exit $code
    }
}

Push-Location $root
try {
    $env:PYTHONIOENCODING = "utf-8"
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8

    # 两个门卫先跑：版本号 / 构建缓存不通过就不必再编。
    Invoke-Gate -Exe "node" -Arguments @("scripts/version-guard.mjs")
    Invoke-Gate -Exe "python" -Arguments @("scripts/cache_guard.py", "--check")
    Invoke-Gate -Exe "python" -Arguments @("scripts/cache_guard.py", "--selftest")

    Invoke-Gate -Exe "cargo" -Arguments @("test", "--workspace")

    Push-Location "app/frontend"
    try {
        Invoke-Gate -Exe "npm" -Arguments @("test")
    }
    finally {
        Pop-Location
    }
}
finally {
    Pop-Location
}
