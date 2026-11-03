//! 浏览器壳的 Rust 侧：把 `farm-core` 包成「一个 JSON 进、一个 JSON 出」。
//!
//! 设计约束：
//! - **零 wasm-bindgen 依赖**：只用 C ABI 传指针与长度，前端手工读写线性内存。
//!   这样不引入额外工具链（wasm-pack / wasm-bindgen-cli），`cargo build` 一步到位。
//! - **时间仍由外部注入**：`now` 与 `tz` 都来自 JS，内核保持零系统时间。
//! - 存档序列化与迁移跟桌面壳走同一套（`SaveFile` + `migrate_save`），
//!   落盘换成 IndexedDB 由前端负责。

use std::cell::RefCell;

use farm_core::{
    economy_preview, generate_seeds, migrate_save, review_batch, Action, ContentPack, Game,
    SaveFile, CORE_VERSION, SAVE_SCHEMA,
};
use serde_json::{json, Value};

struct Session {
    game: Game,
    offset_ms: i64,
    /// `new Date().getTimezoneOffset()`：UTC 以西的分钟数（东八区为 -480）。
    tz_min_west: i32,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------- C ABI

#[no_mangle]
pub extern "C" fn farm_alloc(len: usize) -> *mut u8 {
    let boxed = vec![0u8; len].into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn farm_dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
}

/// 调用一个命令：入参是 UTF-8 JSON，返回 `[len: u32 LE][bytes]`，
/// 用完调 `farm_free_result` 释放。
#[no_mangle]
pub unsafe extern "C" fn farm_call(ptr: *const u8, len: usize) -> *mut u8 {
    let input: &[u8] = if ptr.is_null() || len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, len)
    };
    let text = handle(input);
    let bytes = text.into_bytes();
    let mut buf: Vec<u8> = Vec::with_capacity(bytes.len() + 4);
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(&bytes);
    Box::into_raw(buf.into_boxed_slice()) as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn farm_free_result(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let mut header = [0u8; 4];
    std::ptr::copy_nonoverlapping(ptr, header.as_mut_ptr(), 4);
    let total = u32::from_le_bytes(header) as usize + 4;
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, total)));
}

// ---------------------------------------------------------------- 命令分发

fn handle(input: &[u8]) -> String {
    match dispatch_request(input) {
        Ok(value) => value.to_string(),
        Err(message) => json!({"ok": false, "error": {"message": message}}).to_string(),
    }
}

