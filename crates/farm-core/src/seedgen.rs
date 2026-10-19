use crate::content::{ContentPack, CropDef, RegionAffinity, Resistances};
use crate::model::{Game, Season};
use crate::Action;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
struct Base {
    name: &'static str,
    icon: &'static str,
    seasons: &'static [Season],
    grow: u32,
}

const BASES: &[Base] = &[
    Base {
        name: "小麦",
        icon: "🌾",
        seasons: &[Season::Spring, Season::Autumn],
        grow: 480,
    },
    Base {
        name: "水稻",
        icon: "🌾",
        seasons: &[Season::Summer, Season::Autumn],
        grow: 600,
    },
    Base {
        name: "玉米",
        icon: "🌽",
        seasons: &[Season::Spring, Season::Summer, Season::Autumn],
        grow: 360,
    },
    Base {
        name: "大豆",
        icon: "🫘",
        seasons: &[Season::Spring, Season::Summer],
        grow: 300,
    },
    Base {
        name: "油菜",
        icon: "🌼",
        seasons: &[Season::Spring, Season::Autumn],
        grow: 240,
    },
    Base {
        name: "高粱",
        icon: "🌾",
        seasons: &[Season::Summer, Season::Autumn],
        grow: 420,
    },
    Base {
        name: "谷子",
        icon: "🌾",
        seasons: &[Season::Spring, Season::Summer],
        grow: 300,
    },
    Base {
        name: "红薯",
        icon: "🍠",
        seasons: &[Season::Summer, Season::Autumn],
        grow: 360,
    },
    Base {
        name: "花生",
        icon: "🥜",
        seasons: &[Season::Spring, Season::Summer],
        grow: 300,
    },
    Base {
        name: "萝卜",
        icon: "🥕",
        seasons: &[Season::Spring, Season::Autumn, Season::Winter],
        grow: 120,
    },
    Base {
        name: "青菜",
        icon: "🥬",
        seasons: &[Season::Spring, Season::Autumn, Season::Winter],
        grow: 60,
    },
    Base {
        name: "草莓",
        icon: "🍓",
        seasons: &[Season::Spring, Season::Summer],
        grow: 180,
    },
    Base {
        name: "葡萄",
        icon: "🍇",
        seasons: &[Season::Summer, Season::Autumn],
        grow: 300,
    },
    Base {
        name: "茶",
        icon: "🍃",
        seasons: &[Season::Spring, Season::Summer],
        grow: 420,
    },
    Base {
        name: "药材",
        icon: "🌿",
        seasons: &[Season::Spring, Season::Autumn, Season::Winter],
        grow: 600,
    },
];

const NORTH_PREFIX: &[&str] = &["北地", "雪原", "寒谷", "塞上", "关东"];
const SOUTH_PREFIX: &[&str] = &["南乡", "水乡", "岭南", "江南", "烟雨"];
const ADJECTIVES: &[&str] = &[
    "玉", "金", "青", "紫", "香", "甜", "糯", "脆", "丰", "早", "晚",
];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn pick<'a, T>(&mut self, values: &'a [T]) -> &'a T {
        &values[(self.next() as usize) % values.len()]
    }

    fn range(&mut self, min: u32, max: u32) -> u32 {
        min + (self.next() as u32) % (max - min + 1)
    }
}

