//! 农场订单：每天 2–3 条村庄订单，交付作物换金币/经验/NPC 友好度。
//!
//! 只做内核；面板 UI、日常任务和模拟在后续任务接。

use serde::{Deserialize, Serialize};

use crate::content::ContentPack;
use crate::workshop::{craft_item, EquipmentRecipe};

pub const FARM_ORDER_OFFERS: usize = 3;
pub const FARM_ORDER_MAX_ACTIVE: usize = 2;

const NPC_PROFILES: [(&str, &str); 5] = [
    ("ahe", "阿禾"),
    ("xiaoman", "小满"),
    ("qingxing", "青杏"),
    ("laotao", "老陶"),
    ("liuyue", "六月"),
];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FarmOrder {
    pub id: String,
    pub npc_id: String,
    pub npc_name: String,
    pub crop_id: String,
    pub crop_name: String,
    pub qty: u32,
    /// 最后可交付日期（含当天），YYYY-MM-DD。
    pub expires_day: String,
    pub reward_coins: u64,
    pub reward_exp: u64,
    /// 实际交付的物品键；旧档作物订单为空，回退到 `crop:{crop_id}`。
    #[serde(default)]
    pub item_key: String,
    /// 实际交付的物品名；旧档为空，回退到 `crop_name`。
    #[serde(default)]
    pub item_name: String,
    /// 加工订单的配方；None 表示普通作物订单。
    #[serde(default)]
    pub recipe: Option<EquipmentRecipe>,
}

impl FarmOrder {
    /// 实际交付键；旧档没有 `item_key` 时回退到作物。
    pub fn delivery_key(&self) -> String {
        if self.item_key.is_empty() {
            format!("crop:{}", self.crop_id)
        } else {
            self.item_key.clone()
        }
    }

    pub fn item_name_or_crop(&self) -> &str {
        if self.item_name.is_empty() {
            &self.crop_name
        } else {
            &self.item_name
        }
    }

