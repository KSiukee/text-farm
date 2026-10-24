//! 鱼塘 v1：把钓到的鱼暂养、投喂催肥、按水质成长，达标后捕捞。
//!
//! 和钓鱼分工：钓鱼负责稀有鱼/鱼王/图鉴；鱼塘负责把普通鱼养大、按重量和品质卖。
//! 和水族箱分工：鱼塘是生产，观赏留在房间。

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
            let quality_sum = u32::from(existing.quality) * existing.count
                + u32::from(quality) * count;
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
}
