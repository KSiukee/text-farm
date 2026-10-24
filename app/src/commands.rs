use std::path::Path;

use farm_core::{
    economy_preview, generate_seeds, review_batch, Action, ContentPack, CropDef, EconomyPreview,
    Effect, GameError, RanchBreedPreview, SeedReview, Snapshot, CORE_VERSION, SAVE_SCHEMA,
};
use serde::Serialize;
use tauri::State;

use crate::error::ApiError;
use crate::now_ms;
use crate::state::AppState;
use crate::store;

fn game_error(error: GameError) -> ApiError {
    ApiError::new(error.code(), error.to_string())
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    let game = state.game.lock();
    game.snapshot(now_ms())
}

#[tauri::command]
pub fn get_content(state: State<'_, AppState>) -> ContentPack {
    state.game.lock().content().clone()
}

/// 繁殖预览（只读）：与真繁殖共用 breedSeq，看到什么就生什么。
#[tauri::command]
pub fn preview_ranch_breeding(
    a: u8,
    b: u8,
    state: State<'_, AppState>,
) -> Result<RanchBreedPreview, ApiError> {
    let game = state.game.lock();
    game.preview_ranch_breeding(a, b, now_ms())
        .map_err(game_error)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchReply {
    pub snapshot: Snapshot,
    pub effects: Vec<Effect>,
    pub save_error: Option<String>,
}

#[tauri::command]
pub fn dispatch(action: Action, state: State<'_, AppState>) -> Result<DispatchReply, ApiError> {
    let mut game = state.game.lock();
    let now = now_ms();
    let outcome = game.dispatch(action, now).map_err(game_error)?;
    let save_error = store::save_atomic(&state.save_path, &game, now).err();
    let snapshot = game.snapshot(now);
    Ok(DispatchReply {
        snapshot,
        effects: outcome.effects,
        save_error,
    })
}

#[tauri::command]
pub fn sync(state: State<'_, AppState>) -> DispatchReply {
    let mut game = state.game.lock();
    let now = now_ms();
    let offset = *state.time_offset_ms.lock();
    let outcome = game.sync(now, &crate::today_string_offset(offset));
    let save_error = if outcome.changed {
        store::save_atomic(&state.save_path, &game, now).err()
    } else {
        None
    };
    DispatchReply {
        snapshot: game.snapshot(now),
        effects: outcome.effects,
        save_error,
    }
}

#[tauri::command]
pub fn advance_time(hours: u32, state: State<'_, AppState>) -> Result<DispatchReply, ApiError> {
    if hours == 0 || hours > 24 * 30 {
        return Err(ApiError::new("invalid_hours", "一次只能快进 1–720 小时"));
    }
    let mut game = state.game.lock();
    let now = now_ms();
    let ms = i64::from(hours) * 3_600_000;
    game.shift_time(ms);
    let offset = {
        let mut offset = state.time_offset_ms.lock();
        *offset = offset.saturating_add(ms);
        *offset
    };
    let outcome = game.sync(now, &crate::today_string_offset(offset));
    let save_error = store::save_atomic(&state.save_path, &game, now).err();
    Ok(DispatchReply {
        snapshot: game.snapshot(now),
        effects: outcome.effects,
        save_error,
    })
}

#[tauri::command]
pub fn generate_content_seeds(count: usize, seed: u64) -> Vec<CropDef> {
    generate_seeds(count, seed)
}

#[tauri::command]
pub fn review_content_seeds(count: usize, seed: u64) -> Vec<SeedReview> {
    review_batch(count, seed)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPreview {
    pub issues: Vec<String>,
    pub crops: Vec<EconomyPreview>,
}

#[tauri::command]
pub fn preview_content(pack: ContentPack) -> ContentPreview {
    ContentPreview {
        issues: pack.validate(),
        crops: economy_preview(&pack),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReply {
    pub snapshot: Snapshot,
    pub content: ContentPack,
    pub warning: Option<String>,
}

#[tauri::command]
pub fn export_save(path: String, state: State<'_, AppState>) -> Result<(), ApiError> {
    let game = state.game.lock();
    store::export_save_to(Path::new(&path), &game, now_ms())
        .map_err(|message| ApiError::new("export_failed", message))
}

#[tauri::command]
pub fn import_save(path: String, state: State<'_, AppState>) -> Result<ImportReply, ApiError> {
    let imported = store::read_game(Path::new(&path))
        .map_err(|message| ApiError::new("import_failed", message))?;
    let now = now_ms();
    let mut game = state.game.lock();
    *game = imported;
    store::save_atomic(&state.save_path, &game, now)
        .map_err(|message| ApiError::new("save_failed", message))?;
    let snapshot = game.snapshot(now);
    let content = game.content().clone();
    Ok(ImportReply {
        snapshot,
        content,
        warning: None,
    })
}

#[tauri::command]
pub fn export_content(path: String, state: State<'_, AppState>) -> Result<(), ApiError> {
    let game = state.game.lock();
    store::export_content_to(Path::new(&path), &game)
        .map_err(|message| ApiError::new("export_failed", message))
}

#[tauri::command]
pub fn import_content(path: String, state: State<'_, AppState>) -> Result<ImportReply, ApiError> {
    let pack = store::import_content_from(Path::new(&path))
        .map_err(|message| ApiError::new("import_failed", message))?;
    let now = now_ms();
    let mut game = state.game.lock();
    game.apply_content(pack, now).map_err(game_error)?;
    store::save_atomic(&state.save_path, &game, now)
        .map_err(|message| ApiError::new("save_failed", message))?;
    let snapshot = game.snapshot(now);
    let content = game.content().clone();
    Ok(ImportReply {
        snapshot,
        content,
        warning: None,
    })
}

#[tauri::command]
pub fn apply_content(
    pack: ContentPack,
    state: State<'_, AppState>,
) -> Result<ImportReply, ApiError> {
    let now = now_ms();
    let mut game = state.game.lock();
    game.apply_content(pack, now).map_err(game_error)?;
    store::save_atomic(&state.save_path, &game, now)
        .map_err(|message| ApiError::new("save_failed", message))?;
    let snapshot = game.snapshot(now);
    let content = game.content().clone();
    Ok(ImportReply {
        snapshot,
        content,
        warning: None,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub core_version: String,
    pub save_path: String,
    pub schema: u32,
    pub boot_warning: Option<String>,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        core_version: CORE_VERSION.to_string(),
        save_path: state.save_path.display().to_string(),
        schema: SAVE_SCHEMA,
        boot_warning: state.boot_warning.clone(),
    }
}

#[tauri::command]
pub fn save_now(state: State<'_, AppState>) -> Result<(), ApiError> {
    let game = state.game.lock();
    store::save_atomic(&state.save_path, &game, now_ms())
        .map_err(|message| ApiError::new("save_failed", message))
}
