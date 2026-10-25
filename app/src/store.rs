use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use farm_core::{migrate_save, ContentPack, Game, SaveFile};

fn bak_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn backups_dir(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
}

pub fn read_game(path: &Path) -> Result<Game, String> {
    let bytes = fs::read(path).map_err(|e| format!("读不了 {}：{e}", path.display()))?;
    parse_game(&bytes)
}

/// 从字节解析存档。Android 上文件是 `content://` URI，由前端用 fs 插件读成文本后走这里。
pub fn parse_game(bytes: &[u8]) -> Result<Game, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("JSON 解析失败：{e}"))?;
    let value = migrate_save(value).map_err(|e| e.to_string())?;
    let save: SaveFile =
        serde_json::from_value(value).map_err(|e| format!("存档字段不完整：{e}"))?;
    Game::from_save(save).map_err(|e| e.to_string())
}

fn rotate_backup(path: &Path) {
    if !path.exists() {
        return;
    }
    let dir = backups_dir(path);
    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&dir) {
        let newest = entries
            .flatten()
            .filter_map(|entry| entry.metadata().ok())
            .filter_map(|meta| meta.modified().ok())
            .max();
        if let Some(newest) = newest {
            if SystemTime::now().duration_since(newest).unwrap_or_default()
                < Duration::from_secs(300)
            {
                return;
            }
        }
    }

    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let _ = fs::copy(path, dir.join(format!("save-{stamp}.json")));
    prune_backups(&dir, 10);
}

fn prune_backups(dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("save-"))
        })
        .collect();
    files.sort();
    while files.len() > keep {
        let oldest = files.remove(0);
        let _ = fs::remove_file(oldest);
    }
}

/// 主档 → .bak → 轮转备份 → 新档。返回的警告会显示在界面上，绝不静默吞掉。
pub fn load_or_new(path: &Path, now: i64) -> (Game, Option<String>) {
    let mut warnings = Vec::new();

    if path.exists() {
        match read_game(path) {
            Ok(game) => return (game, None),
            Err(e) => warnings.push(format!("主存档不可用（{e}）")),
        }
    }

    let bak = bak_path(path);
    if bak.exists() {
        match read_game(&bak) {
            Ok(game) => {
                let warning = if warnings.is_empty() {
                    Some("主存档不可用，已从 .bak 恢复。".to_string())
                } else {
                    Some(format!("{}；已从 .bak 恢复。", warnings.join("；")))
                };
                return (game, warning);
            }
            Err(e) => warnings.push(format!("备份也不可用（{e}）")),
        }
    }

    let dir = backups_dir(path);
    if let Ok(entries) = fs::read_dir(&dir) {
        let mut backups: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("save-"))
            })
            .collect();
        backups.sort();
        backups.reverse();
        for backup in backups {
            match read_game(&backup) {
                Ok(game) => {
                    let name = backup
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("backups");
                    let warning = format!("{}；已从最近备份恢复（{name}）。", warnings.join("；"));
                    return (game, Some(warning));
                }
                Err(e) => warnings.push(format!("{} 不可用（{e}）", backup.display())),
            }
        }
    }

    let warning = if warnings.is_empty() {
        None
    } else {
        Some(format!("{}；已新建存档。", warnings.join("；")))
    };
    (Game::new(now), warning)
}

pub fn save_atomic(path: &Path, game: &Game, now: i64) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("存档路径没有父目录：{}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("建存档目录失败（{}）：{e}", parent.display()))?;

    rotate_backup(path);

    let json = serde_json::to_vec_pretty(&game.to_save(now))
        .map_err(|e| format!("序列化存档失败：{e}"))?;

    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)
            .map_err(|e| format!("建临时存档失败（{}）：{e}", tmp.display()))?;
        file.write_all(&json)
            .map_err(|e| format!("写临时存档失败：{e}"))?;
        file.sync_all().map_err(|e| format!("刷盘失败：{e}"))?;
    }

    if path.exists() {
        let _ = fs::copy(path, bak_path(path));
    }

    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("替换存档失败（{}）：{e}", path.display())
    })
}

/// 存档序列化成 JSON 文本（Android 上由前端写进 content URI）。
pub fn export_save_text(game: &Game, now: i64) -> Result<String, String> {
    let json = serde_json::to_vec_pretty(&game.to_save(now))
        .map_err(|e| format!("序列化存档失败：{e}"))?;
    String::from_utf8(json).map_err(|e| format!("存档不是合法 UTF-8：{e}"))
}

pub fn export_save_to(path: &Path, game: &Game, now: i64) -> Result<(), String> {
    let json = export_save_text(game, now)?;
    fs::write(path, json).map_err(|e| format!("写导出文件失败（{}）：{e}", path.display()))
}

/// 内容包序列化成 JSON 文本（同上）。
pub fn export_content_text(game: &Game) -> Result<String, String> {
    serde_json::to_string_pretty(game.content()).map_err(|e| format!("序列化内容包失败：{e}"))
}

pub fn export_content_to(path: &Path, game: &Game) -> Result<(), String> {
    let json = export_content_text(game)?;
    fs::write(path, json).map_err(|e| format!("写内容包失败（{}）：{e}", path.display()))
}

pub fn import_content_from(path: &Path) -> Result<ContentPack, String> {
    let bytes = fs::read(path).map_err(|e| format!("读不了 {}：{e}", path.display()))?;
    parse_content(&bytes)
}

pub fn parse_content(bytes: &[u8]) -> Result<ContentPack, String> {
    let pack: ContentPack =
        serde_json::from_slice(bytes).map_err(|e| format!("内容包 JSON 解析失败：{e}"))?;
    let issues = pack.validate();
    if !issues.is_empty() {
        return Err(format!("内容包不合法：{}", issues.join("；")));
    }
    Ok(pack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use farm_core::{ContentPack, Game};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("text-farm-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_load_backup_and_export_roundtrip() {
        let dir = temp_dir("store");
        let save_path = dir.join("save.json");
        let game = Game::new(1_000);

        save_atomic(&save_path, &game, 1_000).unwrap();
        assert!(save_path.exists());

        save_atomic(&save_path, &game, 2_000).unwrap();
        assert!(bak_path(&save_path).exists());
        assert!(backups_dir(&save_path).exists());

        let loaded = read_game(&save_path).unwrap();
        assert_eq!(loaded.snapshot(0).level, 1);

        fs::write(&save_path, b"{ not json").unwrap();
        let (recovered, warning) = load_or_new(&save_path, 3_000);
        assert_eq!(recovered.snapshot(0).level, 1);
        assert!(warning.unwrap().contains("bak"));

        let export_path = dir.join("export.json");
        export_save_to(&export_path, &game, 4_000).unwrap();
        assert_eq!(read_game(&export_path).unwrap().snapshot(0).level, 1);

        let content_path = dir.join("content.json");
        export_content_to(&content_path, &game).unwrap();
        let pack: ContentPack = import_content_from(&content_path).unwrap();
        assert_eq!(pack.crops.len(), 13);

        let _ = fs::remove_dir_all(dir);
    }
}
