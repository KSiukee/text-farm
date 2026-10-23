use serde::{Deserialize, Serialize};

pub const RANCH_PEN_COUNT: u8 = 3;
pub const RANCH_MAX_PENS: u8 = 5;
/// 扩建花费：3→4 栏 5,000；4→5 栏 20,000（第 5 栏是后期目标）。
pub const RANCH_EXPAND_COSTS: [u64; 2] = [5_000, 20_000];
/// 品质对售价的百分比：普通 / 优良 / 精品。
/// 不做得太陡：精品是长期目标，不是一次专用饲料就翻倍。
pub const RANCH_QUALITY_PCT: [u64; 3] = [100, 115, 130];

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
    #[serde(default)]
    pub bred: bool,
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
            bred: false,
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
        Self {
            kind,
            name: name.into(),
            adopted_at: now,
            satiety: 80,
            health: 100,
            sick: false,
            product_ready_at: now + kind.product_interval_ms(),
            last_tick_at: now,
            quality: 1,
            generation: 0,
            bred: false,
        }
    }

    pub fn tick(&mut self, now: i64, guarded: bool) {
        if now <= self.last_tick_at {
            return;
        }
        let hours = ((now - self.last_tick_at) / 3_600_000).min(24 * 30) as u32;
        self.last_tick_at = now;
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
        if !guarded && self.satiety < 20 && !self.sick {
            let roll = roll(now / 3_600_000, &self.name);
            if roll % 100 < 12 {
                self.sick = true;
            }
        }
    }

    pub fn feed(&mut self) {
        self.feed_with_care(0);
    }

    /// 喂食；`care_pct` 是「照料」专精的加成百分比（0/10/20/30/40）。
    pub fn feed_with_care(&mut self, care_pct: u32) {
        let bonus = 100 + care_pct;
        self.satiety = (self.satiety + 30 * bonus / 100).min(100);
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

    pub fn quality_label(&self) -> &'static str {
        match self.quality {
            1 => "普通",
            2 => "优良",
            _ => "精品",
        }
    }

    pub fn collect(&mut self, now: i64) -> bool {
        if now < self.product_ready_at {
            return false;
        }
        self.product_ready_at = now + self.kind.product_interval_ms();
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchPen {
    pub index: u8,
    pub animal: Option<RanchAnimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchState {
    pub pens: Vec<RanchPen>,
}

impl RanchState {
    pub fn new() -> Self {
        Self {
            pens: (1..=RANCH_PEN_COUNT)
                .map(|index| RanchPen {
                    index,
                    animal: None,
                })
                .collect(),
        }
    }

    pub fn ensure(&mut self) {
        if self.pens.len() < usize::from(RANCH_PEN_COUNT) {
            for index in (self.pens.len() as u8 + 1)..=RANCH_PEN_COUNT {
                self.pens.push(RanchPen {
                    index,
                    animal: None,
                });
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
        self.pens.push(RanchPen {
            index,
            animal: None,
        });
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
