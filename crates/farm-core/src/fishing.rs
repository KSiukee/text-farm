//! 钓鱼：鱼种表、钓点、鱼竿、图鉴、主动会话与自动钓鱼。
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const FISHING_BAIT_PRICE: u64 = 10;
pub const FISHING_START_BAIT: u32 = 10;
pub const FISHING_SESSION_TTL_MS: i64 = 60_000;
pub const FISHING_KING_CATCH_MS: i64 = 20_000;
pub const FISHING_GOLDEN_CATCH_MS: i64 = 20_000;
pub const FISHING_AUTO_CAP_MS: i64 = 8 * 60 * 60 * 1000;
pub const FISHING_AUTO_MAX_STEPS: u32 = 96;
pub const FISHING_ROD_MAINTENANCE_CATCHES: u32 = 30;
pub const FISHING_MAX_LEVEL: u32 = crate::skills::DISCIPLINE_MAX_LEVEL;
pub const FISHING_SKILL_MAX: u32 = crate::skills::SKILL_MAX_LEVEL;

pub use crate::skills::SkillDef as FishingSkillDef;
pub use crate::skills::FISHING_SKILLS;
use crate::skills::{DisciplineKind, SkillDef};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FishingSpot {
    #[default]
    Creek,
    River,
    Jiang,
    Sea,
}

impl FishingSpot {
    pub const ALL: [FishingSpot; 4] = [
        FishingSpot::Creek,
        FishingSpot::River,
        FishingSpot::Jiang,
        FishingSpot::Sea,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FishingSpot::Creek => "小溪",
            FishingSpot::River => "河流",
            FishingSpot::Jiang => "江面",
            FishingSpot::Sea => "大海",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            FishingSpot::Creek => "🏞️",
            FishingSpot::River => "🌊",
            FishingSpot::Jiang => "🚢",
            FishingSpot::Sea => "🌅",
        }
    }

    pub fn unlock_level(self) -> u32 {
        match self {
            FishingSpot::Creek => 1,
            FishingSpot::River => 5,
            FishingSpot::Jiang => 12,
            FishingSpot::Sea => 15,
        }
    }

    /// 存档里的稳定键名（与 serde snake_case 一致）。
    pub fn key(self) -> &'static str {
        match self {
            FishingSpot::Creek => "creek",
            FishingSpot::River => "river",
            FishingSpot::Jiang => "jiang",
            FishingSpot::Sea => "sea",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        FishingSpot::ALL
            .iter()
            .copied()
            .find(|spot| spot.key() == key)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "camelCase")]
pub enum BaitQuality {
    #[default]
    Normal,
    Fine,
    Prime,
    Master,
}

impl BaitQuality {
    pub const ALL: [BaitQuality; 4] = [
        BaitQuality::Normal,
        BaitQuality::Fine,
        BaitQuality::Prime,
        BaitQuality::Master,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BaitQuality::Normal => "普通",
            BaitQuality::Fine => "优良",
            BaitQuality::Prime => "精品",
            BaitQuality::Master => "特级",
        }
    }

    pub fn price(self) -> u64 {
        match self {
            BaitQuality::Normal => 10,
            BaitQuality::Fine => 25,
            BaitQuality::Prime => 60,
            BaitQuality::Master => 150,
        }
    }

    /// 品质概率加成：从 roll%100 里扣除，数值越大越容易出高品质。
    pub fn quality_bonus(self) -> u32 {
        match self {
            BaitQuality::Normal => 0,
            BaitQuality::Fine => 8,
            BaitQuality::Prime => 16,
            BaitQuality::Master => 25,
        }
    }

    /// 重量加成（0.1kg）。
    pub fn weight_bonus(self) -> u32 {
        match self {
            BaitQuality::Normal => 0,
            BaitQuality::Fine => 1,
            BaitQuality::Prime => 2,
            BaitQuality::Master => 3,
        }
    }

    pub fn index(self) -> usize {
        match self {
            BaitQuality::Normal => 0,
            BaitQuality::Fine => 1,
            BaitQuality::Prime => 2,
            BaitQuality::Master => 3,
        }
    }
}

/// 饵料种类：品质之外的第二维，偏不同鱼种。旧档/旧计数都按面团算。
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum BaitType {
    #[default]
    Dough,
    Corn,
    Worm,
    Lure,
}

impl BaitType {
    pub const ALL: [BaitType; 4] = [
        BaitType::Dough,
        BaitType::Corn,
        BaitType::Worm,
        BaitType::Lure,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BaitType::Dough => "面团",
            BaitType::Corn => "玉米饵",
            BaitType::Worm => "蚯蚓",
            BaitType::Lure => "拟饵",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            BaitType::Dough => "dough",
            BaitType::Corn => "corn",
            BaitType::Worm => "worm",
            BaitType::Lure => "lure",
        }
    }
}

/// 制造屋的做饵配方；产出都是普通品质，按类型入对应的饵料库存。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaitRecipe {
    Corn,
    Dough,
    Worm,
    Lure,
}

