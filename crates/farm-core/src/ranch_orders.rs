//! 牧场订单：每天 2 条，NPC 要指定品类 + 品质的蛋/奶/毛或加工品。
//!
//! 交付扣对应品质库存，给金币 + 牧场经验 + 1 份专用饲料。
//! 跨天只清掉过期订单，不扣任何东西。

use serde::{Deserialize, Serialize};

use crate::orders::add_days;
use crate::ranch::{item_def, price_of, quality_name, with_quality};

pub const RANCH_ORDER_OFFERS: usize = 2;
pub const RANCH_ORDER_MAX_ACTIVE: usize = 2;

const NPCS: [&str; 5] = ["阿禾", "小满", "青杏", "老陶", "六月"];
const RAW: [&str; 4] = ["ranch:egg", "ranch:duck_egg", "ranch:milk", "ranch:wool"];
const PROCESSED: [&str; 5] = [
    "ranch:cheese",
    "ranch:butter",
    "ranch:cake",
    "ranch:yarn",
    "ranch:salted_egg",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchOrder {
    pub id: String,
    pub npc_name: String,
    /// 基础物品 key（不带品质后缀）。
    pub base_key: String,
    pub item_name: String,
    pub quality: u8,
    pub qty: u32,
    /// 最后可交付日期（含当天），YYYY-MM-DD。
    pub expires_day: String,
    pub reward_coins: u64,
    pub reward_exp: u64,
    pub reward_feed: u32,
}

impl RanchOrder {
    /// 这条订单要的库存 key（带品质后缀）。
    pub fn item_key(&self) -> String {
        with_quality(&self.base_key, self.quality)
    }

    pub fn quality_label(&self) -> &'static str {
        quality_name(self.quality)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchOrderState {
    pub day: String,
    pub offers: Vec<RanchOrder>,
    pub active: Vec<RanchOrder>,
}

impl RanchOrderState {
    /// 跨天刷新报价；返回当天过期、被清掉的已接订单。
    pub fn ensure_day(&mut self, today: &str, level: u32) -> Vec<RanchOrder> {
        if self.day == today {
            return Vec::new();
        }
        let expired: Vec<RanchOrder> = self
            .active
            .iter()
            .filter(|order| order.expires_day.as_str() < today)
            .cloned()
            .collect();
        self.active
            .retain(|order| order.expires_day.as_str() >= today);
        self.offers = generate_offers(today, level);
        self.day = today.to_string();
        expired
    }

    pub fn accept(&mut self, id: &str) -> Option<RanchOrder> {
        if self.active.len() >= RANCH_ORDER_MAX_ACTIVE {
            return None;
        }
        let index = self.offers.iter().position(|order| order.id == id)?;
        let order = self.offers.remove(index);
        self.active.push(order.clone());
        Some(order)
    }

    pub fn find_offer(&self, id: &str) -> Option<&RanchOrder> {
        self.offers.iter().find(|order| order.id == id)
    }

    pub fn find_active(&self, id: &str) -> Option<&RanchOrder> {
        self.active.iter().find(|order| order.id == id)
    }

    pub fn remove_active(&mut self, id: &str) -> Option<RanchOrder> {
        let index = self.active.iter().position(|order| order.id == id)?;
        Some(self.active.remove(index))
    }
}

fn generate_offers(today: &str, level: u32) -> Vec<RanchOrder> {
    (0..RANCH_ORDER_OFFERS)
        .map(|index| {
            let roll = roll(today, 0x700 + index as u64 * 37);
            // 8 级后开始有人订加工品；12 级后才有精品订单。
            let processed = level >= 8 && roll % 100 < 30;
            let pool: &[&str] = if processed { &PROCESSED } else { &RAW };
            let base = pool[((roll >> 8) as usize) % pool.len()];
            let quality = if level >= 12 && (roll >> 16) % 100 < 25 {
                3
            } else if level >= 5 && (roll >> 16) % 100 < 55 {
                2
            } else {
                1
            };
            let qty = if processed {
                1 + ((roll >> 24) % 2) as u32
            } else {
                2 + ((roll >> 24) % 4) as u32
            };
            let unit_price = price_of(&with_quality(base, quality)).unwrap_or(100);
            let premium = 140 + (roll >> 32) % 21;
            let reward_coins = unit_price
                .saturating_mul(u64::from(qty))
                .saturating_mul(premium)
                / 100;
            let name = item_def(base)
                .map(|def| def.name)
                .unwrap_or(base)
                .to_string();
            RanchOrder {
                id: format!("{today}-r{index}"),
                npc_name: NPCS[((roll >> 40) as usize) % NPCS.len()].to_string(),
                base_key: base.to_string(),
                item_name: name,
                quality,
                qty,
                expires_day: add_days(today, 2),
                reward_coins,
                reward_exp: 20 + u64::from(qty) * 6 + u64::from(quality) * 5,
                reward_feed: 1,
            }
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_are_deterministic_and_two_per_day() {
        let mut first = RanchOrderState::default();
        let mut second = RanchOrderState::default();
        first.ensure_day("2026-10-24", 10);
        second.ensure_day("2026-10-24", 10);
        assert_eq!(first.offers.len(), RANCH_ORDER_OFFERS);
        assert_eq!(first.offers, second.offers);
        assert!(first.offers.iter().all(|order| order.qty > 0));
        assert!(first.offers.iter().all(|order| order.reward_coins > 0));
        assert!(first
            .offers
            .iter()
            .all(|order| order.expires_day.as_str() > "2026-10-24"));
    }

    #[test]
    fn accept_limit_and_expiry_clears_without_penalty() {
        let mut state = RanchOrderState::default();
        state.ensure_day("2026-10-24", 10);
        let ids: Vec<String> = state.offers.iter().map(|order| order.id.clone()).collect();
        assert!(state.accept(&ids[0]).is_some());
        assert!(state.accept(&ids[1]).is_some());
        assert!(state.accept(&ids.get(2).cloned().unwrap_or_default()).is_none());

        let expired = state.ensure_day("2026-11-02", 10);
        assert_eq!(expired.len(), RANCH_ORDER_MAX_ACTIVE);
        assert!(state.active.is_empty());
        assert_eq!(state.offers.len(), RANCH_ORDER_OFFERS);
    }

    #[test]
    fn quality_orders_ask_for_quality_key() {
        let mut state = RanchOrderState::default();
        // 找一个有品质订单的日期（12 级后品质 2/3 更常见）。
        state.ensure_day("2026-10-24", 20);
        let order = state
            .offers
            .iter()
            .find(|order| order.quality > 1)
            .expect("20 级应该能刷出品质订单");
        assert_eq!(order.item_key(), with_quality(&order.base_key, order.quality));
    }
}
