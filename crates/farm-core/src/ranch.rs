use serde::{Deserialize, Serialize};

use crate::error::GameError;

pub const RANCH_PEN_COUNT: u8 = 3;
pub const RANCH_MAX_PENS: u8 = 5;
/// 扩建花费：3→4 栏 5,000；4→5 栏 20,000（第 5 栏是后期目标）。
pub const RANCH_EXPAND_COSTS: [u64; 2] = [5_000, 20_000];
/// 品质对售价的百分比：普通 / 优良 / 精品。
/// 不做得太陡：精品是长期目标，不是一次专用饲料就翻倍。
pub const RANCH_QUALITY_PCT: [u64; 3] = [100, 115, 130];
/// 栏舍卫生上限。
pub const RANCH_CLEAN_MAX: u32 = 100;
/// 收一次产物，栏舍卫生掉多少。
pub const RANCH_COLLECT_DIRT: u32 = 12;
/// 清理一次畜栏恢复多少卫生。
pub const RANCH_CLEAN_GAIN: u32 = 50;
/// 卫生低于这条线会掉品质概率、加生病概率。
pub const RANCH_DIRTY_LINE: u32 = 30;
/// 普通喂食一次加多少饱食。
pub const RANCH_FEED_SATIETY: u32 = 30;
/// 特殊产物概率（百分比），按最终品质取：普通/优良/精品。
pub const RANCH_SPECIAL_PCT: [u32; 3] = [0, 8, 25];
/// 繁殖冷却：72 小时。
pub const RANCH_BREED_COOLDOWN_MS: i64 = 72 * 60 * 60 * 1000;
/// 幼崽期：24 小时，成年后才能产出/繁殖。
pub const RANCH_YOUNG_MS: i64 = 24 * 60 * 60 * 1000;
/// 繁殖的健康门槛（低于这条线先养好）。
pub const RANCH_BREED_MIN_HEALTH: u32 = 60;

/// 生产特性：买来（或出生）时固定，只影响产出与饲养成本。
/// 牧场不碰“亲密度/心情/陪伴”这套语义——那是宠物的领域。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RanchTrait {
    /// 高产：产物间隔 −10%。
    HighYield,
    /// 省料：喂食时省下一份作物的概率 +12 个百分点。
    #[default]
    Efficient,
    /// 壮实：生病概率 −8 个百分点。
    Hardy,
    /// 娇气：生病率 +8 个百分点，高品质产物概率 +8 个百分点。
    Delicate,
    /// 双产：收产物额外 +1 的概率 +15 个百分点。
    TwinYield,
}

impl RanchTrait {
    pub const ALL: [RanchTrait; 5] = [
        RanchTrait::HighYield,
        RanchTrait::Efficient,
        RanchTrait::Hardy,
        RanchTrait::Delicate,
        RanchTrait::TwinYield,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RanchTrait::HighYield => "高产",
            RanchTrait::Efficient => "省料",
            RanchTrait::Hardy => "壮实",
            RanchTrait::Delicate => "娇气",
            RanchTrait::TwinYield => "双产",
        }
    }

    /// 一句话说明，给详情面板看。
    pub fn hint(self) -> &'static str {
        match self {
            RanchTrait::HighYield => "产物间隔 −10%",
            RanchTrait::Efficient => "喂食省料概率 +12%",
            RanchTrait::Hardy => "生病概率 −8%",
            RanchTrait::Delicate => "生病率 +8%，高品质概率 +8%",
            RanchTrait::TwinYield => "收产物额外 +1 概率 +15%",
        }
    }

    /// 高品质产物的额外概率（百分点）。
    pub fn quality_bonus(self) -> u32 {
        match self {
            RanchTrait::Delicate => 8,
            _ => 0,
        }
    }

    /// 生病概率修正（百分点，可为负）。
    pub fn sickness_delta(self) -> i32 {
        match self {
            RanchTrait::Hardy => -8,
            RanchTrait::Delicate => 8,
            _ => 0,
        }
    }

    /// 喂食省料的额外概率（百分点）。
    pub fn feed_save_bonus(self) -> u32 {
        match self {
            RanchTrait::Efficient => 12,
            _ => 0,
        }
    }

    /// 收产物额外 +1 的额外概率（百分点）。
    pub fn yield_bonus(self) -> u32 {
        match self {
            RanchTrait::TwinYield => 15,
            _ => 0,
        }
    }

    /// 产物间隔（高产 −10%）。
    pub fn product_interval(self, base: i64) -> i64 {
        match self {
            RanchTrait::HighYield => base * 9 / 10,
            _ => base,
        }
    }
}