impl BaitRecipe {
    pub const ALL: [BaitRecipe; 4] = [
        BaitRecipe::Corn,
        BaitRecipe::Dough,
        BaitRecipe::Worm,
        BaitRecipe::Lure,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BaitRecipe::Corn => "玉米饵",
            BaitRecipe::Dough => "面团饵",
            BaitRecipe::Worm => "蚯蚓饵",
            BaitRecipe::Lure => "拟饵",
        }
    }

    pub fn bait_type(self) -> BaitType {
        match self {
            BaitRecipe::Corn => BaitType::Corn,
            BaitRecipe::Dough => BaitType::Dough,
            BaitRecipe::Worm => BaitType::Worm,
            BaitRecipe::Lure => BaitType::Lure,
        }
    }

    /// 每次做多少份（普通品质）。
    pub fn output_qty(self) -> u32 {
        match self {
            BaitRecipe::Corn => 10,
            BaitRecipe::Dough => 10,
            BaitRecipe::Worm => 10,
            BaitRecipe::Lure => 5,
        }
    }

    pub fn inputs(self) -> &'static [(&'static str, u32)] {
        match self {
            BaitRecipe::Corn => &[("crop:yumi", 2)],
            BaitRecipe::Dough => &[("crop:ganzhe", 1), ("crop:luobo", 2)],
            BaitRecipe::Worm => &[("crop:xiaobaicai", 2), ("ranch:feed", 1)],
            BaitRecipe::Lure => &[("crop:ganzhe", 2), ("ranch:wool", 1)],
        }
    }

    pub fn duration_ms(self) -> i64 {
        match self {
            BaitRecipe::Corn => 10 * 60_000,
            BaitRecipe::Dough => 10 * 60_000,
            BaitRecipe::Worm => 15 * 60_000,
            BaitRecipe::Lure => 20 * 60_000,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum FishPersonality {
    Lazy,
    #[default]
    Normal,
    Active,
    Fierce,
}

impl FishPersonality {
    pub fn label(self) -> &'static str {
        match self {
            FishPersonality::Lazy => "慵懒",
            FishPersonality::Normal => "普通",
            FishPersonality::Active => "活泼",
            FishPersonality::Fierce => "凶猛",
        }
    }

    fn speed_pct(self) -> u32 {
        match self {
            FishPersonality::Lazy => 100,
            FishPersonality::Normal => 150,
            FishPersonality::Active => 172,
            FishPersonality::Fierce => 192,
        }
    }

    fn escape_pct(self) -> u32 {
        match self {
            FishPersonality::Lazy => 100,
            FishPersonality::Normal => 130,
            FishPersonality::Active => 148,
            FishPersonality::Fierce => 162,
        }
    }

    fn flee_radius_pct(self) -> u32 {
        match self {
            FishPersonality::Lazy => 32,
            FishPersonality::Normal => 58,
            FishPersonality::Active => 68,
            FishPersonality::Fierce => 78,
        }
    }

    fn turn_pct(self) -> u32 {
        match self {
            FishPersonality::Lazy => 100,
            FishPersonality::Normal => 210,
            FishPersonality::Active => 245,
            FishPersonality::Fierce => 275,
        }
    }

    fn target_ms(self) -> (i64, i64) {
        match self {
            FishPersonality::Lazy => (1_100, 2_000),
            FishPersonality::Normal => (500, 950),
            FishPersonality::Active => (400, 850),
            FishPersonality::Fierce => (350, 700),
        }
    }

    fn jerk_add(self) -> u32 {
        match self {
            FishPersonality::Lazy => 5,
            FishPersonality::Normal => 26,
            FishPersonality::Active => 36,
            FishPersonality::Fierce => 46,
        }
    }
}

pub fn pick_personality(def: &FishDef, roll: u64) -> FishPersonality {
    if def.rarity == FishRarity::King {
        return FishPersonality::Fierce;
    }
    match roll % 100 {
        0..=24 => FishPersonality::Lazy,
        25..=64 => FishPersonality::Normal,
        65..=89 => FishPersonality::Active,
        _ => FishPersonality::Fierce,
    }
}

/// 鱼图标素材的默认朝向：true = 头朝右，前端先镜像成朝左，再按游动方向翻转。
/// 清单按 35 张图的实际朝向核对（docs/fishgen/orientation-contact.png）。
pub fn faces_right(id: &str) -> bool {
    matches!(
        id,
        "pale_chub"
            | "catfish"
            | "creek_king"
            | "river_king"
            | "topmouth_culter"
            | "grouper"
            | "spanish_mackerel"
            | "yellowcheek"
            | "hairtail"
            | "large_yellow_croaker"
    )
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "camelCase")]
pub enum RodKind {
    #[default]
    Wood,
    Bamboo,
    Fiberglass,
    Carbon,
}

impl RodKind {
    pub const ALL: [RodKind; 4] = [
        RodKind::Wood,
        RodKind::Bamboo,
        RodKind::Fiberglass,
        RodKind::Carbon,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RodKind::Wood => "木竿",
            RodKind::Bamboo => "竹竿",
            RodKind::Fiberglass => "玻璃钢竿",
            RodKind::Carbon => "碳素竿",
        }
    }

    pub fn cost(self) -> u64 {
        match self {
            RodKind::Wood => 0,
            RodKind::Bamboo => 3_000,
            RodKind::Fiberglass => 12_000,
            RodKind::Carbon => 40_000,
        }
    }

    pub fn unlock_level(self) -> u32 {
        match self {
            RodKind::Wood => 1,
            RodKind::Bamboo => 4,
            RodKind::Fiberglass => 7,
            RodKind::Carbon => 9,
        }
    }

    /// 鱼框高度，百分比。
    pub fn frame_pct(self) -> u32 {
        match self {
            RodKind::Wood => 20,
            RodKind::Bamboo => 18,
            RodKind::Fiberglass => 15,
            RodKind::Carbon => 13,
        }
    }

    pub fn maintenance_cost(self) -> u64 {
        match self {
            RodKind::Wood => 300,
            RodKind::Bamboo => 600,
            RodKind::Fiberglass => 1_200,
            RodKind::Carbon => 1_800,
        }
    }

    /// 保养间隔（条鱼）：高级竿更耐用。
    pub fn durability(self) -> u32 {
        match self {
            RodKind::Wood => 30,
            RodKind::Bamboo => 40,
            RodKind::Fiberglass => 60,
            RodKind::Carbon => 90,
        }
    }

    /// A2 战后售价加成。
    pub fn sell_bonus_pct(self) -> u32 {
        match self {
            RodKind::Wood => 0,
            RodKind::Bamboo => 10,
            RodKind::Fiberglass => 25,
            RodKind::Carbon => 40,
        }
    }

    /// A2 省饵概率（百分点）。
    pub fn bait_save_pct(self) -> u32 {
        match self {
            RodKind::Wood => 0,
            RodKind::Bamboo => 0,
            RodKind::Fiberglass => 10,
            RodKind::Carbon => 20,
        }
    }

    /// G1 金色概率加成（千分点）。
    pub fn golden_bonus_permille(self) -> u64 {
        match self {
            RodKind::Wood => 0,
            RodKind::Bamboo => 5,
            RodKind::Fiberglass => 10,
            RodKind::Carbon => 20,
        }
    }

    /// 高级竿钓鱼经验加成。
    pub fn exp_bonus_pct(self) -> u32 {
        match self {
            RodKind::Wood | RodKind::Bamboo => 0,
            RodKind::Fiberglass => 40,
            RodKind::Carbon => 80,
        }
    }

    pub fn profit_label(self) -> String {
        match self {
            RodKind::Wood => "收益：基准（保养 300/30 条）".into(),
            RodKind::Bamboo => "收益：售价 +10% · 金色 +0.5% · 保养 600/40 条".into(),
            RodKind::Fiberglass => {
                "收益：售价 +25% · 经验 +40% · 省饵 10% · 金色 +1% · 保养 1,200/60 条".into()
            }
            RodKind::Carbon => {
                "收益：售价 +40% · 经验 +80% · 省饵 20% · 金色 +2% · 保养 1,800/90 条".into()
            }
        }
    }

    pub fn auto_interval_ms(self) -> i64 {
        match self {
            RodKind::Wood => 15 * 60 * 1000,
            RodKind::Bamboo => 13 * 60 * 1000,
            RodKind::Fiberglass => 11 * 60 * 1000,
            RodKind::Carbon => 9 * 60 * 1000,
        }
    }

    pub fn auto_interval_label(self) -> String {
        format!("{} 分钟", self.auto_interval_ms() / 60_000)
    }
}

