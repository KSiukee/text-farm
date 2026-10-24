//! 鱼塘：把钓到的鱼暂养、投喂催肥，长到商品规格再捕。
//!
//! v2 走“少操心”路线：水质收成一个分数 + 一句人话建议；留种开关自动来鱼苗；
//! 每天一条看得懂的鱼塘订单；手记只记 4 个里程碑。内部仍保留四值，但不摆到台面上。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::fishing::{self, FishDef};
use crate::model::Season;

pub const POND_BUILD_LEVEL: u32 = 8;
pub const POND_BUILD_COST: u64 = 8_000;
pub const POND_CAPACITY: u32 = 30;
pub const POND_SPECIES_CAP: usize = 4;
pub const POND_FEED_PER_DAY: u32 = 3;
pub const POND_FEED_GROWTH_MS: i64 = 12 * 60 * 60 * 1000;
pub const POND_FEED_AMMONIA: u32 = 8;
pub const POND_CLEAN_GAIN: u32 = 50;
pub const POND_CLEAN_AMMONIA: u32 = 15;
pub const POND_AERATOR_COST: u64 = 2_500;
pub const POND_PLANTS_COST: u64 = 1_200;
pub const POND_FRY_COUNT: u32 = 3;
pub const POND_FRY_WEIGHT_TENTHS: u32 = 5;
pub const POND_FRY_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;
pub const POND_ORDER_QTY_MIN: u32 = 2;
pub const POND_ORDER_QTY_MAX: u32 = 4;
pub const POND_ORDER_WEIGHT_MARGIN: u32 = 2;
pub const POND_MILESTONE_FIRST_STOCK: &str = "first_stock";
pub const POND_MILESTONE_FIRST_HARVEST: &str = "first_harvest";
pub const POND_MILESTONE_FIRST_FRY: &str = "first_fry";
pub const POND_MILESTONE_HUNDRED: &str = "hundred_harvest";
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PondFish {
    pub fish_id: String,
    pub count: u32,
    /// 平均重量（0.1kg）。
    pub weight_tenths: u32,
    pub quality: u8,
    /// 累计生长毫秒；每满 24h 长 1 个 0.1kg。
    #[serde(default)]
    pub growth_ms: i64,
    #[serde(default)]
    pub stocked_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct PondOrder {
    pub id: String,
    pub fish_id: String,
    pub fish_name: String,
    pub qty: u32,
    pub min_weight_tenths: u32,
    pub reward_coins: u64,
}

impl PondOrder {
    pub fn can_deliver(&self, pond: &Pond) -> bool {
        pond.fish.iter().any(|fish| {
            fish.fish_id == self.fish_id
                && fish.count >= self.qty
                && fish.weight_tenths >= self.min_weight_tenths
        })
    }

    pub fn have(&self, pond: &Pond) -> (u32, u32) {
        pond.fish
            .iter()
            .find(|fish| fish.fish_id == self.fish_id)
            .map(|fish| (fish.count, fish.weight_tenths))
            .unwrap_or((0, 0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Pond {
    pub index: u8,
    pub oxygen: u32,
    pub cleanliness: u32,
    pub ammonia: u32,
    /// 水温（0.1°C）。
    pub temperature_tenths: i32,
    pub fish: Vec<PondFish>,
    pub last_tick_at: i64,
    pub last_clean_day: String,
    pub last_feed_day: String,
    pub feed_today: u32,
    pub aerator: bool,
    pub plants: bool,
    #[serde(default)]
    pub low_oxygen_logged: bool,
    /// 留种：每天自动来一窝鱼苗。
    #[serde(default)]
    pub auto_fry: bool,
    #[serde(default)]
    pub last_fry_at: i64,
}

impl Pond {
    pub fn new(index: u8, now: i64) -> Self {
        Self {
            index,
            oxygen: 80,
            cleanliness: 100,
            ammonia: 0,
            temperature_tenths: 180,
            fish: Vec::new(),
            last_tick_at: now,
            last_clean_day: String::new(),
            last_feed_day: String::new(),
            feed_today: 0,
            aerator: false,
            plants: false,
            low_oxygen_logged: false,
            auto_fry: false,
            last_fry_at: now,
        }
    }

    pub fn total_count(&self) -> u32 {
        self.fish.iter().map(|fish| fish.count).sum()
    }

    pub fn species_count(&self) -> usize {
        self.fish.len()
    }

    pub fn set_season(&mut self, season: Season) {
        self.temperature_tenths = match season {
            Season::Spring => 180,
            Season::Summer => 270,
            Season::Autumn => 200,
            Season::Winter => 60,
        };
    }

    pub fn temperature_c(&self) -> i32 {
        self.temperature_tenths / 10
    }

    pub fn water_score(&self) -> u32 {
        let temperature = self.temperature_tenths as f64 / 10.0;
        let temp_score = (100.0 - (temperature - 22.0).abs() * 4.0).clamp(0.0, 100.0) as u32;
        let oxygen = self.oxygen.min(100);
        let clean = self.cleanliness.min(100);
        let ammonia = 100u32.saturating_sub(self.ammonia.min(100));
        (oxygen * 4 + clean * 2 + ammonia * 2 + temp_score * 2) / 10
    }

    /// 留种：塘里有商品规格的鱼时，每 24 小时自动来 3 条鱼苗（受容量限制）。
    pub fn try_fry(&mut self, now: i64) -> Option<(String, u32)> {
        if !self.auto_fry || self.fish.is_empty() {
            return None;
        }
        if self.last_fry_at > 0 && now.saturating_sub(self.last_fry_at) < POND_FRY_INTERVAL_MS {
            return None;
        }
        let free = POND_CAPACITY.saturating_sub(self.total_count());
        if free < 1 {
            return None;
        }
        let parent = self
            .fish
            .iter()
            .find(|fish| fish.weight_tenths >= market_weight_tenths(&fish.fish_id))
            .cloned()?;
        let count = free.min(POND_FRY_COUNT);
        self.last_fry_at = now;
        self.stock(
            &parent.fish_id,
            count,
            POND_FRY_WEIGHT_TENTHS,
            parent.quality,
            now,
        );
        Some((parent.fish_id, count))
    }

    /// 投喂：所有鱼 +12h 生长，氨氮 +8。
    pub fn feed(&mut self) {
        for fish in &mut self.fish {
            fish.growth_ms = fish.growth_ms.saturating_add(POND_FEED_GROWTH_MS);
        }
        self.ammonia = (self.ammonia + POND_FEED_AMMONIA).min(100);
    }

    pub fn clean(&mut self) {
        self.cleanliness = (self.cleanliness + POND_CLEAN_GAIN).min(100);
        self.ammonia = self.ammonia.saturating_sub(POND_CLEAN_AMMONIA);
    }

    pub fn stock(&mut self, fish_id: &str, count: u32, weight_tenths: u32, quality: u8, now: i64) {
        if let Some(existing) = self.fish.iter_mut().find(|fish| fish.fish_id == fish_id) {
            let total = existing.count.saturating_add(count).max(1);
            let weight_sum = u64::from(existing.weight_tenths) * u64::from(existing.count)
                + u64::from(weight_tenths) * u64::from(count);
            let quality_sum =
                u32::from(existing.quality) * existing.count + u32::from(quality) * count;
            existing.count = total;
            existing.weight_tenths = (weight_sum / u64::from(total)) as u32;
            existing.quality = ((quality_sum / total).clamp(1, 3)) as u8;
        } else {
            self.fish.push(PondFish {
                fish_id: fish_id.to_string(),
                count,
                weight_tenths,
                quality: quality.clamp(1, 3),
                growth_ms: 0,
                stocked_at: now,
            });
        }
    }

    pub fn remove_fish(&mut self, fish_id: &str) -> Option<PondFish> {
        let index = self.fish.iter().position(|fish| fish.fish_id == fish_id)?;
        Some(self.fish.remove(index))
    }

    /// 从某组鱼里捞走 `qty` 条；捞空则删组。返回实际捞走的条数。
    pub fn take_fish(&mut self, fish_id: &str, qty: u32) -> u32 {
        let Some(index) = self.fish.iter().position(|fish| fish.fish_id == fish_id) else {
            return 0;
        };
        let taken = qty.min(self.fish[index].count);
        self.fish[index].count -= taken;
        if self.fish[index].count == 0 {
            self.fish.remove(index);
        }
        taken
    }

    /// 按小时推进水质与生长；返回水质是否变化。
    pub fn tick(&mut self, now: i64) -> bool {
        if now <= self.last_tick_at {
            return false;
        }
        let elapsed = now - self.last_tick_at;
        let hours = (elapsed / 3_600_000).min(24 * 30) as u32;
        if hours == 0 {
            return false;
        }
        self.last_tick_at = now;
        let density = self.total_count();
        let temperature = self.temperature_c();
        let mut target = 85.0
            - f64::from((temperature - 20).max(0)) * 2.0
            - f64::from(density) * 0.5;
        if self.aerator {
            target += 12.0;
        }
        if self.plants {
            target += 6.0;
        }
        let target = target.clamp(10.0, 100.0) as u32;
        self.oxygen = move_toward(self.oxygen, target, hours * 3);
        let ammonia_gain = hours * (1 + density / 10) / 2;
        self.ammonia = (self.ammonia + ammonia_gain)
            .saturating_sub(if self.plants { hours / 2 } else { 0 })
            .min(100);
        self.cleanliness = self.cleanliness.saturating_sub(hours / 2);

        let score = self.water_score();
        let growing = self.oxygen >= 20;
        for fish in &mut self.fish {
            let market = market_weight_tenths(&fish.fish_id);
            if fish.weight_tenths >= market {
                fish.growth_ms = 0;
                continue;
            }
            if !growing || score == 0 {
                continue;
            }
            let gain = elapsed.saturating_mul(i64::from(score)) / 100;
            fish.growth_ms = fish.growth_ms.saturating_add(gain);
            while fish.growth_ms >= DAY_MS && fish.weight_tenths < market {
                fish.weight_tenths += 1;
                fish.growth_ms -= DAY_MS;
            }
            if fish.weight_tenths >= market {
                fish.growth_ms = 0;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PondState {
    pub ponds: Vec<Pond>,
    pub order_day: String,
    pub order: Option<PondOrder>,
    pub order_done: bool,
    /// 手记里程碑：first_stock/first_harvest/first_fry/hundred_harvest。
    pub milestones: BTreeSet<String>,
    pub harvested_total: u32,
}

impl PondState {
    pub fn pond(&self) -> Option<&Pond> {
        self.ponds.first()
    }

    pub fn pond_mut(&mut self) -> Option<&mut Pond> {
        self.ponds.first_mut()
    }

    pub fn built(&self) -> bool {
        !self.ponds.is_empty()
    }

    /// 当天订单：每天一条，交付后当天不再生成。
    pub fn ensure_order(&mut self, today: &str, pond: &Pond) -> bool {
        if self.order_day != today {
            self.order_day = today.to_string();
            self.order = None;
            self.order_done = false;
        }
        if self.order_done || self.order.is_some() {
            return false;
        }
        self.order = generate_order(today, pond);
        self.order.is_some()
    }

    pub fn note_stock(&mut self) {
        self.milestones.insert(POND_MILESTONE_FIRST_STOCK.into());
    }

    pub fn note_harvest(&mut self, count: u32) {
        self.milestones.insert(POND_MILESTONE_FIRST_HARVEST.into());
        self.harvested_total = self.harvested_total.saturating_add(count);
        if self.harvested_total >= 100 {
            self.milestones.insert(POND_MILESTONE_HUNDRED.into());
        }
    }

    pub fn note_fry(&mut self) {
        self.milestones.insert(POND_MILESTONE_FIRST_FRY.into());
    }

    pub fn milestone(&self, key: &str) -> bool {
        self.milestones.contains(key)
    }
}

/// 商品规格：便宜的鱼小、贵的鱼大（0.1kg）。
pub fn market_weight_tenths(fish_id: &str) -> u32 {
    let price = fishing::fish(fish_id).map(|def| def.price).unwrap_or(50);
    ((12 + price / 15).clamp(12, 45)) as u32
}

/// 鱼塘捕捞价：按当前重量/品质走市场基准价。
pub fn harvest_price(def: &FishDef, weight_tenths: u32, quality: u8) -> u64 {
    fishing::price_for(def, weight_tenths, u32::from(quality), 100)
}

fn generate_order(today: &str, pond: &Pond) -> Option<PondOrder> {
    let candidates: Vec<&PondFish> = pond.fish.iter().filter(|fish| fish.count > 0).collect();
    if candidates.is_empty() {
        return None;
    }
    let roll = roll(today, 0xA00);
    let parent = candidates[(roll as usize) % candidates.len()];
    let def = fishing::fish(&parent.fish_id)?;
    let qty = (POND_ORDER_QTY_MIN + ((roll >> 8) % 3) as u32)
        .min(parent.count)
        .clamp(1, POND_ORDER_QTY_MAX);
    let market = market_weight_tenths(&parent.fish_id);
    let min_weight = market.saturating_sub(POND_ORDER_WEIGHT_MARGIN).max(1);
    let unit = fishing::price_for(def, min_weight, u32::from(parent.quality), 100);
    let reward_coins = unit
        .saturating_mul(u64::from(qty))
        .saturating_mul(130)
        / 100;
    Some(PondOrder {
        id: format!("{today}-p0"),
        fish_id: parent.fish_id.clone(),
        fish_name: def.name.to_string(),
        qty,
        min_weight_tenths: min_weight,
        reward_coins: reward_coins.max(100),
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

fn move_toward(value: u32, target: u32, step: u32) -> u32 {
    if value < target {
        (value + step).min(target)
    } else {
        value.saturating_sub(step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pond_has_sane_water_and_builds_growth() {
        let mut pond = Pond::new(1, 0);
        assert!(pond.water_score() >= 80);
        pond.stock("crucian", 5, 12, 2, 0);
        assert_eq!(pond.total_count(), 5);
        assert_eq!(pond.species_count(), 1);
        let before = pond.fish[0].weight_tenths;
        pond.tick(DAY_MS * 2);
        assert!(pond.fish[0].weight_tenths > before);
        assert!(pond.oxygen <= 100);
        assert!(pond.ammonia > 0);
    }

    #[test]
    fn feed_and_clean_move_water_the_right_way() {
        let mut pond = Pond::new(1, 0);
        pond.stock("crucian", 5, 12, 2, 0);
        let ammonia = pond.ammonia;
        pond.feed();
        assert!(pond.ammonia > ammonia);
        let dirty = pond.cleanliness;
        pond.clean();
        assert!(pond.cleanliness >= dirty);
    }

    #[test]
    fn market_weight_scales_with_price() {
        assert_eq!(market_weight_tenths("white_bait"), 12);
        assert!(market_weight_tenths("crucian") >= 12);
        assert!(market_weight_tenths("jiang_king") >= 30);
    }

    #[test]
    fn auto_fry_needs_ready_fish_and_interval() {
        let mut pond = Pond::new(1, 0);
        pond.auto_fry = true;
        pond.stock("crucian", 5, 20, 2, 0);
        assert!(pond.try_fry(DAY_MS).is_some());
        assert_eq!(pond.total_count(), 8);
        assert!(pond.try_fry(DAY_MS + 1000).is_none());
        assert!(pond.try_fry(DAY_MS * 2 + 1000).is_some());
    }

    #[test]
    fn daily_order_is_deterministic_and_deliverable() {
        let mut pond = Pond::new(1, 0);
        pond.stock("crucian", 5, 20, 2, 0);
        let mut first = PondState::default();
        let mut second = PondState::default();
        assert!(first.ensure_order("2026-10-24", &pond));
        assert!(second.ensure_order("2026-10-24", &pond));
        assert_eq!(first.order, second.order);
        let order = first.order.clone().unwrap();
        assert!(order.qty >= 1 && order.qty <= 4);
        assert!(order.can_deliver(&pond));
    }
}