/// 按最终品质给特殊产物概率（%）：普通 0、优良 8、精品 25。
pub fn special_chance(quality: u8) -> u32 {
    RANCH_SPECIAL_PCT[usize::from(quality.clamp(1, 3)) - 1]
}

/// 按种子挑一个生产特性：购买/出生时用，同一只动物结果固定。
pub fn roll_trait(seed: i64, salt: &str) -> RanchTrait {
    RanchTrait::ALL[(roll(seed, salt) % RanchTrait::ALL.len() as u64) as usize]
}

/// 一次繁殖的后代（预览与真繁殖共用）。
#[derive(Debug, Clone)]
pub struct RanchBreedPlan {
    pub kind: RanchAnimalKind,
    pub quality: u8,
    pub generation: u32,
    pub line_id: u32,
    pub temper: RanchTrait,
    pub inbred: bool,
    pub parents: String,
}

/// 由父母 + 繁殖序号确定下一次后代。
/// 用 `breedSeq` 当随机源：预览看到的就是下一发，生完序号 +1 才变。
pub fn breed_child(a: &RanchAnimal, b: &RanchAnimal, seq: u32) -> RanchBreedPlan {
    let seed = 0x9E37_79B9_7F4A_7C15u64
        ^ u64::from(seq)
        ^ (u64::from(a.line_id) << 17)
        ^ (u64::from(b.line_id) << 33)
        ^ (u64::from(a.generation) << 7)
        ^ u64::from(b.generation);
    let kind = if roll(seed as i64, "kind") % 2 == 0 {
        a.kind
    } else {
        b.kind
    };
    // 同一条血统线（且不是 0 号未登记线）算近亲：品质上限降到 2。
    let inbred = a.line_id != 0 && a.line_id == b.line_id;
    let base_quality = (a.quality + b.quality) / 2 + 1;
    let quality = if inbred {
        base_quality.min(2)
    } else {
        base_quality.min(3)
    };
    let generation = a.generation.max(b.generation) + 1;
    let line_id = if roll(seed as i64, "line") % 2 == 0 {
        a.line_id
    } else {
        b.line_id
    };
    // 10% 突变，否则父母各 50%。
    let temper = if roll(seed as i64, "mutate") % 100 < 10 {
        RanchTrait::ALL[(roll(seed as i64, "mutate_trait") % 5) as usize]
    } else if roll(seed as i64, "trait") % 2 == 0 {
        a.temper
    } else {
        b.temper
    };
    RanchBreedPlan {
        kind,
        quality,
        generation,
        line_id,
        temper,
        inbred,
        parents: format!(
            "{}（第{}代） × {}（第{}代）",
            a.name,
            a.generation + 1,
            b.name,
            b.generation + 1
        ),
    }
}

/// 繁殖条件检查项；消息给预览看，`error()` 给真动作返回。
#[derive(Debug, Clone)]
pub enum BreedBlocker {
    SamePen,
    Sick { pen: u8, name: String },
    Young { pen: u8, name: String, remaining_ms: i64 },
    Cooldown { pen: u8, name: String, remaining_ms: i64 },
    LowHealth { pen: u8, name: String, health: u32 },
    DirtyPen { pen: u8, cleanliness: u32 },
    NoEmptyPen,
    NoFeed,
}

