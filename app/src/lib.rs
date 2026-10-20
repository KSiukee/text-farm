mod commands;
mod error;
mod paths;
mod state;
mod store;

use tauri::Manager;

pub(crate) fn now_ms() -> i64 {
    chrono::Local::now().timestamp_millis()
}

pub(crate) fn today_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub(crate) fn today_string_offset(offset_ms: i64) -> String {
    (chrono::Local::now() + chrono::Duration::milliseconds(offset_ms))
        .format("%Y-%m-%d")
        .to_string()
}

/// 前端把主题/背景贴好之后调用；窗口先隐藏，避免启动时先闪一下白/奶油底。
#[tauri::command]
fn ui_ready(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        let _ = window.set_focus();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let save_path = paths::save_path(app.handle()).map_err(std::io::Error::other)?;
            let now = now_ms();
            let (mut game, mut boot_warning) = store::load_or_new(&save_path, now);
            game.ensure_npcs(now);
            game.ensure_greenhouse();
            game.ensure_ranch();
            game.ensure_workshop();
            game.ensure_fishing();
            game.ensure_pets();
            game.sync(now, &today_string());

            if let Err(e) = store::save_atomic(&save_path, &game, now) {
                boot_warning = Some(match boot_warning {
                    Some(w) => format!("{w}；初始保存失败：{e}"),
                    None => format!("初始保存失败：{e}"),
                });
            }

            app.manage(state::AppState {
                game: parking_lot::Mutex::new(game),
                save_path,
                boot_warning,
                time_offset_ms: parking_lot::Mutex::new(0),
            });

            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&format!("文字农场 v{}", env!("CARGO_PKG_VERSION")))?;
                // 先不 show：等前端 ui_ready（主题/背景已贴好）再显示。
                // 2.5 秒兜底，前端挂了也不会永远看不见窗口。
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(2500));
                    let target = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        if let Some(window) = target.get_webview_window("main") {
                            let _ = window.show();
                        }
                    });
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                let state = window.state::<state::AppState>();
                let game = state.game.lock();
                let _ = store::save_atomic(&state.save_path, &game, now_ms());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_content,
            commands::dispatch,
            commands::sync,
            commands::advance_time,
            commands::preview_content,
            commands::generate_content_seeds,
            commands::review_content_seeds,
            commands::export_save,
            commands::import_save,
            commands::export_content,
            commands::import_content,
            commands::apply_content,
            commands::app_info,
            commands::save_now,
            ui_ready
        ])
        .run(tauri::generate_context!())
        .expect("文字农场启动失败");
}