fn dispatch_request(input: &[u8]) -> Result<Value, String> {
    let req: Value =
        serde_json::from_slice(input).map_err(|e| format!("请求 JSON 解析失败：{e}"))?;
    let cmd = req.get("cmd").and_then(Value::as_str).unwrap_or_default();
    match cmd {
        "ping" => Ok(json!({"ok": true, "pong": true})),
        "boot" => boot(&req),
        "snapshot" => {
            let now = now_of(&req);
            with_game(|s| Ok(json!({"ok": true, "snapshot": s.game.snapshot(now)})))
        }
        "content" => with_game(|s| Ok(json!({"ok": true, "content": s.game.content()}))),
        "dispatch" => dispatch_action(&req),
        "sync" => {
            let now = now_of(&req);
            with_game(|s| {
                let today = today_of(s, now);
                let outcome = s.game.sync(now, &today);
                Ok(json!({
                    "ok": true,
                    "snapshot": s.game.snapshot(now),
                    "effects": outcome.effects,
                    "changed": outcome.changed,
                }))
            })
        }
        "advance_time" => advance_time(&req),
        "save" => {
            let now = now_of(&req);
            with_game(|s| {
                let text = save_text(&s.game, now)?;
                Ok(json!({"ok": true, "save": text, "savedAt": now}))
            })
        }
        "import_save" => import_save(&req),
        "export_content" => with_game(|s| {
            let text = serde_json::to_string_pretty(s.game.content())
                .map_err(|e| format!("序列化内容包失败：{e}"))?;
            Ok(json!({"ok": true, "content": text}))
        }),
        "import_content" => import_content(&req),
        "apply_content" => apply_content(&req),
        "preview_content" => preview_content(&req),
        "generate_seeds" => {
            let count = req.get("count").and_then(Value::as_u64).unwrap_or(1) as usize;
            let seed = req.get("seed").and_then(Value::as_u64).unwrap_or(0);
            Ok(json!({"ok": true, "crops": generate_seeds(count, seed)}))
        }
        "review_seeds" => {
            let count = req.get("count").and_then(Value::as_u64).unwrap_or(1) as usize;
            let seed = req.get("seed").and_then(Value::as_u64).unwrap_or(0);
            Ok(json!({"ok": true, "reviews": review_batch(count, seed)}))
        }
        "preview_breeding" => {
            let now = now_of(&req);
            let a = req.get("a").and_then(Value::as_u64).unwrap_or(0) as u8;
            let b = req.get("b").and_then(Value::as_u64).unwrap_or(0) as u8;
            with_game(|s| {
                let preview = s
                    .game
                    .preview_ranch_breeding(a, b, now)
                    .map_err(|e| e.to_string())?;
                Ok(json!({"ok": true, "preview": preview}))
            })
        }
        "app_info" => Ok(json!({"ok": true, "info": app_info()})),
        other => Err(format!("未知命令：{other}")),
    }
}

fn boot(req: &Value) -> Result<Value, String> {
    let now = now_of(req);
    let tz = req.get("tz").and_then(Value::as_i64).unwrap_or(0) as i32;
    let save = req
        .get("save")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty());

    let (game, warning) = match save {
        Some(text) => match parse_save(text) {
            Ok(game) => (game, Value::Null),
            Err(err) => (
                Game::new(now),
                json!(format!("存档读不出来（{err}），已新建一份。")),
            ),
        },
        None => (Game::new(now), Value::Null),
    };

    let snapshot = game.snapshot(now);
    let content = game.content().clone();

    SESSION.with(|cell| {
        *cell.borrow_mut() = Some(Session {
            game,
            offset_ms: 0,
            tz_min_west: tz,
        });
    });

    Ok(json!({
        "ok": true,
        "snapshot": snapshot,
        "content": content,
        "info": app_info(),
        "warning": warning,
    }))
}

fn dispatch_action(req: &Value) -> Result<Value, String> {
    let now = now_of(req);
    let action: Action = serde_json::from_value(req.get("action").cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("动作不合法：{e}"))?;
    with_game(|s| {
        let outcome = s.game.dispatch(action, now).map_err(|e| e.to_string())?;
        Ok(json!({
            "ok": true,
            "snapshot": s.game.snapshot(now),
            "effects": outcome.effects,
        }))
    })
}

fn advance_time(req: &Value) -> Result<Value, String> {
    let now = now_of(req);
    let hours = req.get("hours").and_then(Value::as_u64).unwrap_or(0);
    if hours == 0 || hours > 24 * 30 {
        return Err("一次只能快进 1–720 小时".to_string());
    }
    with_game(|s| {
        let ms = (hours as i64) * 3_600_000;
        s.game.shift_time(ms);
        s.offset_ms = s.offset_ms.saturating_add(ms);
        let today = today_of(s, now);
        let outcome = s.game.sync(now, &today);
        Ok(json!({
            "ok": true,
            "snapshot": s.game.snapshot(now),
            "effects": outcome.effects,
        }))
    })
}

fn import_save(req: &Value) -> Result<Value, String> {
    let now = now_of(req);
    let text = req
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "缺少存档文本".to_string())?;
    let game = parse_save(text)?;
    with_game(|s| {
        s.game = game;
        s.offset_ms = 0;
        Ok(json!({
            "ok": true,
            "snapshot": s.game.snapshot(now),
            "content": s.game.content(),
        }))
    })
}