    pub fn is_craft(&self) -> bool {
        self.recipe.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FarmOrderState {
    pub day: String,
    pub offers: Vec<FarmOrder>,
    pub active: Vec<FarmOrder>,
}

impl FarmOrderState {
    /// 跨天刷新报价；返回当天过期的已接订单，交给引擎扣 NPC 友好度。
    pub fn ensure_day(
        &mut self,
        today: &str,
        level: u32,
        workshop_built: bool,
        content: &ContentPack,
    ) -> Vec<FarmOrder> {
        if self.day == today {
            return Vec::new();
        }
        let expired: Vec<FarmOrder> = self
            .active
            .iter()
            .filter(|order| order.expires_day.as_str() < today)
            .cloned()
            .collect();
        self.active
            .retain(|order| order.expires_day.as_str() >= today);
        self.offers = generate_offers(today, level, workshop_built, content);
        self.day = today.to_string();
        expired
    }

    pub fn accept(&mut self, id: &str) -> bool {
        if self.active.len() >= FARM_ORDER_MAX_ACTIVE {
            return false;
        }
        let Some(index) = self.offers.iter().position(|order| order.id == id) else {
            return false;
        };
        self.active.push(self.offers.remove(index));
        true
    }

    pub fn find_offer(&self, id: &str) -> Option<&FarmOrder> {
        self.offers.iter().find(|order| order.id == id)
    }

    pub fn find_active(&self, id: &str) -> Option<&FarmOrder> {
        self.active.iter().find(|order| order.id == id)
    }

    pub fn remove_active(&mut self, id: &str) -> Option<FarmOrder> {
        let index = self.active.iter().position(|order| order.id == id)?;
        Some(self.active.remove(index))
    }
}

fn generate_offers(
    today: &str,
    level: u32,
    workshop_built: bool,
    content: &ContentPack,
) -> Vec<FarmOrder> {
    let crops: Vec<_> = content
        .crops
        .iter()
        .filter(|crop| crop.unlock_level <= level && crop.sell_price > 0)
        .collect();
    if crops.is_empty() {
        return Vec::new();
    }
    let mut offers: Vec<FarmOrder> = (0..FARM_ORDER_OFFERS.saturating_sub(1))
        .map(|index| crop_offer(today, level, index, &crops))
        .collect();
    if workshop_built {
        if let Some(craft) = craft_offer(today, level) {
            offers.push(craft);
        }
    }
    while offers.len() < FARM_ORDER_OFFERS {
        let index = offers.len();
        offers.push(crop_offer(today, level, index, &crops));
    }
    offers
}

fn crop_offer(
    today: &str,
    level: u32,
    index: usize,
    crops: &[&crate::content::CropDef],
) -> FarmOrder {
    let roll = roll(today, 0x200 + index as u64 * 31);
    let crop = crops[(roll as usize) % crops.len()];
    let (npc_id, npc_name) = NPC_PROFILES[((roll >> 8) as usize) % NPC_PROFILES.len()];
    // 便宜作物量大、贵作物量小；上限按单季产量×2 块地，2–3 天交付。
    let budget = 900 + u64::from(level) * 55;
    let max_qty = (u64::from(crop.yield_per_season.max(1)) * 2).clamp(3, 24);
    let base = (budget / crop.sell_price.max(1)).clamp(2, max_qty);
    let qty = (base + ((roll >> 16) % 4) as u64).min(max_qty) as u32;
    let premium = 135 + ((roll >> 24) % 21) as u64;
    let days = 3 + ((roll >> 32) % 2) as u32;
    FarmOrder {
        id: format!("{today}-{index}"),
        npc_id: npc_id.into(),
        npc_name: npc_name.into(),
        crop_id: crop.id.clone(),
        crop_name: crop.name.clone(),
        qty,
        expires_day: add_days(today, days),
        reward_coins: crop
            .sell_price
            .saturating_mul(u64::from(qty))
            .saturating_mul(premium)
            / 100,
        reward_exp: 10 + u64::from(qty) * 2,
        item_key: format!("crop:{}", crop.id),
        item_name: crop.name.clone(),
        recipe: None,
    }
}

/// 只挑「原料全是直接作物/牧场产物」的配方，保证一键送去做能真的开起来，
/// 不需要玩家先手搓半成品。草莓酱这类多级配方以后再做自动排程。
fn craft_offer(today: &str, level: u32) -> Option<FarmOrder> {
    let recipes: Vec<EquipmentRecipe> = EquipmentRecipe::ALL
        .into_iter()
        .filter(|recipe| {
            craft_item(recipe.output_key()).is_some()
                && recipe
                    .inputs()
                    .iter()
                    .all(|(key, _)| key.starts_with("crop:") || key.starts_with("ranch:"))
        })
        .collect();
    if recipes.is_empty() {
        return None;
    }
    let roll = roll(today, 0x900);
    let recipe = recipes[(roll as usize) % recipes.len()];
    let (npc_id, npc_name) = NPC_PROFILES[((roll >> 8) as usize) % NPC_PROFILES.len()];
    let price = craft_item(recipe.output_key())?.price;
    let qty = 1;
    let days = 3 + ((roll >> 24) % 2) as u32;
    Some(FarmOrder {
        id: format!("{today}-craft"),
        npc_id: npc_id.into(),
        npc_name: npc_name.into(),
        crop_id: String::new(),
        crop_name: recipe.label().to_string(),
        qty,
        expires_day: add_days(today, days),
        reward_coins: price.saturating_mul(3) / 2 + 100 + u64::from(level) * 5,
        reward_exp: 20,
        item_key: recipe.output_key().to_string(),
        item_name: recipe.label().to_string(),
        recipe: Some(recipe),
    })
}

fn roll(day: &str, salt: u64) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ salt;
    for byte in day.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^ (hash >> 33)
}

/// 返回 `day` 所在周的周一（YYYY-MM-DD）；坏日期原样返回。
/// 1970-01-01 是周四，所以 `(z + 3) % 7` 正好是“周一 = 0”。
pub fn week_start(day: &str) -> String {
    let Some((year, month, date)) = parse_day(day) else {
        return day.to_string();
    };
    let z = days_from_civil(year, month, date);
    let offset = (z + 3).rem_euclid(7);
    let (year, month, date) = civil_from_days(z - offset);
    format!("{year:04}-{month:02}-{date:02}")
}

/// 给 YYYY-MM-DD 加天数；解析失败时原样返回，避免坏日期把订单打崩。
pub fn add_days(day: &str, days: u32) -> String {
    let Some((year, month, date)) = parse_day(day) else {
        return day.to_string();
    };
    let z = days_from_civil(year, month, date) + i64::from(days);
    let (year, month, date) = civil_from_days(z);
    format!("{year:04}-{month:02}-{date:02}")
}

fn parse_day(day: &str) -> Option<(i64, u32, u32)> {
    let mut parts = day.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let date = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&date) {
        return None;
    }
    Some((year, month, date))
}

