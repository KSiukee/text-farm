//! 玩家自定义资源：头像 / 背景 / 字体统一存到 `custom/` 下，选中的头像记在 profile.json。
//! 不把大文件塞进存档；前端只拿到 data URL。

use std::fs;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::paths;

const MAX_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlayerAssets {
    /// `preset:🌻` 或 `file:avatar.png`。
    pub avatar: String,
}

fn kind_dir(app: &tauri::AppHandle, kind: &str) -> Result<PathBuf, String> {
    let folder = match kind {
        "avatars" => "avatars",
        "backgrounds" => "backgrounds",
        "fonts" => "fonts",
        _ => return Err("未知的自定义资源类型".into()),
    };
    let dir = paths::custom_dir(app)?.join(folder);
    fs::create_dir_all(&dir).map_err(|e| format!("建资源目录失败：{e}"))?;
    Ok(dir)
}

fn safe_name(name: &str) -> Result<&str, String> {
    if name.is_empty()
        || name.len() > 80
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.starts_with('.')
    {
        return Err("资源文件名不合法".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err("资源文件名只能使用英文、数字、点、横线、下划线".into());
    }
    Ok(name)
}

fn mime_for(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let Some((meta, payload)) = data_url.split_once(',') else {
        return Err("资源内容不是 data URL".into());
    };
    if !meta.contains(";base64") {
        return Err("资源内容不是 base64 data URL".into());
    }
    STANDARD
        .decode(payload)
        .map_err(|e| format!("资源 base64 解码失败：{e}"))
}

fn encode_data_url(name: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime_for(name),
        STANDARD.encode(bytes)
    )
}

fn profile_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(paths::custom_dir(app)?.join("profile.json"))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("写资源失败：{e}"))?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("替换资源失败：{e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("保存资源失败：{e}"))
}

#[tauri::command]
pub fn custom_assets_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = paths::custom_dir(&app)?;
    for folder in ["avatars", "backgrounds", "fonts"] {
        fs::create_dir_all(dir.join(folder)).map_err(|e| format!("建资源目录失败：{e}"))?;
    }
    Ok(dir.display().to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_custom_asset(
    app: tauri::AppHandle,
    kind: String,
    name: String,
    data_url: String,
) -> Result<String, String> {
    let name = safe_name(&name)?;
    let bytes = decode_data_url(&data_url)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("这个资源超过 32MB，先压缩一下".into());
    }
    let path = kind_dir(&app, &kind)?.join(name);
    write_atomic(&path, &bytes)?;
    Ok(name.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn read_custom_asset(
    app: tauri::AppHandle,
    kind: String,
    name: String,
) -> Result<Option<String>, String> {
    let name = safe_name(&name)?;
    let path = kind_dir(&app, &kind)?.join(name);
    if !path.exists() {
        return Ok(None);
    }
    let meta = fs::metadata(&path).map_err(|e| format!("读资源信息失败：{e}"))?;
    if meta.len() > MAX_BYTES {
        return Err("资源文件过大，已跳过".into());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读资源失败：{e}"))?;
    Ok(Some(encode_data_url(name, &bytes)))
}

#[tauri::command(rename_all = "camelCase")]
pub fn delete_custom_asset(
    app: tauri::AppHandle,
    kind: String,
    name: String,
) -> Result<(), String> {
    let name = safe_name(&name)?;
    let path = kind_dir(&app, &kind)?.join(name);
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("删除资源失败：{e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn load_player_assets(app: tauri::AppHandle) -> Result<PlayerAssets, String> {
    let path = profile_path(&app)?;
    if !path.exists() {
        return Ok(PlayerAssets::default());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("读玩家资源失败：{e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("玩家资源格式不对：{e}"))
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_player_assets(app: tauri::AppHandle, assets: PlayerAssets) -> Result<(), String> {
    let text = serde_json::to_vec_pretty(&assets).map_err(|e| format!("序列化玩家资源失败：{e}"))?;
    write_atomic(&profile_path(&app)?, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_names_and_data_urls() {
        assert!(safe_name("avatar.png").is_ok());
        assert!(safe_name("../avatar.png").is_err());
        assert!(safe_name("背景.png").is_err());
        let bytes = decode_data_url("data:image/png;base64,aGVsbG8=").unwrap();
        assert_eq!(bytes, b"hello");
        assert_eq!(
            encode_data_url("font.woff2", b"hi"),
            "data:font/woff2;base64,aGk="
        );
    }
}
