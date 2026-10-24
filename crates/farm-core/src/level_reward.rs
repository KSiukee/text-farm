use serde::Serialize;

use crate::content::ContentPack;
use crate::model::{GameState, MAX_LEVEL};

/// 等级奖励从 2 级开始；1 级是开局等级，没有「补发」。
pub const LEVEL_REWARD_MIN: u32 = 2;
/// 到达某个等级时，该等级新解锁的每种作物送几颗种子。
pub const NEW_CROP_SEEDS: u32 = 2;

/// 每级都有的一份金币，随等级缓慢上涨。
pub fn coins_for(level: u32) -> u64 {
    50 + u64::from(level) * 20
}

/// 关键等级额外给的物品（目前是牧场专用饲料）。
pub fn milestone_items(level: u32) -> &'static [(&'static str, u32)] {
    const FEED_ONE: &[(&str, u32)] = &[("ranch:feed", 1)];
    const FEED_THREE: &[(&str, u32)] = &[("ranch:feed", 3)];
    match level {
        5 | 10 | 15 | 20 | 25 => FEED_ONE,
        30 => FEED_THREE,
        _ => &[],
    }
}

/// 等级里程碑送的绑定家具（不卖，只送）。
pub fn milestone_furniture(level: u32) -> &'static [&'static str] {
    match level {
        10 => &["gift_quilt"],
        20 => &["gift_harvest_rug"],
        30 => &["gift_anniversary_lamp"],
        _ => &[],
    }
}

/// 只有文本是给界面看的；数字仍然只从 Rust 出。
pub fn reward_text(content: &ContentPack, level: u32) -> String {
    let mut parts = vec![format!("金币+{}", coins_for(level))];
    for crop in content
        .crops
        .iter()
        .filter(|crop| crop.unlock_level == level)
    {
        parts.push(format!("{}种子×{}", crop.name, NEW_CROP_SEEDS));
    }
    for (key, qty) in milestone_items(level) {
        let name = crate::ranch::item_def(key)
            .map(|def| def.name)
            .unwrap_or(key);
        parts.push(format!("{name}×{qty}"));
    }
    for id in milestone_furniture(level) {
        if let Some(def) = crate::room::furniture_def(id) {
            parts.push(format!("{}×1", def.name));
        }
    }
    parts.join(" · ")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelRewardView {
    pub level: u32,
    pub coins: u64,
    pub text: String,
    pub claimed: bool,
    pub claimable: bool,
}

pub fn views(state: &GameState, content: &ContentPack) -> Vec<LevelRewardView> {
    (LEVEL_REWARD_MIN..=MAX_LEVEL)
        .map(|level| {
            let claimed = state.level_rewards_claimed.contains(&level);
            LevelRewardView {
                level,
                coins: coins_for(level),
                text: reward_text(content, level),
                claimed,
                claimable: !claimed && state.level >= level,
            }
        })
        .collect()
}

pub fn claimable_count(state: &GameState) -> u32 {
    (LEVEL_REWARD_MIN..=MAX_LEVEL)
        .filter(|level| !state.level_rewards_claimed.contains(level) && state.level >= *level)
        .count() as u32
}