/// 生成一批可读、可校验的种子定义；纯整数、可复现。
pub fn generate(count: usize, seed: u64) -> Vec<CropDef> {
    let count = count.min(200);
    let mut rng = Rng(if seed == 0 { 0x9E37_79B9 } else { seed });
    let mut crops = Vec::with_capacity(count);
    let mut names = std::collections::BTreeSet::new();

    for index in 0..count {
        let base = rng.pick(BASES);
        let (region, prefix) = match rng.range(0, 9) {
            0 | 1 => (RegionAffinity::All, ""),
            2..=5 => (RegionAffinity::North, *rng.pick(NORTH_PREFIX)),
            6..=9 => (RegionAffinity::South, *rng.pick(SOUTH_PREFIX)),
            _ => (RegionAffinity::All, ""),
        };
        let level = rng.range(1, 30);
        let mut name = format!("{}{}{}", prefix, rng.pick(ADJECTIVES), base.name);
        if !names.insert(name.clone()) {
            name = format!("{name}{}", index + 1);
            names.insert(name.clone());
        }

        let grow = base.grow / 4 + level * 12 + rng.range(0, 40);
        let yield_per_season = 2 + level / 4 + rng.range(0, 3);
        let seed_price = 5 + u64::from(level) * 30 + u64::from(rng.range(0, 20));
        let target_per_hour = 30 + u64::from(level) * 4 + u64::from(rng.range(0, 30));
        let target_profit = target_per_hour * u64::from(grow) / 60;
        let sell_price = (seed_price + target_profit) / u64::from(yield_per_season);
        let exp = 2 + level / 2;
        let seasons = base.seasons.to_vec();
        let specialty = rng.range(0, 3);
        let mut resistances = Resistances::default();
        let value = rng.range(20, 50) as u8;
        match specialty {
            0 => resistances.drought = value,
            1 => resistances.flood = value,
            2 => resistances.cold = value,
            _ => resistances.pest = value,
        }

        crops.push(CropDef {
            id: format!("gen_{seed:x}_{index}"),
            name,
            icon: base.icon.into(),
            unlock_level: level,
            grow_minutes: grow,
            regrow_minutes: 0,
            seasons: 1,
            yield_per_season,
            seed_price,
            sell_price,
            exp,
            note: "种子生成器".into(),
            sow_seasons: seasons,
            region,
            resistances,
        });
    }
    crops
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Accepted,
    Revised,
    Rejected,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedMetrics {
    pub profit_per_hour: u64,
    pub exp_per_hour: u64,
    pub resistance_total: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeedReview {
    pub crop: CropDef,
    pub status: ReviewStatus,
    pub reasons: Vec<String>,
    pub metrics: SeedMetrics,
}

fn review_metrics(crop: &CropDef) -> SeedMetrics {
    let revenue = crop.sell_price * u64::from(crop.yield_per_season);
    let profit = revenue.saturating_sub(crop.seed_price);
    let grow = u64::from(crop.grow_minutes.max(1));
    SeedMetrics {
        profit_per_hour: profit.saturating_mul(60) / grow,
        exp_per_hour: u64::from(crop.exp).saturating_mul(60) / grow,
        resistance_total: u32::from(crop.resistances.drought)
            + u32::from(crop.resistances.flood)
            + u32::from(crop.resistances.cold)
            + u32::from(crop.resistances.pest),
    }
}

fn name_ok(name: &str) -> bool {
    let len = name.chars().count();
    (2..=12).contains(&len) && !name.contains(' ') && !name.contains('的') && !name.contains("··")
}

fn clamp_resistances(crop: &mut CropDef) -> bool {
    let mut changed = false;
    let mut values = [
        &mut crop.resistances.drought,
        &mut crop.resistances.flood,
        &mut crop.resistances.cold,
        &mut crop.resistances.pest,
    ];
    for value in &mut values {
        if **value > 50 {
            **value = 50;
            changed = true;
        }
    }
    let total = values.iter().map(|value| u32::from(**value)).sum::<u32>();
    if total > 80 {
        let mut excess = total - 80;
        for value in values {
            let take = excess.min(u32::from(*value));
            *value = value.saturating_sub(take as u8);
            excess -= take;
            if excess == 0 {
                break;
            }
        }
        changed = true;
    }
    changed
}

fn can_simulate(crop: &CropDef) -> Result<(), String> {
    let mut pack = ContentPack::starter();
    pack.crops.push(crop.clone());
    let mut state = Game::new(0).state().clone();
    state.level = 30;
    state.coins = 100_000;
    state.settings.region = match crop.region {
        RegionAffinity::South => crate::Region::South,
        _ => crate::Region::North,
    };
    state.season = crop
        .sow_seasons
        .first()
        .copied()
        .ok_or_else(|| "没有适宜季节".to_string())?;
    state.inventory.clear();
    state.inventory.insert(format!("seed:{}", crop.id), 1);
    let mut game = Game::from_parts(state, pack);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: crop.id.clone(),
        },
        0,
    )
    .map_err(|error| format!("播种失败: {error}"))?;
    let ready = game.snapshot(0).plots[0]
        .crop
        .as_ref()
        .map(|crop| crop.ready_at)
        .ok_or_else(|| "播种后没有作物".to_string())?;
    game.dispatch(Action::Harvest { plot: 1 }, ready)
        .map_err(|error| format!("成熟收获失败: {error}"))?;
    let harvested = game
        .snapshot(ready)
        .inventory
        .iter()
        .find(|item| item.key == format!("crop:{}", crop.id))
        .map(|item| item.qty)
        .unwrap_or(0);
    if harvested == 0 {
        return Err("模拟收获为 0".into());
    }
    Ok(())
}

/// 审核已有的候选种子；供生成器、导入包和人工复检共用。
pub fn review_candidates(generated: Vec<CropDef>) -> Vec<SeedReview> {
    let mut names = BTreeSet::new();
    generated
        .into_iter()
        .map(|mut crop| {
            let mut reasons = Vec::new();
            let mut revised = false;

            if !names.insert(crop.name.clone()) {
                crop.name = format!("{}·改", crop.name);
                reasons.push("重名，已加区分后缀".into());
                revised = true;
            }
            if !name_ok(&crop.name) {
                reasons.push("名字长度/用词不合格，作废".into());
                return SeedReview {
                    metrics: review_metrics(&crop),
                    crop,
                    status: ReviewStatus::Rejected,
                    reasons,
                };
            }
            if clamp_resistances(&mut crop) {
                reasons.push("抗性超过单项 50 / 总和 80，已压回上限".into());
                revised = true;
            }

            let metrics = review_metrics(&crop);
            let reject = if crop.seed_price == 0 || crop.yield_per_season == 0 {
                Some("种子价或产量为 0")
            } else if metrics.profit_per_hour < 5 {
                Some("时收益低于 5")
            } else if metrics.profit_per_hour > 300 {
                Some("时收益高于 300")
            } else if metrics.exp_per_hour > 120 {
                Some("时经验高于 120")
            } else if metrics.resistance_total > 80 {
                Some("抗性总和仍超 80")
            } else {
                None
            };
            if let Some(reason) = reject {
                reasons.push(reason.into());
                return SeedReview {
                    metrics,
                    crop,
                    status: ReviewStatus::Rejected,
                    reasons,
                };
            }
            if let Err(error) = can_simulate(&crop) {
                reasons.push(error);
                return SeedReview {
                    metrics,
                    crop,
                    status: ReviewStatus::Rejected,
                    reasons,
                };
            }

            SeedReview {
                metrics,
                crop,
                status: if revised {
                    ReviewStatus::Revised
                } else {
                    ReviewStatus::Accepted
                },
                reasons,
            }
        })
        .collect()
}

/// 生成后用规则审核 + 单作物时间加速沙盒验证。返回全部候选（含被作废的）。
pub fn review_batch(count: usize, seed: u64) -> Vec<SeedReview> {
    review_candidates(generate(count, seed))
}