// Howard Hinnant 的 civil date 算法；只用于订单截止日。
fn days_from_civil(year: i64, month: u32, date: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = i64::from(month);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(date) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let date = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
    (year + i64::from(month <= 2), month, date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_days_handles_month_and_year() {
        assert_eq!(add_days("2026-10-24", 1), "2026-10-25");
        assert_eq!(add_days("2026-10-31", 1), "2026-11-01");
        assert_eq!(add_days("2026-12-31", 1), "2027-01-01");
        assert_eq!(add_days("2028-02-28", 1), "2028-02-29");
        assert_eq!(add_days("bad", 1), "bad");
    }

    #[test]
    fn week_start_is_monday() {
        assert_eq!(week_start("2026-10-19"), "2026-10-19");
        assert_eq!(week_start("2026-10-25"), "2026-10-19");
        assert_eq!(week_start("2026-10-24"), "2026-10-19");
        assert_eq!(week_start("bad"), "bad");
    }

    #[test]
    fn offers_are_deterministic_and_use_unlocked_crops() {
        let content = ContentPack::starter();
        let mut first = FarmOrderState::default();
        let mut second = FarmOrderState::default();
        first.ensure_day("2026-10-24", 10, false, &content);
        second.ensure_day("2026-10-24", 10, false, &content);
        assert_eq!(first.offers.len(), FARM_ORDER_OFFERS);
        assert_eq!(first.offers, second.offers);
        assert!(first
            .offers
            .iter()
            .all(|order| content.crop(&order.crop_id).is_some_and(|crop| crop.unlock_level <= 10)));
        assert!(first.offers.iter().all(|order| order.qty > 0));
        assert!(first.offers.iter().all(|order| order.reward_coins > 0));
        assert!(first
            .offers
            .iter()
            .all(|order| order.expires_day.as_str() > "2026-10-24"));
    }

    #[test]
    fn workshop_built_adds_a_direct_craft_offer() {
        let content = ContentPack::starter();
        let mut state = FarmOrderState::default();
        state.ensure_day("2026-10-24", 10, true, &content);
        assert_eq!(state.offers.len(), FARM_ORDER_OFFERS);
        let craft = state
            .offers
            .iter()
            .find(|order| order.recipe.is_some())
            .expect("built workshop should add a craft order");
        assert!(craft.item_key.starts_with("craft:"));
        let recipe = craft.recipe.unwrap();
        assert!(recipe
            .inputs()
            .iter()
            .all(|(key, _)| key.starts_with("crop:") || key.starts_with("ranch:")));
    }

    #[test]
    fn accept_limit_and_expiry() {
        let content = ContentPack::starter();
        let mut state = FarmOrderState::default();
        state.ensure_day("2026-10-24", 10, false, &content);
        let ids: Vec<String> = state.offers.iter().map(|order| order.id.clone()).collect();
        assert!(state.accept(&ids[0]));
        assert!(state.accept(&ids[1]));
        assert!(!state.accept(&ids[2]));
        let expired = state.ensure_day("2026-11-02", 10, false, &content);
        assert_eq!(expired.len(), 2);
        assert!(state.active.is_empty());
        assert_eq!(state.offers.len(), FARM_ORDER_OFFERS);
    }
}