/// 主动钓获的手感奖励：品质 +1 档（最高精品）、重量 +0.1kg；自动钓获不享受。
pub const MANUAL_QUALITY_BONUS: u32 = 1;
pub const MANUAL_WEIGHT_BONUS_TENTHS: u32 = 1;

pub fn apply_manual_bonus(quality: u32, weight_tenths: u32) -> (u32, u32) {
    (
        quality.saturating_add(MANUAL_QUALITY_BONUS).min(3),
        weight_tenths.saturating_add(MANUAL_WEIGHT_BONUS_TENTHS),
    )
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FishRarity {
    Common,
    Uncommon,
    Rare,
    King,
}

impl FishRarity {
    pub fn label(self) -> &'static str {
        match self {
            FishRarity::Common => "普通",
            FishRarity::Uncommon => "优良",
            FishRarity::Rare => "稀有",
            FishRarity::King => "鱼王",
        }
    }
}

/// 本级升到下一级需要的钓鱼经验；满级返回 0。
pub fn fishing_exp_need(level: u32) -> u64 {
    crate::skills::Discipline::need(level)
}

/// 一条鱼的钓鱼经验：稀有度基础 + 品质 + 重量。
pub fn catch_exp(rarity: FishRarity, quality: u32, weight_tenths: u32) -> u64 {
    let base = match rarity {
        FishRarity::Common => 3,
        FishRarity::Uncommon => 5,
        FishRarity::Rare => 8,
        FishRarity::King => 40,
    };
    base + u64::from(quality.saturating_sub(1)) + u64::from(weight_tenths / 10)
}

pub fn skill_def(id: &str) -> Option<&'static SkillDef> {
    crate::skills::skill_def(DisciplineKind::Fishing, id)
}

/// 从 current_level 升到下一级的技能点价格。
pub fn skill_cost(current_level: u32) -> u32 {
    crate::skills::skill_cost(current_level)
}

pub fn skill_level(skills: &BTreeMap<String, u32>, id: &str) -> u32 {
    skills.get(id).copied().unwrap_or(0).min(FISHING_SKILL_MAX)
}

pub fn bait_saver_pct(level: u32) -> u32 {
    level.min(FISHING_SKILL_MAX) * 8
}

pub fn reel_bonus(level: u32) -> u32 {
    level.min(FISHING_SKILL_MAX) * 2
}

pub fn big_fish_bonus_tenths(level: u32) -> u32 {
    level.min(FISHING_SKILL_MAX)
}

/// 自动间隔随「自动化」技能缩短；未点技能时与原值一致。
pub fn auto_interval_ms_with_skill(rod: RodKind, speed_level: u32) -> i64 {
    let pct = 100 - i64::from(speed_level.min(FISHING_SKILL_MAX) * 4);
    rod.auto_interval_ms().saturating_mul(pct) / 100
}

#[derive(Debug, Clone, Copy)]
pub struct FishDef {
    pub id: &'static str,
    pub name: &'static str,
    pub spot: FishingSpot,
    pub rarity: FishRarity,
    pub price: u64,
    pub difficulty: u32,
    /// 同钓点内权重，每个钓点合计 1000。
    pub weight: u32,
}

