use std::path::PathBuf;

use farm_core::Game;
use parking_lot::Mutex;

pub struct AppState {
    pub game: Mutex<Game>,
    pub save_path: PathBuf,
    pub boot_warning: Option<String>,
    /// 调试时间加速累计的偏移；让 `today` 跟游戏内被拨快的时钟一致。
    pub time_offset_ms: Mutex<i64>,
}
