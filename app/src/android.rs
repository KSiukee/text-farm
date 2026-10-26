//! Android 专属壳能力：钓鱼小游戏的「全屏横屏」。
//!
//! 桌面是空实现——前端无论在哪个平台都调同一个命令，不会报错。
//!
//! 为什么走 JNI 而不是 Kotlin：`gen/android` 是 `tauri android init` 的生成物，
//! 手改会被覆盖；Tauri 的 `Webview::jni_handle()` 能直接拿到当前 Activity，
//! 在 Rust 里调 `setRequestedOrientation` / `WindowInsetsController` 就够了。

#[cfg(target_os = "android")]
mod imp {
    use jni::objects::{JObject, JValue};

    /// ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE
    const LANDSCAPE: i32 = 0;
    /// ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
    const PORTRAIT: i32 = 1;
    /// WindowInsets.Type.systemBars() = statusBars(1) | navigationBars(2) | captionBar(4)
    const SYSTEM_BARS: i32 = 7;
    /// WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    const BEHAVIOR_TRANSIENT_BY_SWIPE: i32 = 1;

    pub fn set_game_mode(webview: &tauri::Webview, enable: bool) {
        // JniHandle 挂在 PlatformWebview 上（Webview::with_webview 回调里拿）
        let _ = webview.with_webview(move |platform| {
            platform.jni_handle().exec(move |env, activity, _webview| {
                // 1) 锁方向：横屏进游戏，退出回竖屏
                let orientation = if enable { LANDSCAPE } else { PORTRAIT };
                let _ = env.call_method(
                    activity,
                    "setRequestedOrientation",
                    "(I)V",
                    &[JValue::Int(orientation)],
                );

                // 2) 沉浸式：把状态栏/导航栏收起来（API 30+ 的 WindowInsetsController）
                let window = match env
                    .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
                    .and_then(|v| v.l())
                {
                    Ok(v) => v,
                    Err(_) => return,
                };
                if window.is_null() {
                    return;
                }
                let window = unsafe { JObject::from_raw(window.as_raw()) };

                let controller = match env
                    .call_method(
                        &window,
                        "getInsetsController",
                        "()Landroid/view/WindowInsetsController;",
                        &[],
                    )
                    .and_then(|v| v.l())
                {
                    Ok(v) => v,
                    Err(_) => return,
                };
                if controller.is_null() {
                    return;
                }
                let controller = unsafe { JObject::from_raw(controller.as_raw()) };

                if enable {
                    // 只 hide，不设 BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE：
                    // 那个 behavior 会触发系统的「Viewing full screen」教育弹窗（每次进来都弹一次）
                    let _ = env.call_method(
                        &controller,
                        "hide",
                        "(I)V",
                        &[JValue::Int(SYSTEM_BARS)],
                    );
                } else {
                    let _ = env.call_method(
                        &controller,
                        "show",
                        "(I)V",
                        &[JValue::Int(SYSTEM_BARS)],
                    );
                }
            });
        });
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    pub fn set_game_mode(_webview: &tauri::Webview, _enable: bool) {}
}

/// 进入 / 退出「钓鱼全屏横屏」。
#[tauri::command]
pub fn game_mode(enable: bool, webview: tauri::Webview) {
    imp::set_game_mode(&webview, enable);
}