pub const FISH: [FishDef; 35] = [
    // 小溪
    FishDef {
        id: "white_bait",
        name: "白条",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Common,
        price: 10,
        difficulty: 1,
        weight: 140,
    },
    FishDef {
        id: "crucian",
        name: "鲫鱼",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Common,
        price: 15,
        difficulty: 1,
        weight: 130,
    },
    FishDef {
        id: "loach",
        name: "泥鳅",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Common,
        price: 12,
        difficulty: 2,
        weight: 100,
    },
    FishDef {
        id: "topmouth_gudgeon",
        name: "麦穗鱼",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Common,
        price: 8,
        difficulty: 1,
        weight: 80,
    },
    FishDef {
        id: "chinese_hooksnout",
        name: "马口鱼",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Uncommon,
        price: 20,
        difficulty: 2,
        weight: 120,
    },
    FishDef {
        id: "pale_chub",
        name: "宽鳍鱲",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Uncommon,
        price: 24,
        difficulty: 2,
        weight: 110,
    },
    FishDef {
        id: "yellow_catfish",
        name: "黄颡鱼",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Uncommon,
        price: 30,
        difficulty: 3,
        weight: 100,
    },
    FishDef {
        id: "stone_grouper",
        name: "溪石斑",
        spot: FishingSpot::Creek,
        rarity: FishRarity::Rare,
        price: 40,
        difficulty: 3,
        weight: 215,
    },
    FishDef {
        id: "creek_king",
        name: "溪王·金鳞",
        spot: FishingSpot::Creek,
        rarity: FishRarity::King,
        price: 400,
        difficulty: 5,
        weight: 5,
    },
    // 河流
    FishDef {
        id: "carp",
        name: "鲤鱼",
        spot: FishingSpot::River,
        rarity: FishRarity::Common,
        price: 30,
        difficulty: 2,
        weight: 130,
    },
    FishDef {
        id: "grass_carp",
        name: "草鱼",
        spot: FishingSpot::River,
        rarity: FishRarity::Common,
        price: 35,
        difficulty: 2,
        weight: 120,
    },
    FishDef {
        id: "silver_carp",
        name: "鲢鱼",
        spot: FishingSpot::River,
        rarity: FishRarity::Common,
        price: 32,
        difficulty: 2,
        weight: 110,
    },
    FishDef {
        id: "bream",
        name: "鳊鱼",
        spot: FishingSpot::River,
        rarity: FishRarity::Common,
        price: 38,
        difficulty: 2,
        weight: 90,
    },
    FishDef {
        id: "bighead_carp",
        name: "鳙鱼",
        spot: FishingSpot::River,
        rarity: FishRarity::Uncommon,
        price: 45,
        difficulty: 3,
        weight: 130,
    },
    FishDef {
        id: "black_carp",
        name: "青鱼",
        spot: FishingSpot::River,
        rarity: FishRarity::Uncommon,
        price: 55,
        difficulty: 3,
        weight: 110,
    },
    FishDef {
        id: "catfish",
        name: "鲶鱼",
        spot: FishingSpot::River,
        rarity: FishRarity::Uncommon,
        price: 50,
        difficulty: 3,
        weight: 90,
    },
    FishDef {
        id: "topmouth_culter",
        name: "翘嘴鲌",
        spot: FishingSpot::River,
        rarity: FishRarity::Rare,
        price: 70,
        difficulty: 4,
        weight: 215,
    },
    FishDef {
        id: "river_king",
        name: "河王·赤尾",
        spot: FishingSpot::River,
        rarity: FishRarity::King,
        price: 800,
        difficulty: 5,
        weight: 5,
    },
    // 江面
    FishDef {
        id: "bronze_carp",
        name: "铜鱼",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Common,
        price: 60,
        difficulty: 3,
        weight: 170,
    },
    FishDef {
        id: "river_perch",
        name: "江鲈",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Common,
        price: 70,
        difficulty: 3,
        weight: 150,
    },
    FishDef {
        id: "red_eye_chub",
        name: "赤眼鳟",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Common,
        price: 55,
        difficulty: 2,
        weight: 130,
    },
    FishDef {
        id: "jiang_tuan",
        name: "江团",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Uncommon,
        price: 120,
        difficulty: 4,
        weight: 130,
    },
    FishDef {
        id: "mandarin_fish",
        name: "鳜鱼",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Uncommon,
        price: 100,
        difficulty: 4,
        weight: 110,
    },
    FishDef {
        id: "mongolian_culter",
        name: "蒙古鲌",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Uncommon,
        price: 80,
        difficulty: 3,
        weight: 90,
    },
    FishDef {
        id: "eel",
        name: "鳗鲡",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Rare,
        price: 160,
        difficulty: 4,
        weight: 120,
    },
    FishDef {
        id: "yellowcheek",
        name: "鳡鱼",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::Rare,
        price: 220,
        difficulty: 5,
        weight: 95,
    },
    FishDef {
        id: "jiang_king",
        name: "江王·铜头",
        spot: FishingSpot::Jiang,
        rarity: FishRarity::King,
        price: 1400,
        difficulty: 5,
        weight: 5,
    },
    // 大海
    FishDef {
        id: "hairtail",
        name: "带鱼",
        spot: FishingSpot::Sea,
        rarity: FishRarity::Common,
        price: 80,
        difficulty: 3,
        weight: 280,
    },
    FishDef {
        id: "sea_perch",
        name: "海鲈",
        spot: FishingSpot::Sea,
        rarity: FishRarity::Common,
        price: 90,
        difficulty: 3,
        weight: 220,
    },
    FishDef {
        id: "large_yellow_croaker",
        name: "大黄鱼",
        spot: FishingSpot::Sea,
        rarity: FishRarity::Uncommon,
        price: 180,
        difficulty: 4,
        weight: 110,
    },
    FishDef {
        id: "silver_pomfret",
        name: "银鲳",
        spot: FishingSpot::Sea,
        rarity: FishRarity::Uncommon,
        price: 120,
        difficulty: 3,
        weight: 100,
    },
    FishDef {
        id: "spanish_mackerel",
        name: "蓝点马鲛",
        spot: FishingSpot::Sea,
        rarity: FishRarity::Uncommon,
        price: 150,
        difficulty: 4,
        weight: 90,
    },
    FishDef {
        id: "red_seabream",
        name: "真鲷",
        spot: FishingSpot::Sea,
        rarity: FishRarity::Uncommon,
        price: 140,
        difficulty: 4,
        weight: 75,
    },
    FishDef {
        id: "grouper",
        name: "青石斑",
        spot: FishingSpot::Sea,
        rarity: FishRarity::Rare,
        price: 220,
        difficulty: 4,
        weight: 120,
    },
    FishDef {
        id: "bluefin_king",
        name: "海王·蓝鳍",
        spot: FishingSpot::Sea,
        rarity: FishRarity::King,
        price: 2200,
        difficulty: 5,
        weight: 5,
    },
];

pub fn fish(id: &str) -> Option<&'static FishDef> {
    FISH.iter().find(|def| def.id == id)
}

pub fn fish_list() -> &'static [FishDef] {
    &FISH
}

pub fn spot_fish(spot: FishingSpot) -> impl Iterator<Item = &'static FishDef> {
    FISH.iter().filter(move |def| def.spot == spot)
}

pub fn spot_fish_count(spot: FishingSpot) -> usize {
    spot_fish(spot).count()
}

/// 分钓点集齐奖励：金币 + 鱼饵（种类, 品质, 数量）。
pub fn spot_reward(spot: FishingSpot) -> (u64, BaitType, BaitQuality, u32) {
    match spot {
        FishingSpot::Creek => (2_000, BaitType::Worm, BaitQuality::Fine, 10),
        FishingSpot::River => (6_000, BaitType::Corn, BaitQuality::Prime, 5),
        FishingSpot::Jiang => (15_000, BaitType::Lure, BaitQuality::Prime, 5),
        FishingSpot::Sea => (40_000, BaitType::Lure, BaitQuality::Master, 5),
    }
}

pub const FULL_CODEX_REWARD: u64 = 100_000;
pub const FULL_CODEX_BAIT: (BaitType, BaitQuality, u32) = (BaitType::Lure, BaitQuality::Master, 10);

/// 不同饵料偏好的鱼种；命中时权重 ×2。
const CORN_FISH: [&str; 9] = [
    "carp",
    "grass_carp",
    "silver_carp",
    "bream",
    "bighead_carp",
    "black_carp",
    "crucian",
    "bronze_carp",
    "red_eye_chub",
];
const WORM_FISH: [&str; 6] = [
    "loach",
    "yellow_catfish",
    "catfish",
    "eel",
    "jiang_tuan",
    "stone_grouper",
];
const LURE_FISH: [&str; 10] = [
    "topmouth_culter",
    "yellowcheek",
    "mandarin_fish",
    "sea_perch",
    "spanish_mackerel",
    "grouper",
    "red_seabream",
    "chinese_hooksnout",
    "pale_chub",
    "mongolian_culter",
];

