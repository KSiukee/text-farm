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
    [switch]$KeepSymbols,
    # ---- 发布：release 构建 + 正式签名 ----
    # keystore 放项目外（默认 D:\deepseek-files\_keys\text-farm-release.jks），密码在同目录
    # keystore.properties（都不入库）。第一次跑没有会自动生成——**这文件丢了就没法给老用户升级**。
    [switch]$Release,
    [string]$KeyDir = "D:\deepseek-files\_keys",
    # 同时出 AAB（用 jarsigner 签，Google Play 上传那种）
    [switch]$Aab,
    # 跳过发布前闸门（只该在确认过闸门结果的情况下用）
    [switch]$SkipGate
)

$ErrorActionPreference = "Stop"

# ---- 发布棘轮：release 包先过闸门 ----
# 正式包是给人装的，不能凭记性。debug 包不拦（测试用）。
if ($Release -and -not $SkipGate) {
    Write-Host "== release 包：先跑发布前闸门 ==" -ForegroundColor Cyan
    python scripts\release-check.py
    if ($LASTEXITCODE -ne 0) {
        throw "发布前闸门没过，拒绝打 release 包。测试请用 debug（去掉 -Release）。"
    }
}
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

$buildTools = Get-ChildItem (Join-Path $sdk "build-tools") -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
$zipalign = if ($buildTools) { Join-Path $buildTools.FullName "zipalign.exe" } else { $null }
$apksigner = if ($buildTools) { Join-Path $buildTools.FullName "apksigner.bat" } else { $null }

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

$profile = if ($Release) { "release" } else { "debug" }
$profileCap = if ($Release) { "Release" } else { "Debug" }

# ---- release 签名用的 keystore（放项目外，不入库）----
$keystoreProps = $null
$ksFile = $null
if ($Release) {
    if (-not (Test-Path $KeyDir)) { New-Item -ItemType Directory -Force $KeyDir | Out-Null }
    $propFile = Join-Path $KeyDir "keystore.properties"
    $ksFile = Join-Path $KeyDir "text-farm-release.jks"
    if (Test-Path $propFile) {
        $keystoreProps = ConvertFrom-StringData (Get-Content $propFile -Raw)
        if (-not (Test-Path $ksFile)) { throw "有密码文件却没有 keystore：$ksFile（无法签发布包）" }
    } else {
        $chars = [char[]]"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        $pass = -join (1..24 | ForEach-Object { $chars | Get-Random })
        Write-Host "== 首次生成发布 keystore：$ksFile ==" -ForegroundColor Yellow
        & keytool -genkeypair -v -keystore $ksFile -alias textfarm -keyalg RSA -keysize 2048 `
            -validity 10000 -storetype PKCS12 -storepass $pass -keypass $pass `
            -dname "CN=Text Farm, OU=Personal, O=Personal, L=Unknown, ST=Unknown, C=CN"
        if ($LASTEXITCODE -ne 0) { throw "keytool 生成 keystore 失败" }
        [IO.File]::WriteAllText($propFile, "storeFile=text-farm-release.jks`nstorePassword=$pass`nkeyAlias=textfarm`nkeyPassword=$pass`n", [Text.UTF8Encoding]::new($false))
        $keystoreProps = ConvertFrom-StringData (Get-Content $propFile -Raw)
        Write-Host "   密码写在 $propFile —— 自己备份好，丢了就没法给老用户升级" -ForegroundColor Yellow
    }
}

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

# ---- 返回键：先问网页，再决定退不退 ----
# MainActivity 是 tauri android init 的生成物（gen/ 不进库、重新 init 会被覆盖），
# 所以每次构建都补一遍；已经补过就跳过。
$activity = Join-Path $genAndroid "app\src\main\java\app\textfarm\desktop\MainActivity.kt"
if ((Test-Path $activity) -and ((Get-Content $activity -Raw) -notmatch "__tfBack")) {
$kotlin = @'
package app.textfarm.desktop

import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  /**
   * 返回键先问网页：window.__tfBack() 返回 "handled" 就由网页关掉当前页面，
   * 返回其它（或者根本没有这个函数）才真的退出。
   */
  override fun onBackPressed() {
    val webView = findWebView(window.decorView)
    if (webView == null) {
      super.onBackPressed()
      return
    }
    webView.evaluateJavascript("(window.__tfBack ? window.__tfBack() : 'exit')") { result ->
      if (result == null || !result.contains("handled")) {
        runOnUiThread { exitFromBack() }
      }
    }
  }

  private fun exitFromBack() {
    super.onBackPressed()
  }

  private fun findWebView(view: View): WebView? {
    if (view is WebView) return view
    if (view is ViewGroup) {
      for (index in 0 until view.childCount) {
        val found = findWebView(view.getChildAt(index))
        if (found != null) return found
      }
    }
    return null
  }
}
'@
    [IO.File]::WriteAllText($activity, $kotlin, [Text.UTF8Encoding]::new($false))
    Write-Host "== 返回键：已给 MainActivity 补 onBackPressed ==" -ForegroundColor Cyan
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
Write-Host "== 编 libtext_farm_lib.so（$Arch / $profile）==" -ForegroundColor Cyan
Push-Location $appDir
try {
    $cargoArgs = @("build", "--package", "text-farm", "--target", $spec.Rust, "--features", "tauri/custom-protocol", "--lib")
    if ($Release) { $cargoArgs += "--release" }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build 失败" }
} finally { Pop-Location }

