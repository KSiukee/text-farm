use std::path::PathBuf;

use tauri::Manager;

pub fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = if let Some(dir) = std::env::var_os("TEXT_FARM_DATA_DIR") {
        PathBuf::from(dir)
    } else {
        app.path()
            .app_data_dir()
            .map_err(|e| format!("拿不到应用数据目录：{e}"))?
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("建应用数据目录失败（{}）：{e}", dir.display()))?;
    Ok(dir)
}

pub fn save_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("save.json"))
}

/// 玩家自定义资源：头像 / 背景 / 字体 / profile.json 都放在这里。
pub fn custom_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("custom");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("建自定义资源目录失败（{}）：{e}", dir.display()))?;
    Ok(dir)
}