/// 饵料种类对某条鱼的权重百分比（100 = 不偏）。
pub fn bait_weight(bait_type: BaitType, fish_id: &str) -> u32 {
    match bait_type {
        BaitType::Dough => 100,
        BaitType::Corn if CORN_FISH.contains(&fish_id) => 200,
        BaitType::Worm if WORM_FISH.contains(&fish_id) => 200,
        BaitType::Lure if LURE_FISH.contains(&fish_id) => 200,
        _ => 100,
    }
}

/// 主动钓鱼按权重抽鱼；roll 由引擎按时间/会话生成。
pub fn pick_fish(spot: FishingSpot, roll: u64) -> &'static FishDef {
    pick_fish_with_bait(spot, roll, BaitType::Dough)
}

pub fn pick_fish_with_bait(spot: FishingSpot, roll: u64, bait_type: BaitType) -> &'static FishDef {
    let weight =
        |def: &FishDef| u64::from(def.weight) * u64::from(bait_weight(bait_type, def.id)) / 100;
    let total: u64 = spot_fish(spot).map(weight).sum();
    let mut point = if total == 0 { 0 } else { roll % total };
    let mut last = spot_fish(spot).next().expect("spot has fish");
    for def in spot_fish(spot) {
        last = def;
        let w = weight(def);
        if point < w {
            return def;
        }
        point -= w;
    }
    last
}

/// 自动钓鱼只钓“已点亮图鉴”的鱼（鱼王除外），按稀有度权重。
/// 唯一例外：新号图鉴为空且在小溪开自动，先给一条初始鱼点亮图鉴；
/// 之后自动钓鱼不再解锁任何新鱼。
pub fn pick_auto_fish(
    spot: FishingSpot,
    roll: u64,
    codex: &BTreeMap<String, FishCodexEntry>,
) -> Option<&'static FishDef> {
    pick_auto_fish_with_bait(spot, roll, codex, BaitType::Dough)
}