impl BreedBlocker {
    pub fn message(&self) -> String {
        match self {
            BreedBlocker::SamePen => "不能选同一只动物".into(),
            BreedBlocker::Sick { pen, name } => {
                format!("{pen} 号畜栏的{name}生病了，先治疗")
            }
            BreedBlocker::Young {
                pen,
                name,
                remaining_ms,
            } => format!(
                "{pen} 号畜栏的{name}还没成年（还要 {}）",
                remaining_text(*remaining_ms)
            ),
            BreedBlocker::Cooldown {
                pen,
                name,
                remaining_ms,
            } => format!(
                "{pen} 号畜栏的{name}还在繁殖冷却（还要 {}）",
                remaining_text(*remaining_ms)
            ),
            BreedBlocker::LowHealth {
                pen,
                name,
                health,
            } => format!("{pen} 号畜栏的{name}健康只有 {health}，先养到 {RANCH_BREED_MIN_HEALTH}"),
            BreedBlocker::DirtyPen { pen, cleanliness } => {
                format!("{pen} 号畜栏卫生只有 {cleanliness}，先清理")
            }
            BreedBlocker::NoEmptyPen => "没有空畜栏放幼崽".into(),
            BreedBlocker::NoFeed => "没有专用饲料（繁殖要 1 份）".into(),
        }
    }

    pub fn error(&self) -> GameError {
        match self {
            BreedBlocker::SamePen => GameError::RanchBreedSamePen,
            BreedBlocker::Sick { .. } => GameError::RanchAnimalSick,
            BreedBlocker::Young { .. } => GameError::RanchAnimalYoung,
            BreedBlocker::Cooldown { .. } => GameError::RanchBreedCooldown,
            BreedBlocker::LowHealth { .. } => GameError::RanchBreedUnhealthy,
            BreedBlocker::DirtyPen { .. } => GameError::RanchPenDirty,
            BreedBlocker::NoEmptyPen => GameError::RanchNoEmptyPen,
            BreedBlocker::NoFeed => GameError::RanchNoFeed,
        }
    }
}

/// 把毫秒说成人话（向上取整到分钟）。
pub fn remaining_text(ms: i64) -> String {
    let minutes = (ms.max(0) + 59_999) / 60_000;
    if minutes >= 60 {
        format!("{} 小时 {} 分", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} 分钟")
    }
}

pub fn quality_name(quality: u8) -> &'static str {
    match quality {
        2 => "优良",
        3 => "精品",
        _ => "普通",
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RanchAnimalKind {
    Chicken,
    Duck,
    Cow,
    Sheep,
}

impl RanchAnimalKind {
    pub fn label(self) -> &'static str {
        match self {
            RanchAnimalKind::Chicken => "母鸡",
            RanchAnimalKind::Duck => "鸭子",
            RanchAnimalKind::Cow => "奶牛",
            RanchAnimalKind::Sheep => "绵羊",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            RanchAnimalKind::Chicken => "🐔",
            RanchAnimalKind::Duck => "🦆",
            RanchAnimalKind::Cow => "🐄",
            RanchAnimalKind::Sheep => "🐑",
        }
    }

    pub fn cost(self) -> u64 {
        match self {
            RanchAnimalKind::Chicken => 800,
            RanchAnimalKind::Duck => 1_200,
            RanchAnimalKind::Cow => 5_000,
            RanchAnimalKind::Sheep => 3_500,
        }
    }

    pub fn product_key(self) -> &'static str {
        match self {
            RanchAnimalKind::Chicken => "ranch:egg",
            RanchAnimalKind::Duck => "ranch:duck_egg",
            RanchAnimalKind::Cow => "ranch:milk",
            RanchAnimalKind::Sheep => "ranch:wool",
        }
    }

    pub fn product_name(self) -> &'static str {
        match self {
            RanchAnimalKind::Chicken => "鸡蛋",
            RanchAnimalKind::Duck => "鸭蛋",
            RanchAnimalKind::Cow => "牛奶",
            RanchAnimalKind::Sheep => "羊毛",
        }
    }

    pub fn product_interval_ms(self) -> i64 {
        match self {
            RanchAnimalKind::Chicken => 8 * 60 * 60 * 1000,
            RanchAnimalKind::Duck => 10 * 60 * 60 * 1000,
            RanchAnimalKind::Cow => 16 * 60 * 60 * 1000,
            RanchAnimalKind::Sheep => 20 * 60 * 60 * 1000,
        }
    }

    /// 亲密度够高时才会出的特殊产物。
    pub fn special_key(self) -> &'static str {
        match self {
            RanchAnimalKind::Chicken => "ranch:golden_egg",
            RanchAnimalKind::Duck => "ranch:big_duck_egg",
            RanchAnimalKind::Cow => "ranch:cream",
            RanchAnimalKind::Sheep => "ranch:fine_wool",
        }
    }

    pub fn special_name(self) -> &'static str {
        match self {
            RanchAnimalKind::Chicken => "金蛋",
            RanchAnimalKind::Duck => "大鸭蛋",
            RanchAnimalKind::Cow => "奶油",
            RanchAnimalKind::Sheep => "细羊毛",
        }
    }
}

