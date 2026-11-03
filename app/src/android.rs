//! Android 专属壳能力：钓鱼小游戏的「沉浸式」。
//!
//! **只管隐藏/恢复系统栏，不动屏幕方向。**
//! 早先这里连着 `setRequestedOrientation` 一起做（点抛竿强制横屏），
//! 但那个调用会绕过系统旋转锁：玩家明明锁了竖屏，还是被扳过去、钓完又扳回来，
//! 交互很暴力。现在方向交还给玩家——想横就自己转，CSS 的 landscape 布局会接管。
//!
//! 桌面是空实现：前端无论在哪个平台都调同一个命令，不会报错。

#[cfg(target_os = "android")]
mod imp {
    use jni::objects::{JObject, JValue};

    /// WindowInsets.Type.systemBars() = statusBars(1) | navigationBars(2) | captionBar(4)
    const SYSTEM_BARS: i32 = 7;

    pub fn set_immersive(webview: &tauri::Webview, enable: bool) {
        // JniHandle 挂在 PlatformWebview 上（Webview::with_webview 回调里拿）
        let _ = webview.with_webview(move |platform| {
            platform.jni_handle().exec(move |env, activity, _webview| {
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

                // API 30+ 的 WindowInsetsController；老机器上拿不到就安静退出
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

                // 只 hide/show，不设 BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE：
                // 那个 behavior 会触发系统的「Viewing full screen」教育弹窗（每次进来都弹一次）
                let method = if enable { "hide" } else { "show" };
                let _ = env.call_method(
                    &controller,
                    method,
                    "(I)V",
                    &[JValue::Int(SYSTEM_BARS)],
                );
            });
        });
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    pub fn set_immersive(_webview: &tauri::Webview, _enable: bool) {}
}

/// 进入 / 退出「钓鱼沉浸式」：藏系统栏，**不改方向**。
#[tauri::command]
pub fn set_immersive(enable: bool, webview: tauri::Webview) {
    imp::set_immersive(&webview, enable);
}