pub fn pick_auto_fish_with_bait(
    spot: FishingSpot,
    roll: u64,
    codex: &BTreeMap<String, FishCodexEntry>,
    bait_type: BaitType,
) -> Option<&'static FishDef> {
    let pool: Vec<&FishDef> = spot_fish(spot)
        .filter(|def| def.rarity != FishRarity::King)
        .filter(|def| {
            codex
                .get(def.id)
                .map(|entry| entry.count > 0)
                .unwrap_or(false)
        })
        .collect();

    if pool.is_empty() {
        if spot == FishingSpot::Creek && codex.is_empty() {
            return spot_fish(spot).next();
        }
        return None;
    }

    let rarity_weight = |def: &FishDef| match def.rarity {
        FishRarity::Common => 60u64,
        FishRarity::Uncommon => 30,
        FishRarity::Rare => 10,
        FishRarity::King => 0,
    };
    let weight =
        |def: &FishDef| rarity_weight(def) * u64::from(bait_weight(bait_type, def.id)) / 100;
    let total: u64 = pool.iter().map(|def| weight(def)).sum();
    if total == 0 {
        return None;
    }
    let mut point = roll % total;
    for def in &pool {
        let w = weight(def);
        if point < w {
            return Some(def);
        }
        point -= w;
    }
    pool.first().copied()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FishingStats {
    #[serde(default)]
    pub casts: u64,
    #[serde(default)]
    pub catches: u64,
    #[serde(default)]
    pub fails: u64,
    #[serde(default)]
    pub auto_catches: u64,
    #[serde(default)]
    pub golden_catches: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FishCodexEntry {
    #[serde(default)]
    pub first_at: i64,
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub best_weight_tenths: u32,
    #[serde(default)]
    pub best_quality: u32,
    #[serde(default)]
    pub best_price: u64,
    #[serde(default)]
    pub manual_count: u64,
    #[serde(default)]
    pub manual_best_weight_tenths: u32,
    #[serde(default)]
    pub manual_best_quality: u32,
    #[serde(default)]
    pub manual_best_price: u64,
    #[serde(default)]
    pub auto_count: u64,
    #[serde(default)]
    pub auto_best_weight_tenths: u32,
    #[serde(default)]
    pub auto_best_quality: u32,
    #[serde(default)]
    pub auto_best_price: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingSession {
    pub id: u64,
    pub spot: FishingSpot,
    #[serde(rename = "fishId")]
    pub fish_id: String,
    pub started_at: i64,
    pub expires_at: i64,
    #[serde(default)]
    pub bait_quality: BaitQuality,
    #[serde(default)]
    pub personality: FishPersonality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoFishing {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub spot: FishingSpot,
    #[serde(default)]
    pub last_at: i64,
    #[serde(default)]
    pub rod: RodKind,
    #[serde(default)]
    pub bait_quality: BaitQuality,
    #[serde(default)]
    pub bait_type: BaitType,
}

impl Default for AutoFishing {
    fn default() -> Self {
        Self {
            enabled: false,
            spot: FishingSpot::Creek,
            last_at: 0,
            rod: RodKind::Wood,
            bait_quality: BaitQuality::Normal,
            bait_type: BaitType::Dough,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutoReport {
    #[serde(default)]
    pub at: i64,
    #[serde(default)]
    pub bait_bought: u32,
    #[serde(default)]
    pub bait_used: u32,
    #[serde(default)]
    pub coins_spent: u64,
    #[serde(default)]
    pub catches: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingState {
    #[serde(default)]
    pub rod: RodKind,
    #[serde(default = "default_bait")]
    pub bait: u32,
    #[serde(default)]
    pub bait_fine: u32,
    #[serde(default)]
    pub bait_prime: u32,
    #[serde(default)]
    pub bait_master: u32,
    #[serde(default)]
    pub bait_corn: [u32; 4],
    #[serde(default)]
    pub bait_worm: [u32; 4],
    #[serde(default)]
    pub bait_lure: [u32; 4],
    #[serde(default)]
    pub active_bait: BaitQuality,
    #[serde(default)]
    pub active_bait_type: BaitType,
    /// 已拥有的鱼竿；旧档没有这个字段时按「当前竿及以下都拥有」回填。
    #[serde(default)]
    pub owned_rods: BTreeSet<RodKind>,
    #[serde(default)]
    pub rod_wear: BTreeMap<RodKind, u32>,
    #[serde(default)]
    pub locked_fish: BTreeSet<String>,
    #[serde(default)]
    pub auto: AutoFishing,
    #[serde(default)]
    pub session: Option<FishingSession>,
    #[serde(default)]
    pub codex: BTreeMap<String, FishCodexEntry>,
    /// 已领过的分钓点图鉴奖励（钓点名）。
    #[serde(default)]
    pub spot_rewards: BTreeSet<String>,
    /// 金色变异图鉴：鱼 id → 金色累计次数；不影响库存/鱼价，只是图鉴格。
    #[serde(default)]
    pub golden_codex: BTreeMap<String, u64>,
    /// 35/35 全图鉴奖励是否已领。
    #[serde(default)]
    pub full_codex_claimed: bool,
    #[serde(default)]
    pub fish_batches: BTreeMap<String, FishBatch>,
    #[serde(default)]
    pub last_catch: Option<FishCatchResult>,
    #[serde(default)]
    pub last_auto_report: Option<AutoReport>,
    #[serde(default)]
    pub stats: FishingStats,
    #[serde(default)]
    pub session_seq: u64,
    /// v31→v32 迁移时记下的「按农场等级当时已解锁」的钓点；之后不再新增。
    #[serde(default)]
    pub legacy_unlocks: BTreeSet<String>,
}

fn default_bait() -> u32 {
    FISHING_START_BAIT
}

impl Default for FishingState {
    fn default() -> Self {
        Self::new()
    }
}

impl FishingState {
    pub fn new() -> Self {
        Self {
            rod: RodKind::Wood,
            bait: FISHING_START_BAIT,
            bait_fine: 0,
            bait_prime: 0,
            bait_master: 0,
            bait_corn: [0; 4],
            bait_worm: [0; 4],
            bait_lure: [0; 4],
            active_bait: BaitQuality::Normal,
            active_bait_type: BaitType::Dough,
            owned_rods: BTreeSet::from([RodKind::Wood]),
            rod_wear: BTreeMap::new(),
            locked_fish: BTreeSet::new(),
            auto: AutoFishing::default(),
            session: None,
            codex: BTreeMap::new(),
            spot_rewards: BTreeSet::new(),
            golden_codex: BTreeMap::new(),
            full_codex_claimed: false,
            fish_batches: BTreeMap::new(),
            last_catch: None,
            last_auto_report: None,
            stats: FishingStats::default(),
            session_seq: 0,
            legacy_unlocks: BTreeSet::new(),
        }
    }

    pub fn ensure(&mut self) {
        if self.session_seq == 0 {
            self.session_seq = 1;
        }
        if self.owned_rods.is_empty() {
            self.owned_rods = RodKind::ALL
                .iter()
                .copied()
                .filter(|rod| *rod <= self.rod)
                .collect();
        }
        self.owned_rods.insert(RodKind::Wood);
        for entry in self.codex.values_mut() {
            if entry.count > 0 && entry.manual_count == 0 && entry.auto_count == 0 {
                entry.manual_count = entry.count;
                entry.manual_best_weight_tenths = entry.best_weight_tenths;
                entry.manual_best_quality = entry.best_quality;
                entry.manual_best_price = entry.best_price;
            }
        }
    }

    /// 钓点是否可用：钓鱼等级够，或者迁移前就已经按农场等级解锁（legacy）。
    pub fn spot_unlocked(&self, fishing_level: u32, spot: FishingSpot) -> bool {
        fishing_level >= spot.unlock_level() || self.legacy_unlocks.contains(spot.key())
    }

    /// 鱼竿是否可购买/升级：只看钓鱼等级；已拥有的竿不受影响。
    pub fn rod_unlocked(&self, fishing_level: u32, rod: RodKind) -> bool {
        fishing_level >= rod.unlock_level()
    }

    pub fn owns_rod(&self, rod: RodKind) -> bool {
        self.owned_rods.contains(&rod)
    }

    pub fn caught_count(&self) -> usize {
        self.codex.values().filter(|entry| entry.count > 0).count()
    }

    pub fn spot_caught_count(&self, spot: FishingSpot) -> usize {
        self.codex
            .iter()
            .filter(|(id, entry)| entry.count > 0 && fish(id).is_some_and(|def| def.spot == spot))
            .count()
    }

    pub fn spot_complete(&self, spot: FishingSpot) -> bool {
        self.spot_caught_count(spot) == spot_fish_count(spot)
    }

    pub fn bait_count_of(&self, bait_type: BaitType, quality: BaitQuality) -> u32 {
        match bait_type {
            BaitType::Dough => match quality {
                BaitQuality::Normal => self.bait,
                BaitQuality::Fine => self.bait_fine,
                BaitQuality::Prime => self.bait_prime,
                BaitQuality::Master => self.bait_master,
            },
            BaitType::Corn => self.bait_corn[quality.index()],
            BaitType::Worm => self.bait_worm[quality.index()],
            BaitType::Lure => self.bait_lure[quality.index()],
        }
    }

    pub fn add_bait_of(&mut self, bait_type: BaitType, quality: BaitQuality, qty: u32) {
        let idx = quality.index();
        match bait_type {
            BaitType::Dough => match quality {
                BaitQuality::Normal => self.bait = self.bait.saturating_add(qty),
                BaitQuality::Fine => self.bait_fine = self.bait_fine.saturating_add(qty),
                BaitQuality::Prime => self.bait_prime = self.bait_prime.saturating_add(qty),
                BaitQuality::Master => self.bait_master = self.bait_master.saturating_add(qty),
            },
            BaitType::Corn => {
                self.bait_corn[idx] = self.bait_corn[idx].saturating_add(qty);
            }
            BaitType::Worm => {
                self.bait_worm[idx] = self.bait_worm[idx].saturating_add(qty);
            }
            BaitType::Lure => {
                self.bait_lure[idx] = self.bait_lure[idx].saturating_add(qty);
            }
        }
    }

    pub fn take_bait_of(&mut self, bait_type: BaitType, quality: BaitQuality) -> bool {
        if self.bait_count_of(bait_type, quality) == 0 {
            return false;
        }
        let idx = quality.index();
        match bait_type {
            BaitType::Dough => {
                let slot = match quality {
                    BaitQuality::Normal => &mut self.bait,
                    BaitQuality::Fine => &mut self.bait_fine,
                    BaitQuality::Prime => &mut self.bait_prime,
                    BaitQuality::Master => &mut self.bait_master,
                };
                *slot -= 1;
            }
            BaitType::Corn => self.bait_corn[idx] -= 1,
            BaitType::Worm => self.bait_worm[idx] -= 1,
            BaitType::Lure => self.bait_lure[idx] -= 1,
        }
        true
    }

    /// 旧接口默认面团饵。
    pub fn bait_count(&self, quality: BaitQuality) -> u32 {
        self.bait_count_of(BaitType::Dough, quality)
    }

    pub fn add_bait(&mut self, quality: BaitQuality, qty: u32) {
        self.add_bait_of(BaitType::Dough, quality, qty);
    }

    pub fn take_bait(&mut self, quality: BaitQuality) -> bool {
        self.take_bait_of(BaitType::Dough, quality)
    }

    pub fn rod_wear(&self, rod: RodKind) -> u32 {
        self.rod_wear.get(&rod).copied().unwrap_or(0)
    }

    pub fn add_rod_wear(&mut self, rod: RodKind) {
        let wear = self.rod_wear.entry(rod).or_insert(0);
        *wear = (*wear + 1).min(rod.durability());
    }

    pub fn needs_maintenance(&self, rod: RodKind) -> bool {
        self.rod_wear(rod) >= rod.durability()
    }

    pub fn maintain_rod(&mut self, rod: RodKind) {
        self.rod_wear.remove(&rod);
    }

    pub fn record_catch(
        &mut self,
        def: &FishDef,
        weight_tenths: u32,
        quality: u32,
        price: u64,
        is_auto: bool,
        now: i64,
    ) -> bool {
        let is_record = self
            .codex
            .get(def.id)
            .map(|entry| weight_tenths > entry.best_weight_tenths)
            .unwrap_or(true);
        let entry = self
            .codex
            .entry(def.id.to_string())
            .or_insert(FishCodexEntry {
                first_at: now,
                count: 0,
                best_weight_tenths: 0,
                best_quality: 0,
                best_price: 0,
                manual_count: 0,
                manual_best_weight_tenths: 0,
                manual_best_quality: 0,
                manual_best_price: 0,
                auto_count: 0,
                auto_best_weight_tenths: 0,
                auto_best_quality: 0,
                auto_best_price: 0,
            });
        entry.count = entry.count.saturating_add(1);
        entry.best_weight_tenths = entry.best_weight_tenths.max(weight_tenths);
        entry.best_quality = entry.best_quality.max(quality);
        entry.best_price = entry.best_price.max(price);
        if is_auto {
            entry.auto_count = entry.auto_count.saturating_add(1);
            entry.auto_best_weight_tenths = entry.auto_best_weight_tenths.max(weight_tenths);
            entry.auto_best_quality = entry.auto_best_quality.max(quality);
            entry.auto_best_price = entry.auto_best_price.max(price);
        } else {
            entry.manual_count = entry.manual_count.saturating_add(1);
            entry.manual_best_weight_tenths = entry.manual_best_weight_tenths.max(weight_tenths);
            entry.manual_best_quality = entry.manual_best_quality.max(quality);
            entry.manual_best_price = entry.manual_best_price.max(price);
        }
        is_record
    }
}

pub fn spot_unlocked(level: u32, spot: FishingSpot) -> bool {
    level >= spot.unlock_level()
}

pub fn rod_unlocked(level: u32, rod: RodKind) -> bool {
    level >= rod.unlock_level()
}

/// 各钓点鱼王基础价。
pub fn king_base_price(spot: FishingSpot) -> u64 {
    match spot {
        FishingSpot::Creek => 400,
        FishingSpot::River => 800,
        FishingSpot::Jiang => 1_400,
        FishingSpot::Sea => 2_200,
    }
}

/// G1：金色变异按半条鱼王基础价收购。
pub fn golden_floor_price(spot: FishingSpot) -> u64 {
    king_base_price(spot) / 2
}

pub fn sell_price(price: u64, rod: RodKind) -> u64 {
    price * u64::from(100 + rod.sell_bonus_pct()) / 100
}

/// 一条鱼的经验按竿加成；自动钓鱼再乘 3/5。
pub fn rod_exp(base: u64, rod: RodKind) -> u64 {
    base * u64::from(100 + rod.exp_bonus_pct()) / 100
}

/// 金色变异在抛竿时就按会话种子 + 竿加成定好；结算/图鉴/时长同一个口径。
pub fn golden_session(id: u64, spot: FishingSpot, rod: RodKind) -> bool {
    let mut hash = 1_469_598_103_934_665_603u64 ^ (id ^ 0x60d1);
    for byte in b"golden" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^= u64::from(spot as u8);
    hash = hash.wrapping_mul(1_099_511_628_211);
    let roll = hash ^ (hash >> 33);
    roll % 1000 < 15 + rod.golden_bonus_permille()
}

/// 小游戏参数：框高、鱼速度、变向频率、时限。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingParams {
    pub frame_pct: u32,
    pub frame_speed_pct: u32,
    /// 磨损只损加速度，不损基础速度。
    pub frame_accel_pct: u32,
    pub fish_speed_pct: u32,
    pub fish_jerk_pct: u32,
    pub fish_escape_pct: u32,
    pub flee_radius_pct: u32,
    pub turn_pct: u32,
    pub target_min_ms: i64,
    pub target_max_ms: i64,
    pub personality: FishPersonality,
    pub personality_label: String,
    pub catch_ms: i64,
    pub progress_gain: u32,
    pub progress_loss: u32,
}

pub fn params(
    rod: RodKind,
    def: &FishDef,
    worn: bool,
    personality: FishPersonality,
) -> FishingParams {
    params_with_reel(rod, def, worn, personality, 0)
}

pub fn params_with_reel(
    rod: RodKind,
    def: &FishDef,
    worn: bool,
    personality: FishPersonality,
    reel_level: u32,
) -> FishingParams {
    let difficulty = def.difficulty.clamp(1, 5);
    let base_speed = 12 + difficulty * 2;
    let base_escape = match def.rarity {
        FishRarity::Common => 100,
        FishRarity::Uncommon => 106,
        FishRarity::Rare => 114,
        FishRarity::King => 118,
    };
    let (target_min_ms, target_max_ms) = personality.target_ms();
    // 鱼王在凶猛的基础上再快一档；逃逸已有稀有度 118% 加成。
    let king_boost = if def.rarity == FishRarity::King {
        110
    } else {
        100
    };
    FishingParams {
        frame_pct: rod.frame_pct(),
        frame_speed_pct: 100,
        frame_accel_pct: if worn { 62 } else { 100 },
        fish_speed_pct: base_speed * personality.speed_pct() / 100 * king_boost / 100,
        fish_jerk_pct: 40 + difficulty * 10 + personality.jerk_add(),
        fish_escape_pct: base_escape * personality.escape_pct() / 100,
        flee_radius_pct: personality.flee_radius_pct(),
        turn_pct: personality.turn_pct(),
        target_min_ms,
        target_max_ms,
        personality,
        personality_label: personality.label().into(),
        catch_ms: if def.rarity == FishRarity::King {
            FISHING_KING_CATCH_MS
        } else {
            11_000 + i64::from(difficulty) * 900
        },
        progress_gain: 22 + reel_bonus(reel_level),
        progress_loss: 12,
    }
}

/// 同一鱼种的累计批次：库存只存条数，价格/重量/品质按批次平均。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FishBatch {
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub total_weight_tenths: u64,
    #[serde(default)]
    pub total_quality: u64,
    #[serde(default)]
    pub total_value: u64,
}

impl FishBatch {
    pub fn add(&mut self, weight_tenths: u32, quality: u32, value: u64) {
        self.count = self.count.saturating_add(1);
        self.total_weight_tenths = self
            .total_weight_tenths
            .saturating_add(u64::from(weight_tenths));
        self.total_quality = self.total_quality.saturating_add(u64::from(quality));
        self.total_value = self.total_value.saturating_add(value);
    }

    pub fn unit_price(&self) -> u64 {
        if self.count == 0 {
            0
        } else {
            self.total_value / u64::from(self.count)
        }
    }

    pub fn avg_weight_tenths(&self) -> u32 {
        if self.count == 0 {
            0
        } else {
            (self.total_weight_tenths / u64::from(self.count)) as u32
        }
    }

    pub fn avg_quality(&self) -> u32 {
        if self.count == 0 {
            0
        } else {
            (self.total_quality / u64::from(self.count)) as u32
        }
    }

    /// 卖掉 qty 条，返回应得金币；批次按平均价值扣减。
    pub fn take_value(&mut self, qty: u32) -> u64 {
        if qty == 0 || self.count == 0 {
            return 0;
        }
        let qty = qty.min(self.count);
        let value = if qty == self.count {
            self.total_value
        } else {
            self.total_value * u64::from(qty) / u64::from(self.count)
        };
        self.count -= qty;
        self.total_value = self.total_value.saturating_sub(value);
        self.total_weight_tenths = self
            .total_weight_tenths
            .saturating_sub(u64::from(self.avg_weight_tenths()) * u64::from(qty));
        self.total_quality = self
            .total_quality
            .saturating_sub(u64::from(self.avg_quality()) * u64::from(qty));
        value
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FishCatchResult {
    #[serde(rename = "fishId")]
    pub fish_id: String,
    pub weight_tenths: u32,
    pub quality: u32,
    pub price: u64,
    pub market_pct: u32,
    pub at: i64,
    #[serde(default)]
    pub golden: bool,
    #[serde(default)]
    pub is_record: bool,
}

pub fn quality_label(quality: u32) -> &'static str {
    match quality {
        3 => "精品",
        2 => "优良",
        _ => "普通",
    }
}

/// 鱼竿影响品质概率：木 60/35/5，竹 45/40/15，玻璃钢 30/45/25，碳素 15/45/40。
pub fn quality_for(rod: RodKind, roll: u64) -> u32 {
    quality_for_with_bait(rod, BaitQuality::Normal, roll)
}

pub fn quality_for_with_bait(rod: RodKind, bait: BaitQuality, roll: u64) -> u32 {
    let p = ((roll % 100) as u32).saturating_sub(bait.quality_bonus());
    match rod {
        RodKind::Wood => {
            if p < 60 {
                1
            } else if p < 95 {
                2
            } else {
                3
            }
        }
        RodKind::Bamboo => {
            if p < 45 {
                1
            } else if p < 85 {
                2
            } else {
                3
            }
        }
        RodKind::Fiberglass => {
            if p < 30 {
                1
            } else if p < 75 {
                2
            } else {
                3
            }
        }
        RodKind::Carbon => {
            if p < 15 {
                1
            } else if p < 60 {
                2
            } else {
                3
            }
        }
    }
}

/// 重量（0.1kg 单位）：基础 0.8–1.2kg，鱼王 1.0–1.5kg；鱼竿和品质加成。
pub fn weight_tenths_for(rod: RodKind, def: &FishDef, roll: u64, quality: u32) -> u32 {
    weight_tenths_for_bait(rod, BaitQuality::Normal, def, roll, quality)
}

pub fn weight_tenths_for_bait(
    rod: RodKind,
    bait: BaitQuality,
    def: &FishDef,
    roll: u64,
    quality: u32,
) -> u32 {
    let base = if def.rarity == FishRarity::King {
        10 + (roll % 6) as u32
    } else {
        8 + (roll % 5) as u32
    };
    let rod_bonus = match rod {
        RodKind::Wood => 0,
        RodKind::Bamboo => 1,
        RodKind::Fiberglass => 2,
        RodKind::Carbon => 3,
    };
    (base + rod_bonus + bait.weight_bonus() + quality.saturating_sub(1)).min(20)
}

/// 市场浮动 90%–110%。
pub fn market_pct(roll: u64) -> u32 {
    90 + (roll % 21) as u32
}

pub fn price_for(def: &FishDef, weight_tenths: u32, quality: u32, market_pct: u32) -> u64 {
    let quality_factor = match quality {
        3 => 160u64,
        2 => 125,
        _ => 100,
    };
    let price = def
        .price
        .saturating_mul(u64::from(weight_tenths))
        .saturating_mul(quality_factor)
        .saturating_mul(u64::from(market_pct))
        / 100_000;
    price.max(1)
}
