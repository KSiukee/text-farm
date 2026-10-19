use serde::{Deserialize, Serialize};

use crate::model::Season;

pub const CONTENT_SCHEMA: u32 = 5;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RegionAffinity {
    #[default]
    All,
    North,
    South,
}

impl RegionAffinity {
    pub fn label(self) -> &'static str {
        match self {
            RegionAffinity::All => "通用",
            RegionAffinity::North => "北方",
            RegionAffinity::South => "南方",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Resistances {
    pub drought: u8,
    pub flood: u8,
    pub cold: u8,
    pub pest: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropDef {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub unlock_level: u32,
    pub grow_minutes: u32,
    pub regrow_minutes: u32,
    pub seasons: u8,
    pub yield_per_season: u32,
    pub seed_price: u64,
    pub sell_price: u64,
    pub exp: u32,
    pub note: String,
    #[serde(default)]
    pub sow_seasons: Vec<Season>,
    #[serde(default)]
    pub region: RegionAffinity,
    #[serde(default)]
    pub resistances: Resistances,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nouns {
    pub farm: String,
    pub owner: String,
    pub plot: String,
    pub currency: String,
}

impl Default for Nouns {
    fn default() -> Self {
        Self {
            farm: "我的农场".into(),
            owner: "农场主".into(),
            plot: "土坑".into(),
            currency: "金币".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPack {
    pub schema: u32,
    pub crops: Vec<CropDef>,
    pub nouns: Nouns,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomyPreview {
    pub crop_id: String,
    pub name: String,
    pub coins_per_hour: u64,
    pub exp_per_hour: u64,
    pub payback_minutes: u64,
    pub warning: Option<String>,
}

pub fn economy_preview(pack: &ContentPack) -> Vec<EconomyPreview> {
    pack.crops
        .iter()
        .map(|crop| {
            let total_minutes = u64::from(crop.grow_minutes)
                + u64::from(crop.seasons.saturating_sub(1)) * u64::from(crop.regrow_minutes);
            let revenue = u64::from(crop.seasons)
                .saturating_mul(u64::from(crop.yield_per_season))
                .saturating_mul(crop.sell_price);
            let profit = revenue.saturating_sub(crop.seed_price);
            let coins_per_hour = profit
                .saturating_mul(60)
                .checked_div(total_minutes)
                .unwrap_or(0);
            let exp_per_hour = u64::from(crop.exp)
                .saturating_mul(u64::from(crop.seasons))
                .saturating_mul(60)
                .checked_div(total_minutes)
                .unwrap_or(0);
            let payback_minutes = crop
                .seed_price
                .saturating_mul(total_minutes)
                .checked_div(revenue)
                .unwrap_or(0);
            let warning = if profit == 0 {
                Some("不赚钱".into())
            } else if crop.seed_price == 0 {
                Some("种子免费".into())
            } else {
                None
            };
            EconomyPreview {
                crop_id: crop.id.clone(),
                name: crop.name.clone(),
                coins_per_hour,
                exp_per_hour,
                payback_minutes,
                warning,
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn crop(
    id: &str,
    name: &str,
    icon: &str,
    unlock_level: u32,
    grow_minutes: u32,
    regrow_minutes: u32,
    seasons: u8,
    yield_per_season: u32,
    seed_price: u64,
    sell_price: u64,
    exp: u32,
) -> CropDef {
    CropDef {
        id: id.into(),
        name: name.into(),
        icon: icon.into(),
        unlock_level,
        grow_minutes,
        regrow_minutes,
        seasons,
        yield_per_season,
        seed_price,
        sell_price,
        exp,
        note: String::new(),
        sow_seasons: Vec::new(),
        region: RegionAffinity::All,
        resistances: Resistances::default(),
    }
}

fn default_sow_seasons(id: &str) -> Vec<Season> {
    match id {
        "caomei" | "yumi" | "kafei" | "yuejiancao" => {
            vec![Season::Spring, Season::Summer, Season::Autumn]
        }
        "ganzhe" => vec![Season::Summer, Season::Autumn],
        "xiangrikui" | "putao" | "huolongguo" => {
            vec![Season::Summer, Season::Autumn]
        }
        "xigua" => vec![Season::Summer],
        "renshen" => vec![Season::Spring, Season::Autumn, Season::Winter],
        "lingzhi" => vec![
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ],
        _ => vec![
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ],
    }
}

impl ContentPack {
    /// 首版中性包：12 种，覆盖 1–30 级。
    pub fn starter() -> Self {
        let mut pack = Self {
            schema: CONTENT_SCHEMA,
            nouns: Nouns::default(),
            crops: vec![
                crop("xiaobaicai", "小青菜", "🥬", 1, 3, 0, 1, 2, 5, 6, 3),
                crop("luobo", "萝卜", "🥕", 1, 10, 0, 1, 4, 10, 8, 6),
                crop("caomei", "草莓", "🍓", 3, 30, 15, 2, 5, 35, 15, 12),
                crop("yumi", "玉米", "🌽", 5, 60, 30, 2, 8, 80, 20, 20),
                crop("ganzhe", "甘蔗", "🎍", 7, 90, 45, 2, 8, 70, 22, 22),
                crop("xiangrikui", "向日葵", "🌻", 8, 120, 0, 1, 20, 150, 24, 30),
                crop("putao", "葡萄", "🍇", 11, 240, 120, 3, 12, 450, 40, 45),
                crop("xigua", "西瓜", "🍉", 14, 480, 0, 1, 40, 900, 45, 60),
                crop("renshen", "人参", "🌿", 18, 720, 0, 1, 10, 1600, 360, 70),
                crop("lingzhi", "灵芝", "🍄", 21, 480, 240, 2, 6, 2000, 240, 80),
                crop(
                    "huolongguo",
                    "火龙果",
                    "🍈",
                    24,
                    600,
                    300,
                    2,
                    10,
                    2900,
                    210,
                    90,
                ),
                crop("kafei", "咖啡", "☕", 27, 360, 180, 3, 8, 3200, 145, 110),
                crop(
                    "yuejiancao",
                    "月见草",
                    "🌼",
                    30,
                    720,
                    0,
                    1,
                    12,
                    4400,
                    450,
                    140,
                ),
            ],
        };
        for crop in &mut pack.crops {
            crop.sow_seasons = default_sow_seasons(&crop.id);
        }
        pack
    }

    pub fn crop(&self, id: &str) -> Option<&CropDef> {
        self.crops.iter().find(|c| c.id == id)
    }

    /// 把内置内容的旧平衡值迁到当前版本；用户改过的数值保持不动。
    pub fn migrate_balance(&mut self) {
        let defaults = ContentPack::starter();
        // v5 新增甘蔗：旧内容包缺这一格时补上；空内容包/已有同名 id 则不动。
        if self.schema < CONTENT_SCHEMA && !self.crops.is_empty() && self.crop("ganzhe").is_none() {
            if let Some(def) = defaults.crop("ganzhe") {
                self.crops.push(def.clone());
            }
        }
        if self.schema >= CONTENT_SCHEMA {
            return;
        }
        let old_balance: [(&str, u64, u64); 7] = [
            ("putao", 350, 45),
            ("xigua", 700, 50),
            ("renshen", 1200, 450),
            ("lingzhi", 1500, 300),
            ("huolongguo", 2200, 260),
            ("kafei", 2600, 180),
            ("yuejiancao", 4000, 500),
        ];
        for crop in &mut self.crops {
            let Some((_, old_seed, old_sell)) =
                old_balance.iter().find(|(id, _, _)| *id == crop.id)
            else {
                continue;
            };
            if crop.seed_price == *old_seed && crop.sell_price == *old_sell {
                if let Some(def) = defaults.crop(&crop.id) {
                    crop.seed_price = def.seed_price;
                    crop.sell_price = def.sell_price;
                }
            }
        }
        for crop in &mut self.crops {
            if crop.sow_seasons.is_empty() {
                crop.sow_seasons = default_sow_seasons(&crop.id);
            }
        }
        self.schema = CONTENT_SCHEMA;
    }

    /// 只报错，不报「建议」。返回空 Vec 表示可用。
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.crops.is_empty() {
            issues.push("内容包没有任何作物".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for c in &self.crops {
            if c.id.trim().is_empty() {
                issues.push("有作物 id 为空".into());
            } else if !seen.insert(c.id.clone()) {
                issues.push(format!("作物 id 重复：{}", c.id));
            }
            if c.name.trim().is_empty() {
                issues.push(format!("{} 的名字为空", c.id));
            }
            if c.unlock_level == 0 || c.unlock_level > 30 {
                issues.push(format!("{} 的解锁等级不在 1–30", c.name));
            }
            if c.grow_minutes == 0 {
                issues.push(format!("{} 的成熟时间为 0", c.name));
            }
            if c.sow_seasons.is_empty() {
                issues.push(format!("{} 没有适宜季节", c.name));
            }
            let mut seasons = Vec::new();
            for season in &c.sow_seasons {
                if seasons.contains(season) {
                    issues.push(format!("{} 的适宜季节重复", c.name));
                }
                seasons.push(*season);
            }
            if !(1..=4).contains(&c.seasons) {
                issues.push(format!("{} 的季数不在 1–4", c.name));
            }
            if c.yield_per_season == 0 {
                issues.push(format!("{} 的产量为 0", c.name));
            }
            if c.seasons > 1 && c.regrow_minutes == 0 {
                issues.push(format!("{} 是多季作物，但再熟时间为 0", c.name));
            }
        }
        issues
    }
}
