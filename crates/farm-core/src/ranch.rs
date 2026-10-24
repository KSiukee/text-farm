use serde::{Deserialize, Serialize};

pub const RANCH_PEN_COUNT: u8 = 3;
pub const RANCH_MAX_PENS: u8 = 5;
/// 扩建花费：3→4 栏 5,000；4→5 栏 20,000（第 5 栏是后期目标）。
pub const RANCH_EXPAND_COSTS: [u64; 2] = [5_000, 20_000];
/// 品质对售价的百分比：普通 / 优良 / 精品。
/// 不做得太陡：精品是长期目标，不是一次专用饲料就翻倍。
pub const RANCH_QUALITY_PCT: [u64; 3] = [100, 115, 130];
/// 亲密度的上限与“能出特殊产物”的门槛。
pub const RANCH_AFFECTION_MAX: u32 = 100;
/// 清洁度上限。
pub const RANCH_CLEAN_MAX: u32 = 100;
/// 亲密度到这条线，收产物才有机会出特殊产物。
pub const RANCH_SPECIAL_AFFECTION: u32 = 80;
/// 达到门槛后，每次收产物出特殊产物的百分比。
pub const RANCH_SPECIAL_PCT: u32 = 15;
/// 普通喂食一次加多少饱食；挑食动物打八折。
pub const RANCH_FEED_SATIETY: u32 = 30;
/// 梳毛 / 放牧 / 互动各加多少（亲密度由性格再修正）。
pub const RANCH_GROOM_CLEAN: u32 = 40;
pub const RANCH_GROOM_AFFECTION: u32 = 2;
pub const RANCH_GRAZE_SATIETY: u32 = 15;
pub const RANCH_GRAZE_AFFECTION: u32 = 3;
pub const RANCH_PET_AFFECTION: u32 = 5;
/// 清洁度每小时掉多少（离线也照掉）。
pub const RANCH_CLEAN_DECAY_PER_HOUR: u32 = 1;
/// 清洁度低于这条线会掉品质概率、加生病概率。
pub const RANCH_DIRTY_LINE: u32 = 30;

/// 动物性格：买来（或出生）时固定，不能改。
/// 每种只改一件事，方便玩家一眼比较；不做“纯负面”性格。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RanchTrait {
    /// 高产：产物间隔 −10%。
    HighYield,
    /// 温顺：互动涨亲密度 +50%。
    #[default]
    Gentle,
    /// 挑食：普通喂食饱食 −20%，专用饲料效果更强。
    Picky,
    /// 娇气：生病率 +，高品质产物概率 +。
    Delicate,
    /// 恋家：牧场有宠物陪伴时，健康恢复更快、品质概率 +。
    Homebody,
}

