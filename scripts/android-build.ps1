# Android debug APK 构建（Tauri 2）
#
# 为什么不用 `cargo tauri android build`：
#   1) 它在 jniLibs 里给 .so 建符号链接，Windows 没开开发者模式会直接失败；
#   2) 本项目在中文路径下，NDK 的 clang.cmd 用 GBK 转参数，链接器认不出路径。
# 本脚本改成：ASCII 的 target 目录里编 .so → 直接复制进 jniLibs → 交给 gradle 打包。
#
# 用法：
#   powershell -File scripts/android-build.ps1                     只出 APK
#   powershell -File scripts/android-build.ps1 -Install -Start     出包 + 装到设备 + 启动
#   powershell -File scripts/android-build.ps1 -Arch aarch64       给真机（arm64）
param(
    [ValidateSet("x86_64", "aarch64", "armv7", "i686")]
    [string]$Arch = "x86_64",
    # 必须是不含中文的路径（NDK 链接器只吃 ASCII）
    [string]$TargetDir = "D:\tf-target",
    [switch]$Install,
    [switch]$Start,
    # 保留 Rust 调试符号（默认剥掉：APK 370MB → ~40MB，装得快、真机也友好）
    [switch]$KeepSymbols
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$appDir = Join-Path $root "app"
$genAndroid = Join-Path $appDir "gen\android"

# ---- 环境 ----
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { "D:\deepseek-files\tools\android-sdk" }
$jdk = if ($env:JAVA_HOME) { $env:JAVA_HOME } else { "D:\deepseek-files\tools\jdk-17" }
if (-not (Test-Path $sdk)) { throw "找不到 Android SDK：$sdk（设 ANDROID_HOME 或改本脚本默认值）" }
if (-not (Test-Path $jdk)) { throw "找不到 JDK：$jdk（设 JAVA_HOME 或改本脚本默认值）" }
$ndkRoot = Join-Path $sdk "ndk"
$ndk = Get-ChildItem $ndkRoot -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
if (-not $ndk) { throw "找不到 NDK：$ndkRoot" }

if ($TargetDir -match "[^\x00-\x7F]") { throw "TargetDir 不能含中文：$TargetDir" }
New-Item -ItemType Directory -Force $TargetDir | Out-Null

$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:NDK_HOME = $ndk.FullName
$env:JAVA_HOME = $jdk
$env:CARGO_TARGET_DIR = $TargetDir
$env:PATH = "$env:PATH;$jdk\bin;$(Join-Path $sdk 'platform-tools')"

# ---- 架构映射：rust target / gradle flavor / jniLibs ABI ----
$map = @{
    x86_64  = @{ Rust = "x86_64-linux-android";    Clang = "x86_64-linux-android24-clang.cmd";    Flavor = "x86_64"; ABI = "x86_64" }
    aarch64 = @{ Rust = "aarch64-linux-android";   Clang = "aarch64-linux-android24-clang.cmd";   Flavor = "arm64";  ABI = "arm64-v8a" }
    armv7   = @{ Rust = "armv7-linux-androideabi"; Clang = "armv7a-linux-androideabi24-clang.cmd"; Flavor = "arm";  ABI = "armeabi-v7a" }
    i686    = @{ Rust = "i686-linux-android";      Clang = "i686-linux-android24-clang.cmd";      Flavor = "x86";    ABI = "x86" }
}
$spec = $map[$Arch]
$toolBin = Join-Path $ndk.FullName "toolchains\llvm\prebuilt\windows-x86_64\bin"
$clang = Join-Path $toolBin $spec.Clang
if (-not (Test-Path $clang)) { throw "找不到 NDK 编译器：$clang" }

$rustEnv = $spec.Rust.ToUpper().Replace("-", "_")
Set-Item -Path "Env:CC_$($spec.Rust.Replace('-', '_'))" -Value $clang
Set-Item -Path "Env:AR_$($spec.Rust.Replace('-', '_'))" -Value (Join-Path $toolBin "llvm-ar.exe")
Set-Item -Path "Env:CARGO_TARGET_$($rustEnv)_LINKER" -Value $clang

# ---- 首次：生成 Android 工程 ----
if (-not (Test-Path $genAndroid)) {
    Write-Host "== 首次生成 Android 工程 ==" -ForegroundColor Cyan
    Push-Location $appDir
    try { cargo tauri android init --ci } finally { Pop-Location }
}

# ---- 把 Tauri 插件同步进 gradle 工程 ----
# `tauri android init` 只写它当时认得的那批插件；后加的（如 tauri-plugin-fs）要自己补，
# 否则 APK 里没有对应的 Kotlin 类，启动直接崩：
#   ClassNotFoundException: com.plugin.fs.FsPlugin → Rust 侧 panic → SIGABRT
$cargoToml = Get-Content (Join-Path $appDir "Cargo.toml") -Raw
$pluginNames = [regex]::Matches($cargoToml, '(?m)^(tauri-plugin-[\w-]+)\s*=') | ForEach-Object { $_.Groups[1].Value } | Select-Object -Unique
if ($pluginNames) {
    $cargoHomeDir = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE ".cargo" }
    $registrySrc = Get-ChildItem (Join-Path $cargoHomeDir "registry\src") -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($registrySrc) {
        $settingsPath = Join-Path $genAndroid "tauri.settings.gradle"
        $settingsTxt = [IO.File]::ReadAllText($settingsPath)
        $buildKtsPath = Join-Path $genAndroid "app\tauri.build.gradle.kts"
        $buildKts = [IO.File]::ReadAllText($buildKtsPath)
        $added = @()
        foreach ($name in $pluginNames) {
            $dir = Get-ChildItem $registrySrc.FullName -Directory -Filter "$name-*" | Select-Object -Last 1
            if (-not $dir -or -not (Test-Path (Join-Path $dir.FullName "android"))) { continue }
            if ($settingsTxt -notmatch [regex]::Escape("include ':$name'")) {
                $p = (Join-Path $dir.FullName "android") -replace '\\', '/'
                $settingsTxt = $settingsTxt.TrimEnd() + "`ninclude ':$name'`nproject(':$name').projectDir = new File(`"$p`")`n"
                $added += $name
            }
            if ($buildKts -notmatch [regex]::Escape("project(`":$name`")")) {
                $buildKts = $buildKts -replace '(implementation\(project\(":tauri-plugin-[\w-]+"\)\))', "`$1`n  implementation(project(`":$name`"))"
                if ($added -notcontains $name) { $added += $name }
            }
        }
        if ($added.Count -gt 0) {
            [IO.File]::WriteAllText($settingsPath, $settingsTxt, [Text.UTF8Encoding]::new($false))
            [IO.File]::WriteAllText($buildKtsPath, $buildKts, [Text.UTF8Encoding]::new($false))
            Write-Host "== 补进 gradle 的插件：$($added -join ', ') ==" -ForegroundColor Cyan
        }
    }
}

