use std::path::PathBuf;

use tauri::Manager;

pub fn save_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = if let Some(dir) = std::env::var_os("TEXT_FARM_DATA_DIR") {
        PathBuf::from(dir)
    } else {
        app.path()
            .app_data_dir()
            .map_err(|e| format!("拿不到应用数据目录：{e}"))?
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("建应用数据目录失败（{}）：{e}", dir.display()))?;
    Ok(dir.join("save.json"))
}