impl RanchTrait {
    pub const ALL: [RanchTrait; 5] = [
        RanchTrait::HighYield,
        RanchTrait::Gentle,
        RanchTrait::Picky,
        RanchTrait::Delicate,
        RanchTrait::Homebody,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RanchTrait::HighYield => "高产",
            RanchTrait::Gentle => "温顺",
            RanchTrait::Picky => "挑食",
            RanchTrait::Delicate => "娇气",
            RanchTrait::Homebody => "恋家",
        }
    }

    /// 一句话说明，给详情面板看。
    pub fn hint(self) -> &'static str {
        match self {
            RanchTrait::HighYield => "产物间隔 −10%",
            RanchTrait::Gentle => "梳毛/放牧/互动涨亲密度 +50%",
            RanchTrait::Picky => "普通喂食饱食 −20%，专用饲料效果更强",
            RanchTrait::Delicate => "生病率略高，但更容易出高品质产物",
            RanchTrait::Homebody => "牧场有宠物陪伴时恢复更快、品质概率 +5%",
        }
    }

    /// 高品质产物的额外概率（百分点）。
    pub fn quality_bonus(self) -> u32 {
        match self {
            RanchTrait::Delicate => 8,
            RanchTrait::Homebody => 5,
            _ => 0,
        }
    }

    /// 生病概率的额外百分点。
    pub fn sickness_bonus(self) -> u32 {
        match self {
            RanchTrait::Delicate => 8,
            _ => 0,
        }
    }

    /// 亲密度收益（温顺 +50%，向下取整）。
    pub fn affection_gain(self, base: u32) -> u32 {
        match self {
            RanchTrait::Gentle => base + base / 2,
            _ => base,
        }
    }

    /// 普通喂食的饱食收益（挑食 −20%）。
    pub fn feed_satiety(self, base: u32) -> u32 {
        match self {
            RanchTrait::Picky => base * 8 / 10,
            _ => base,
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

/// 按种子挑一个性格：购买/出生时用，同一只动物结果固定。
pub fn roll_trait(seed: i64, salt: &str) -> RanchTrait {
    RanchTrait::ALL[(roll(seed, salt) % RanchTrait::ALL.len() as u64) as usize]
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
    #[serde(default)]
    pub bred: bool,
    /// 固定性格；JSON 里叫 `trait`，Rust 里避开关键字。
    #[serde(rename = "trait")]
    pub temper: RanchTrait,
    /// 亲密度 0–100：影响高品质与特殊产物。
    pub affection: u32,
    /// 清洁度 0–100：太低会掉品质、加生病。
    pub cleanliness: u32,
    /// 今天是否已经梳毛/放牧/互动过；空字符串表示没记过。
    #[serde(default)]
    pub last_groom_day: String,
    #[serde(default)]
    pub last_graze_day: String,
    #[serde(default)]
    pub last_pet_day: String,
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
            temper: RanchTrait::Gentle,
            affection: 30,
            cleanliness: RANCH_CLEAN_MAX,
            last_groom_day: String::new(),
            last_graze_day: String::new(),
            last_pet_day: String::new(),
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
            bred: false,
            temper: roll_trait(now, kind.label()),
            affection: 30,
            cleanliness: RANCH_CLEAN_MAX,
            last_groom_day: String::new(),
            last_graze_day: String::new(),
            last_pet_day: String::new(),
        };
        animal.product_ready_at = now + animal.product_interval();
        animal
    }

    /// 个体产物间隔：受「高产」修正。
    pub fn product_interval(&self) -> i64 {
        self.temper.product_interval(self.kind.product_interval_ms())
    }

    pub fn tick(&mut self, now: i64, guarded: bool) {
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
        self.cleanliness = self
            .cleanliness
            .saturating_sub(hours.saturating_mul(RANCH_CLEAN_DECAY_PER_HOUR));
        if self.satiety == 0 {
            self.health = self.health.saturating_sub(hours / 3);
            if !self.sick && self.health < 50 {
                self.sick = true;
            }
        }
        if self.sick {
            self.health = self.health.saturating_sub(hours / 6);
        } else if self.satiety > 30 && self.health < 100 {
            // 恋家 + 有宠物陪着，恢复更快。
            let extra = if guarded && self.temper == RanchTrait::Homebody {
                hours / 2
            } else {
                0
            };
            self.health = (self.health + hours / 2 + extra).min(100);
        }
        if !guarded && !self.sick {
            let dirty = self.cleanliness < RANCH_DIRTY_LINE;
            let hungry = self.satiety < 20;
            if dirty || hungry {
                let chance = 12
                    + self.temper.sickness_bonus()
                    + if dirty { 6 } else { 0 };
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
        let base = self.temper.feed_satiety(RANCH_FEED_SATIETY);
        self.satiety = (self.satiety + base * bonus / 100).min(100);
        self.health = (self.health + 5 * bonus / 100).min(100);
        self.affection = (self.affection + 1).min(RANCH_AFFECTION_MAX);
    }

    /// 治疗；`care_pct` 同上。
    pub fn heal_with_care(&mut self, care_pct: u32) {
        self.sick = false;
        self.health = (self.health + 20 * (100 + care_pct) / 100).min(100);
    }

    pub fn feed_premium(&mut self) {
        // 挑食的动物偏爱专用饲料：饱食与亲密度都更高。
        let (satiety, affection) = if self.temper == RanchTrait::Picky {
            (50, 8)
        } else {
            (40, 5)
        };
        self.satiety = (self.satiety + satiety).min(100);
        self.health = (self.health + 10).min(100);
        self.quality = (self.quality + 1).min(3);
        self.affection = (self.affection + affection).min(RANCH_AFFECTION_MAX);
    }

    /// 梳毛：清洁 +40，亲密 +2（温顺 ×1.5）。返回实际亲密收益。
    pub fn groom(&mut self) -> u32 {
        self.cleanliness = (self.cleanliness + RANCH_GROOM_CLEAN).min(RANCH_CLEAN_MAX);
        let gain = self.temper.affection_gain(RANCH_GROOM_AFFECTION);
        self.affection = (self.affection + gain).min(RANCH_AFFECTION_MAX);
        gain
    }

    /// 放牧：饱食 +15、亲密 +3（温顺 ×1.5），清洁 −10；冬天收益减半。
    /// 返回（实际饱食收益、实际亲密收益）。
    pub fn graze(&mut self, winter: bool) -> (u32, u32) {
        let base_satiety = if winter {
            RANCH_GRAZE_SATIETY / 2
        } else {
            RANCH_GRAZE_SATIETY
        };
        let before = self.satiety;
        self.satiety = (self.satiety + base_satiety).min(100);
        self.cleanliness = self.cleanliness.saturating_sub(10);
        let base_affection = if winter {
            RANCH_GRAZE_AFFECTION.saturating_sub(1)
        } else {
            RANCH_GRAZE_AFFECTION
        };
        let gain = self.temper.affection_gain(base_affection);
        self.affection = (self.affection + gain).min(RANCH_AFFECTION_MAX);
        (self.satiety - before, gain)
    }

    /// 互动：亲密 +5（温顺 ×1.5）。返回实际亲密收益。
    pub fn pet_interact(&mut self) -> u32 {
        let gain = self.temper.affection_gain(RANCH_PET_AFFECTION);
        self.affection = (self.affection + gain).min(RANCH_AFFECTION_MAX);
        gain
    }

    /// 高品质产物的基础概率（百分点）：照料 + 性格；专精/加成由引擎另加。
    pub fn care_quality_chance(&self, guarded: bool) -> u32 {
        let mut chance = 8 + self.affection / 5 + self.cleanliness / 10;
        if self.temper == RanchTrait::Delicate {
            chance += self.temper.quality_bonus();
        }
        if self.temper == RanchTrait::Homebody && guarded {
            chance += 5;
        }
        if self.cleanliness < RANCH_DIRTY_LINE {
            chance = chance.saturating_sub(5);
        }
        chance
    }

    /// 亲密度够高时，这次收产物是否出特殊产物。
    pub fn special_ready(&self, now: i64, pen: u8) -> bool {
        self.affection >= RANCH_SPECIAL_AFFECTION
            && self.cleanliness >= RANCH_DIRTY_LINE
            && roll(now.max(0) ^ 0x5eed, &format!("{pen}:{}", self.name)) % 100
                < u64::from(RANCH_SPECIAL_PCT)
    }

    pub fn quality_label(&self) -> &'static str {
        match self.quality {
            1 => "普通",
            2 => "优良",
            _ => "精品",
        }
    }

    /// 一眼能看懂的状态，给牧场卡片看。
    pub fn mood_label(&self) -> &'static str {
        if self.sick {
            "不舒服"
        } else if self.satiety < 30 {
            "饿了"
        } else if self.cleanliness < RANCH_DIRTY_LINE {
            "有点脏"
        } else if self.affection >= RANCH_SPECIAL_AFFECTION {
            "很亲近你"
        } else if self.affection >= 50 {
            "认得你"
        } else {
            "还不太熟"
        }
    }

    pub fn collect(&mut self, now: i64) -> bool {
        if now < self.product_ready_at {
            return false;
        }
        self.product_ready_at = now + self.product_interval();
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