$so = Join-Path $TargetDir (Join-Path $spec.Rust "$profile\libtext_farm_lib.so")
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
$gradleTask = ":app:assemble$($spec.Flavor)$profileCap"
$rustTask = ":app:rustBuild$($spec.Flavor)$profileCap"
Write-Host "== gradle $gradleTask ==" -ForegroundColor Cyan
Push-Location $genAndroid
try {
    & .\gradlew.bat $gradleTask "-x" $rustTask --console=plain
    if ($LASTEXITCODE -ne 0) { throw "gradle 打包失败" }
} finally { Pop-Location }

if ($Release) {
    # 没配 signingConfig，gradle 出的是 unsigned；自己 zipalign + apksigner
    $unsigned = Join-Path $genAndroid "app\build\outputs\apk\$($spec.Flavor)\release\app-$($spec.Flavor)-release-unsigned.apk"
    if (-not (Test-Path $unsigned)) { throw "找不到 unsigned APK：$unsigned" }
    if (-not $zipalign -or -not (Test-Path $zipalign)) { throw "找不到 zipalign（build-tools）" }
    if (-not $apksigner -or -not (Test-Path $apksigner)) { throw "找不到 apksigner（build-tools）" }
    $dist = Join-Path $root "dist"
    New-Item -ItemType Directory -Force $dist | Out-Null
    $aligned = Join-Path ([IO.Path]::GetTempPath()) "tf-$($spec.Flavor)-aligned.apk"
    & $zipalign -p -f 4 $unsigned $aligned
    if ($LASTEXITCODE -ne 0) { throw "zipalign 失败" }
    $apk = Join-Path $dist "文字农场-$ver-$($spec.Flavor)-release.apk"
    if (Test-Path $apk) { Remove-Item $apk -Force }
    & $apksigner sign --ks $ksFile --ks-pass "pass:$($keystoreProps.storePassword)" --ks-key-alias $($keystoreProps.keyAlias) --key-pass "pass:$($keystoreProps.keyPassword)" --out $apk $aligned
    if ($LASTEXITCODE -ne 0) { throw "apksigner 签名失败" }
    Write-Host "== 验签 ==" -ForegroundColor Cyan
    & $apksigner verify --print-certs $apk
    if ($LASTEXITCODE -ne 0) { throw "签名校验失败" }

    if ($Aab) {
        Write-Host "== gradle bundle$($spec.Flavor)$profileCap ==" -ForegroundColor Cyan
        Push-Location $genAndroid
        try {
            & .\gradlew.bat ":app:bundle$($spec.Flavor)$profileCap" "-x" $rustTask --console=plain
            if ($LASTEXITCODE -ne 0) { throw "AAB 打包失败" }
        } finally { Pop-Location }
        $aabDir = Join-Path $genAndroid "app\build\outputs\bundle"
        $unsignedAab = Get-ChildItem $aabDir -Recurse -Filter "*.aab" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if (-not $unsignedAab) { throw "找不到 AAB（$aabDir）" }
        # 注意：PowerShell 变量不区分大小写，$aab 会和参数 [switch]$Aab 撞名 → 用 $aabPath
        $aabPath = Join-Path $dist "文字农场-$ver-$($spec.Flavor)-release.aab"
        if (Test-Path $aabPath) { Remove-Item $aabPath -Force }
        & jarsigner -keystore $ksFile -storepass $($keystoreProps.storePassword) -keypass $($keystoreProps.keyPassword) -signedjar $aabPath $unsignedAab.FullName $($keystoreProps.keyAlias)
        if ($LASTEXITCODE -ne 0) { throw "AAB 签名失败" }
        Write-Host "== AAB：$aabPath ==" -ForegroundColor Green
    }
} else {
    $apk = Join-Path $genAndroid "app\build\outputs\apk\$($spec.Flavor)\debug\app-$($spec.Flavor)-debug.apk"
}
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