/// 牧场物品目录：原料与加工品共用 `ranch:` 前缀。
/// 品质不单独存字段，而是写在 key 后缀里（`.fine` / `.prime`），
/// 这样背包、卖出、旧档全部按一份整数库存走，不用改库存结构。
#[derive(Debug, Clone, Copy)]
pub struct RanchItemDef {
    pub key: &'static str,
    pub name: &'static str,
    pub base_price: u64,
    pub sellable: bool,
}

pub const RANCH_ITEMS: &[RanchItemDef] = &[
    RanchItemDef {
        key: "ranch:egg",
        name: "鸡蛋",
        base_price: 45,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:duck_egg",
        name: "鸭蛋",
        base_price: 75,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:milk",
        name: "牛奶",
        base_price: 220,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:wool",
        name: "羊毛",
        base_price: 150,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:golden_egg",
        name: "金蛋",
        base_price: 260,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:big_duck_egg",
        name: "大鸭蛋",
        base_price: 320,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:cream",
        name: "奶油",
        base_price: 560,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:fine_wool",
        name: "细羊毛",
        base_price: 480,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:cheese",
        name: "奶酪",
        base_price: 840,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:butter",
        name: "黄油",
        base_price: 520,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:cake",
        name: "蛋糕",
        base_price: 910,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:yarn",
        name: "毛线",
        base_price: 900,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:salted_egg",
        name: "咸鸭蛋",
        base_price: 450,
        sellable: true,
    },
    RanchItemDef {
        key: "ranch:feed",
        name: "专用饲料",
        base_price: 0,
        sellable: false,
    },
];

/// 拆品质后缀：`ranch:egg.prime` → (`ranch:egg`, 3)。
pub fn split_quality(key: &str) -> (&str, u8) {
    if let Some(base) = key.strip_suffix(".prime") {
        (base, 3)
    } else if let Some(base) = key.strip_suffix(".fine") {
        (base, 2)
    } else {
        (key, 1)
    }
}

pub fn with_quality(key: &str, quality: u8) -> String {
    match quality {
        2 => format!("{key}.fine"),
        3 => format!("{key}.prime"),
        _ => key.to_string(),
    }
}

pub fn item_def(key: &str) -> Option<&'static RanchItemDef> {
    let (base, _) = split_quality(key);
    RANCH_ITEMS.iter().find(|item| item.key == base)
}

/// 单件售价（已乘品质系数）；饲料等不可卖物品返回 None。
pub fn price_of(key: &str) -> Option<u64> {
    let def = item_def(key)?;
    if !def.sellable {
        return None;
    }
    let (_, quality) = split_quality(key);
    let pct = RANCH_QUALITY_PCT[usize::from(quality.clamp(1, 3)) - 1];
    Some(def.base_price * pct / 100)
}

pub fn display_name(key: &str) -> Option<String> {
    let def = item_def(key)?;
    let (_, quality) = split_quality(key);
    Some(if quality > 1 {
        format!("{}{}", quality_name(quality), def.name)
    } else {
        def.name.to_string()
    })
}

pub fn is_sellable(key: &str) -> bool {
    item_def(key).is_some_and(|def| def.sellable)
}

/// 产物加工配方：只吃牧场原料，按时间换加工品。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RanchRecipe {
    Cheese,
    Butter,
    Cake,
    Yarn,
    SaltedDuckEgg,
}