fn import_content(req: &Value) -> Result<Value, String> {
    let now = now_of(req);
    let text = req
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "缺少内容包文本".to_string())?;
    let pack = parse_content(text.as_bytes())?;
    with_game(|s| {
        s.game.apply_content(pack, now).map_err(|e| e.to_string())?;
        Ok(json!({
            "ok": true,
            "snapshot": s.game.snapshot(now),
            "content": s.game.content(),
        }))
    })
}

fn apply_content(req: &Value) -> Result<Value, String> {
    let now = now_of(req);
    let pack: ContentPack = serde_json::from_value(req.get("pack").cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("内容包不合法：{e}"))?;
    with_game(|s| {
        s.game.apply_content(pack, now).map_err(|e| e.to_string())?;
        Ok(json!({
            "ok": true,
            "snapshot": s.game.snapshot(now),
            "content": s.game.content(),
        }))
    })
}

fn preview_content(req: &Value) -> Result<Value, String> {
    let pack: ContentPack = serde_json::from_value(req.get("pack").cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("内容包不合法：{e}"))?;
    Ok(json!({
        "ok": true,
        "issues": pack.validate(),
        "crops": economy_preview(&pack),
    }))
}

// ---------------------------------------------------------------- 内部工具

fn with_game<T>(f: impl FnOnce(&mut Session) -> Result<T, String>) -> Result<T, String> {
    SESSION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let session = slot
            .as_mut()
            .ok_or_else(|| "还没启动：先调用 boot".to_string())?;
        f(session)
    })
}

fn now_of(req: &Value) -> i64 {
    req.get("now").and_then(Value::as_i64).unwrap_or(0)
}

/// 与桌面壳同构：JSON → 迁移 → `SaveFile` → `Game`。
fn parse_save(text: &str) -> Result<Game, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("JSON 解析失败：{e}"))?;
    let value = migrate_save(value).map_err(|e| e.to_string())?;
    let save: SaveFile =
        serde_json::from_value(value).map_err(|e| format!("存档字段不完整：{e}"))?;
    Game::from_save(save).map_err(|e| e.to_string())
}

fn save_text(game: &Game, now: i64) -> Result<String, String> {
    serde_json::to_string_pretty(&game.to_save(now)).map_err(|e| format!("序列化存档失败：{e}"))
}

fn parse_content(bytes: &[u8]) -> Result<ContentPack, String> {
    let pack: ContentPack =
        serde_json::from_slice(bytes).map_err(|e| format!("内容包 JSON 解析失败：{e}"))?;
    let issues = pack.validate();
    if !issues.is_empty() {
        return Err(format!("内容包不合法：{}", issues.join("；")));
    }
    Ok(pack)
}

fn app_info() -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "coreVersion": CORE_VERSION,
        "savePath": "浏览器存储（IndexedDB）",
        "schema": SAVE_SCHEMA,
        "bootWarning": Value::Null,
    })
}

/// 生效时间 = 真实时间 + 快进偏移；再按本地时区折算成 `YYYY-MM-DD`。
fn today_of(session: &Session, now: i64) -> String {
    local_date(
        now.saturating_add(session.offset_ms),
        session.tz_min_west,
    )
}

/// 纯整数日期换算（Howard Hinnant 的 civil_from_days），不引入日期库。
fn local_date(ms: i64, tz_min_west: i32) -> String {
    let local = ms - i64::from(tz_min_west) * 60_000;
    let days = local.div_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_matches_known_days() {
        // 2026-11-03 00:00:00 UTC+8 = 2026-11-02T16:00:00Z
        let ms = 1_793_635_200_000; // 2026-11-02T16:00:00Z
        assert_eq!(local_date(ms, -480), "2026-11-03");
        assert_eq!(local_date(ms, 0), "2026-11-02");
    }
}