# AGP 默认拒绝非 ASCII 项目路径；本机实测可用，补开关（gen/ 是生成物，重新 init 会丢，所以每次都补）
$gradleProps = Join-Path $genAndroid "gradle.properties"
if ((Get-Content $gradleProps -Raw) -notmatch "android.overridePathCheck") {
    Add-Content -Path $gradleProps -Value "`nandroid.overridePathCheck=true" -Encoding utf8
}

# ---- 前端产物 ----
# 前端 dist 会被嵌进 .so 的 custom-protocol 资源里；不跑这步，改了前端也不会生效。
Write-Host "== 构建前端 ==" -ForegroundColor Cyan
Push-Location (Join-Path $appDir "frontend")
try {
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "npm run build 失败" }
} finally { Pop-Location }

# ---- 编 Rust 动态库 ----
Write-Host "== 编 libtext_farm_lib.so（$Arch）==" -ForegroundColor Cyan
Push-Location $appDir
try {
    cargo build --package text-farm --target $spec.Rust --features tauri/custom-protocol --lib
    if ($LASTEXITCODE -ne 0) { throw "cargo build 失败" }
} finally { Pop-Location }

$so = Join-Path $TargetDir (Join-Path $spec.Rust "debug\libtext_farm_lib.so")
if (-not (Test-Path $so)) { throw "找不到 .so：$so" }
$jniDir = Join-Path $genAndroid "app\src\main\jniLibs\$($spec.ABI)"
New-Item -ItemType Directory -Force $jniDir | Out-Null
$jniSo = Join-Path $jniDir "libtext_farm_lib.so"
Copy-Item $so $jniSo -Force
if (-not $KeepSymbols) {
    $strip = Join-Path $toolBin "llvm-strip.exe"
    if (Test-Path $strip) { & $strip --strip-debug --strip-unneeded $jniSo }
}
Write-Host ("   .so {0:N0} MB（进包 {1:N0} MB）→ {2}" -f ((Get-Item $so).Length / 1MB), ((Get-Item $jniSo).Length / 1MB), $jniDir)

# ---- APK 版本号 ----
# gradle 读的是 app/tauri.properties；这条平时由 tauri CLI 写，我们直接跑 gradle 就得自己写。
$conf = Get-Content (Join-Path $appDir "tauri.conf.json") -Raw | ConvertFrom-Json
$ver = $conf.version
$nums = $ver.Split(".")
$code = [int]$nums[0] * 1000000 + [int]$nums[1] * 1000 + [int]$nums[2]
$props = Join-Path $genAndroid "app\tauri.properties"
[IO.File]::WriteAllText($props, "tauri.android.versionName=$ver`ntauri.android.versionCode=$code`n", [Text.UTF8Encoding]::new($false))
Write-Host "== APK 版本 $ver（versionCode $code）==" -ForegroundColor Cyan

# ---- gradle 打包（跳过它自己的 rust 任务，.so 已经手动放好）----
Write-Host "== gradle assemble$($spec.Flavor)Debug ==" -ForegroundColor Cyan
Push-Location $genAndroid
try {
    & .\gradlew.bat ":app:assemble$($spec.Flavor)Debug" "-x" ":app:rustBuild$($spec.Flavor)Debug" --console=plain
    if ($LASTEXITCODE -ne 0) { throw "gradle 打包失败" }
} finally { Pop-Location }

$apk = Join-Path $genAndroid "app\build\outputs\apk\$($spec.Flavor)\debug\app-$($spec.Flavor)-debug.apk"
if (-not (Test-Path $apk)) { throw "找不到 APK：$apk" }
Write-Host ("== APK：{0}（{1:N1} MB）" -f $apk, ((Get-Item $apk).Length / 1MB)) -ForegroundColor Green

$adb = Join-Path $sdk "platform-tools\adb.exe"
if ($Install) {
    Write-Host "== 安装到设备 ==" -ForegroundColor Cyan
    & $adb install -r $apk
    if ($LASTEXITCODE -ne 0) { throw "adb install 失败" }
}
if ($Start) {
    Write-Host "== 启动 ==" -ForegroundColor Cyan
    & $adb shell am start -n "app.textfarm.desktop/.MainActivity"
}