impl RanchRecipe {
    pub const ALL: [RanchRecipe; 5] = [
        RanchRecipe::Cheese,
        RanchRecipe::Butter,
        RanchRecipe::Cake,
        RanchRecipe::Yarn,
        RanchRecipe::SaltedDuckEgg,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RanchRecipe::Cheese => "奶酪",
            RanchRecipe::Butter => "黄油",
            RanchRecipe::Cake => "蛋糕",
            RanchRecipe::Yarn => "毛线",
            RanchRecipe::SaltedDuckEgg => "咸鸭蛋",
        }
    }

    pub fn inputs(self) -> &'static [(&'static str, u32)] {
        match self {
            RanchRecipe::Cheese => &[("ranch:milk", 2)],
            RanchRecipe::Butter => &[("ranch:milk", 1)],
            RanchRecipe::Cake => &[("ranch:egg", 2), ("ranch:milk", 1)],
            RanchRecipe::Yarn => &[("ranch:wool", 2)],
            RanchRecipe::SaltedDuckEgg => &[("ranch:duck_egg", 2)],
        }
    }

    pub fn duration_ms(self) -> i64 {
        match self {
            RanchRecipe::Cheese => 4 * 60 * 60 * 1000,
            RanchRecipe::Butter => 3 * 60 * 60 * 1000,
            RanchRecipe::Cake => 6 * 60 * 60 * 1000,
            RanchRecipe::Yarn => 6 * 60 * 60 * 1000,
            RanchRecipe::SaltedDuckEgg => 3 * 60 * 60 * 1000,
        }
    }

    pub fn output_key(self) -> &'static str {
        match self {
            RanchRecipe::Cheese => "ranch:cheese",
            RanchRecipe::Butter => "ranch:butter",
            RanchRecipe::Cake => "ranch:cake",
            RanchRecipe::Yarn => "ranch:yarn",
            RanchRecipe::SaltedDuckEgg => "ranch:salted_egg",
        }
    }

    pub fn output_name(self) -> &'static str {
        item_def(self.output_key())
            .map(|def| def.name)
            .unwrap_or("加工品")
    }

    pub fn output_price(self) -> u64 {
        item_def(self.output_key())
            .map(|def| def.base_price)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchAnimal {
    pub kind: RanchAnimalKind,
    pub name: String,
    pub adopted_at: i64,
    pub satiety: u32,
    pub health: u32,
    pub sick: bool,
    pub product_ready_at: i64,
    pub last_tick_at: i64,
    #[serde(default = "default_quality")]
    pub quality: u8,
    #[serde(default)]
    pub generation: u32,
    /// 血统线；同一线的两只动物繁殖算近亲。
    #[serde(default)]
    pub line_id: u32,
    /// 上次繁殖时间；72 小时内不能再繁殖。
    #[serde(default)]
    pub last_breed_at: i64,
    /// 成年时间；买来的直接成年，出生的幼崽 +24h。
    #[serde(default)]
    pub adult_at: i64,
    /// 近亲后代：生病率 +8%。
    #[serde(default)]
    pub inbred: bool,
    /// 亲本记录：`名字（第N代） × 名字（第N代）`。
    #[serde(default)]
    pub parents: String,
    /// 固定生产特性；JSON 里叫 `trait`，Rust 里避开关键字。
    #[serde(rename = "trait")]
    pub temper: RanchTrait,
}

impl Default for RanchAnimal {
    fn default() -> Self {
        Self {
            kind: RanchAnimalKind::Chicken,
            name: "小母鸡".into(),
            adopted_at: 0,
            satiety: 80,
            health: 100,
            sick: false,
            product_ready_at: 0,
            last_tick_at: 0,
            quality: 1,
            generation: 0,
            line_id: 0,
            last_breed_at: 0,
            adult_at: 0,
            inbred: false,
            parents: String::new(),
            temper: RanchTrait::Efficient,
        }
    }
}

fn default_quality() -> u8 {
    1
}

impl RanchAnimal {
    pub fn new(kind: RanchAnimalKind, now: i64) -> Self {
        let name = match kind {
            RanchAnimalKind::Chicken => "小母鸡",
            RanchAnimalKind::Duck => "小鸭",
            RanchAnimalKind::Cow => "小奶牛",
            RanchAnimalKind::Sheep => "小绵羊",
        };
        let mut animal = Self {
            kind,
            name: name.into(),
            adopted_at: now,
            satiety: 80,
            health: 100,
            sick: false,
            product_ready_at: 0,
            last_tick_at: now,
            quality: 1,
            generation: 0,
            line_id: 0,
            last_breed_at: 0,
            adult_at: now,
            inbred: false,
            parents: String::new(),
            temper: roll_trait(now, kind.label()),
        };
        animal.product_ready_at = now + animal.product_interval();
        animal
    }

    /// 幼崽期：不能产出、不能繁殖。
    pub fn is_young(&self, now: i64) -> bool {
        now < self.adult_at
    }

    pub fn young_remaining(&self, now: i64) -> i64 {
        (self.adult_at - now).max(0)
    }

    /// 繁殖冷却是否结束（`lastBreedAt = 0` 表示从没繁殖过）。
    pub fn breed_ready(&self, now: i64) -> bool {
        self.last_breed_at <= 0 || now - self.last_breed_at >= RANCH_BREED_COOLDOWN_MS
    }

    pub fn breed_remaining(&self, now: i64) -> i64 {
        if self.last_breed_at <= 0 {
            0
        } else {
            (self.last_breed_at + RANCH_BREED_COOLDOWN_MS - now).max(0)
        }
    }

    /// 个体产物间隔：受「高产」修正。
    pub fn product_interval(&self) -> i64 {
        self.temper.product_interval(self.kind.product_interval_ms())
    }

    /// `pen_clean` 是这间畜栏的卫生：脏栏会加生病概率、掉品质。
    pub fn tick(&mut self, now: i64, guarded: bool, pen_clean: u32) {
        if now <= self.last_tick_at {
            return;
        }
        let elapsed = now - self.last_tick_at;
        let hours = (elapsed / 3_600_000).min(24 * 30) as u32;
        if hours == 0 {
            // 壳每 30 秒就 sync 一次；不满一小时先攒着，别把零头抹掉。
            return;
        }
        if elapsed / 3_600_000 >= 24 * 30 {
            self.last_tick_at = now;
        } else {
            self.last_tick_at += i64::from(hours) * 3_600_000;
        }
        self.satiety = self.satiety.saturating_sub(hours);
        if self.satiety == 0 {
            self.health = self.health.saturating_sub(hours / 3);
            if !self.sick && self.health < 50 {
                self.sick = true;
            }
        }
        if self.sick {
            self.health = self.health.saturating_sub(hours / 6);
        } else if self.satiety > 30 && self.health < 100 {
            self.health = (self.health + hours / 2).min(100);
        }
        if !guarded && !self.sick {
            let dirty = pen_clean < RANCH_DIRTY_LINE;
            let hungry = self.satiety < 20;
            if dirty || hungry {
                let base = 12
                    + self.temper.sickness_delta()
                    + if dirty { 6 } else { 0 }
                    + if self.inbred { 8 } else { 0 };
                let chance = u32::try_from(base.clamp(0, 100)).unwrap_or(0);
                let roll = roll(now / 3_600_000, &self.name);
                if roll % 100 < u64::from(chance) {
                    self.sick = true;
                }
            }
        }
    }

    pub fn feed(&mut self) {
        self.feed_with_care(0);
    }

    /// 喂食；`care_pct` 是「照料」专精的加成百分比（0/10/20/30/40）。
    pub fn feed_with_care(&mut self, care_pct: u32) {
        let bonus = 100 + care_pct;
        self.satiety = (self.satiety + RANCH_FEED_SATIETY * bonus / 100).min(100);
        self.health = (self.health + 5 * bonus / 100).min(100);
    }

    /// 治疗；`care_pct` 同上。
    pub fn heal_with_care(&mut self, care_pct: u32) {
        self.sick = false;
        self.health = (self.health + 20 * (100 + care_pct) / 100).min(100);
    }

    pub fn feed_premium(&mut self) {
        self.satiety = (self.satiety + 40).min(100);
        self.health = (self.health + 10).min(100);
        self.quality = (self.quality + 1).min(3);
    }

    /// 高品质产物的基础概率（百分点）：栏舍卫生 + 生产特性；专精/加成由引擎另加。
    pub fn quality_chance(&self, pen_clean: u32) -> u32 {
        let mut chance = 8 + pen_clean / 4;
        chance += self.temper.quality_bonus();
        if pen_clean < RANCH_DIRTY_LINE {
            chance = chance.saturating_sub(5);
        }
        chance
    }

    pub fn quality_label(&self) -> &'static str {
        match self.quality {
            1 => "普通",
            2 => "优良",
            _ => "精品",
        }
    }

    pub fn collect(&mut self, now: i64) -> bool {
        if now < self.adult_at || now < self.product_ready_at {
            return false;
        }
        self.product_ready_at = now + self.product_interval();
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchPen {
    pub index: u8,
    pub animal: Option<RanchAnimal>,
    /// 栏舍卫生 0–100：收产物会掉，太低掉品质、加生病。
    pub cleanliness: u32,
    /// 今天是否已经清理过；空字符串表示没记过。
    pub last_clean_day: String,
}

impl Default for RanchPen {
    fn default() -> Self {
        Self {
            index: 0,
            animal: None,
            cleanliness: RANCH_CLEAN_MAX,
            last_clean_day: String::new(),
        }
    }
}

impl RanchPen {
    pub fn new(index: u8) -> Self {
        Self {
            index,
            ..Self::default()
        }
    }

    /// 清理：卫生 +50，返回实际恢复量。
    pub fn clean(&mut self) -> u32 {
        let before = self.cleanliness;
        self.cleanliness = (self.cleanliness + RANCH_CLEAN_GAIN).min(RANCH_CLEAN_MAX);
        self.cleanliness - before
    }

    /// 收一次产物弄脏栏舍。
    pub fn get_dirty(&mut self) {
        self.cleanliness = self.cleanliness.saturating_sub(RANCH_COLLECT_DIRT);
    }

    pub fn can_clean(&self, today: &str) -> bool {
        today.is_empty() || self.last_clean_day != today
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchState {
    pub pens: Vec<RanchPen>,
    /// 繁殖序号：预览与真繁殖共用，保证“预览什么就生什么”。
    #[serde(default)]
    pub breed_seq: u32,
    /// 血统线序号：每只买来的动物开一条新线。
    #[serde(default)]
    pub line_seq: u32,
}

impl RanchState {
    pub fn new() -> Self {
        Self {
            pens: (1..=RANCH_PEN_COUNT).map(RanchPen::new).collect(),
            breed_seq: 0,
            line_seq: 0,
        }
    }

    pub fn ensure(&mut self) {
        if self.pens.len() < usize::from(RANCH_PEN_COUNT) {
            for index in (self.pens.len() as u8 + 1)..=RANCH_PEN_COUNT {
                self.pens.push(RanchPen::new(index));
            }
        }
    }

    pub fn pen(&self, index: u8) -> Option<&RanchPen> {
        self.pens.iter().find(|pen| pen.index == index)
    }

    pub fn pen_mut(&mut self, index: u8) -> Option<&mut RanchPen> {
        self.pens.iter_mut().find(|pen| pen.index == index)
    }

    pub fn has_animal(&self) -> bool {
        self.pens.iter().any(|pen| pen.animal.is_some())
    }

    pub fn can_expand(&self) -> bool {
        self.pens.len() < usize::from(RANCH_MAX_PENS)
    }

    /// 下一次扩建的价格：3→4 栏 5,000；4→5 栏 20,000。
    pub fn next_expand_cost(&self) -> u64 {
        let step = self.pens.len().saturating_sub(usize::from(RANCH_PEN_COUNT));
        RANCH_EXPAND_COSTS
            .get(step)
            .copied()
            .unwrap_or(*RANCH_EXPAND_COSTS.last().unwrap_or(&0))
    }

    pub fn expand(&mut self) -> bool {
        if !self.can_expand() {
            return false;
        }
        let index = self.pens.len() as u8 + 1;
        self.pens.push(RanchPen::new(index));
        true
    }
}

fn roll(now: i64, name: &str) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ (now as u64);
    for byte in name.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^ (hash >> 33)
}
