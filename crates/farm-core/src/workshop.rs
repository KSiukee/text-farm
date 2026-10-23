use serde::{Deserialize, Serialize};

use crate::fishing::BaitRecipe;
use crate::ranch::RanchRecipe;

pub const WORKSHOP_BASE_SLOTS: u8 = 3;
pub const WORKSHOP_MAX_SLOTS: u8 = 5;
pub const WORKSHOP_COST: u64 = 6_000;
pub const WORKSHOP_LEVEL: u32 = 8;
/// 扩槽费用/等级：3→4 槽、4→5 槽。
pub const WORKSHOP_SLOT_COSTS: [u64; 2] = [30_000, 80_000];
pub const WORKSHOP_SLOT_LEVELS: [u32; 2] = [18, 24];

/// 制造屋调平目标（#125）：每个加工槽每小时净赚 100 金币。
/// 手工台/设备配方按「原料价值 + 工时边距」定价，礼盒按原料价值的 20% 封顶。
pub const WORKSHOP_PROFIT_PER_HOUR: u64 = 100;

/// 加工一份要的工时边距；100 金币/小时 → 每 36,000 毫秒 1 金币。
pub const fn workshop_margin(duration_ms: i64) -> u64 {
    if duration_ms <= 0 {
        0
    } else {
        (duration_ms as u64) / 36_000
    }
}

/// 原料按市场价计入后，加一份固定工时边距。
pub const fn workshop_price(input_value: u64, duration_ms: i64) -> u64 {
    input_value.saturating_add(workshop_margin(duration_ms))
}

/// 作物礼盒：5 份同种作物 + 3 小时 → 1 个礼盒。
/// 售价 = 5 份原料价值 + 20% 原料价值，边距限制在 60–300（20–100 金币/小时）。
pub const CROP_BOX_INPUT: u32 = 5;
pub const CROP_BOX_INTERVAL_MS: i64 = 3 * 60 * 60 * 1000;
pub const CROP_BOX_MIN_MARGIN: u64 = 60;

/// 制造屋鱼加工配方：主料是「任意未锁定鱼」，按鱼价从低到高消耗，避免误吃鱼王。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FishRecipe {
    Jerky,
    Ball,
    Soup,
    Oil,
}

impl FishRecipe {
    pub const ALL: [FishRecipe; 4] = [
        FishRecipe::Jerky,
        FishRecipe::Ball,
        FishRecipe::Soup,
        FishRecipe::Oil,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FishRecipe::Jerky => "鱼干",
            FishRecipe::Ball => "鱼丸",
            FishRecipe::Soup => "鱼汤",
            FishRecipe::Oil => "鱼油",
        }
    }

    pub fn fish_count(self) -> u32 {
        match self {
            FishRecipe::Jerky => 2,
            FishRecipe::Ball => 3,
            FishRecipe::Soup => 1,
            FishRecipe::Oil => 4,
        }
    }

    pub fn other_inputs(self) -> &'static [(&'static str, u32)] {
        match self {
            FishRecipe::Jerky => &[],
            FishRecipe::Ball => &[("crop:luobo", 1)],
            FishRecipe::Soup => &[("crop:xiaobaicai", 2)],
            FishRecipe::Oil => &[],
        }
    }

    pub fn output_key(self) -> &'static str {
        match self {
            FishRecipe::Jerky => "craft:fish_jerky",
            FishRecipe::Ball => "craft:fish_ball",
            FishRecipe::Soup => "craft:fish_soup",
            FishRecipe::Oil => "craft:fish_oil",
        }
    }

    pub fn output_name(self) -> &'static str {
        self.label()
    }

    pub fn output_qty(self) -> u32 {
        match self {
            FishRecipe::Jerky => 1,
            FishRecipe::Ball => 2,
            FishRecipe::Soup => 1,
            FishRecipe::Oil => 1,
        }
    }

    pub fn duration_ms(self) -> i64 {
        match self {
            FishRecipe::Jerky => 30 * 60_000,
            FishRecipe::Ball => 45 * 60_000,
            FishRecipe::Soup => 30 * 60_000,
            FishRecipe::Oil => 60 * 60_000,
        }
    }
}

pub fn crop_box_key(crop_id: &str) -> String {
    format!("craft:box:{crop_id}")
}

pub fn crop_box_price(crop_sell_price: u64) -> u64 {
    let input = crop_sell_price.saturating_mul(u64::from(CROP_BOX_INPUT));
    let margin = (input.saturating_mul(20) / 100).clamp(
        CROP_BOX_MIN_MARGIN,
        workshop_margin(CROP_BOX_INTERVAL_MS),
    );
    input.saturating_add(margin)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentKind {
    Manual,
    StoneMill,
    Juicer,
    BoilingPot,
}

impl EquipmentKind {
    pub const INSTALLABLE: [EquipmentKind; 3] = [
        EquipmentKind::StoneMill,
        EquipmentKind::Juicer,
        EquipmentKind::BoilingPot,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EquipmentKind::Manual => "手工台",
            EquipmentKind::StoneMill => "石磨",
            EquipmentKind::Juicer => "榨汁机",
            EquipmentKind::BoilingPot => "熬煮锅",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            EquipmentKind::Manual => "🧰",
            EquipmentKind::StoneMill => "🪨",
            EquipmentKind::Juicer => "🧃",
            EquipmentKind::BoilingPot => "🍯",
        }
    }

    pub fn cost(self) -> u64 {
        match self {
            EquipmentKind::Manual => 0,
            EquipmentKind::StoneMill => 4_000,
            EquipmentKind::Juicer => 6_000,
            EquipmentKind::BoilingPot => 8_000,
        }
    }

    pub fn level(self) -> u32 {
        match self {
            EquipmentKind::Manual => 1,
            EquipmentKind::StoneMill => 10,
            EquipmentKind::Juicer => 12,
            EquipmentKind::BoilingPot => 14,
        }
    }

    pub fn recipes(self) -> &'static [EquipmentRecipe] {
        match self {
            EquipmentKind::Manual => &[],
            EquipmentKind::StoneMill => &[
                EquipmentRecipe::CornStarch,
                EquipmentRecipe::CoffeePowder,
                EquipmentRecipe::GinsengSlice,
                EquipmentRecipe::LingzhiPowder,
            ],
            EquipmentKind::Juicer => &[
                EquipmentRecipe::CaneJuice,
                EquipmentRecipe::StrawberryJuice,
                EquipmentRecipe::GrapeJuice,
                EquipmentRecipe::WatermelonJuice,
            ],
            EquipmentKind::BoilingPot => &[
                EquipmentRecipe::Sugar,
                EquipmentRecipe::Cream,
                EquipmentRecipe::StrawberryJam,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentRecipe {
    CornStarch,
    CoffeePowder,
    GinsengSlice,
    LingzhiPowder,
    CaneJuice,
    StrawberryJuice,
    GrapeJuice,
    WatermelonJuice,
    Sugar,
    Cream,
    StrawberryJam,
}

impl EquipmentRecipe {
    pub const ALL: [EquipmentRecipe; 11] = [
        EquipmentRecipe::CornStarch,
        EquipmentRecipe::CoffeePowder,
        EquipmentRecipe::GinsengSlice,
        EquipmentRecipe::LingzhiPowder,
        EquipmentRecipe::CaneJuice,
        EquipmentRecipe::StrawberryJuice,
        EquipmentRecipe::GrapeJuice,
        EquipmentRecipe::WatermelonJuice,
        EquipmentRecipe::Sugar,
        EquipmentRecipe::Cream,
        EquipmentRecipe::StrawberryJam,
    ];

    pub fn equipment(self) -> EquipmentKind {
        match self {
            EquipmentRecipe::CornStarch
            | EquipmentRecipe::CoffeePowder
            | EquipmentRecipe::GinsengSlice
            | EquipmentRecipe::LingzhiPowder => EquipmentKind::StoneMill,
            EquipmentRecipe::CaneJuice
            | EquipmentRecipe::StrawberryJuice
            | EquipmentRecipe::GrapeJuice
            | EquipmentRecipe::WatermelonJuice => EquipmentKind::Juicer,
            EquipmentRecipe::Sugar | EquipmentRecipe::Cream | EquipmentRecipe::StrawberryJam => {
                EquipmentKind::BoilingPot
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EquipmentRecipe::CornStarch => "玉米淀粉",
            EquipmentRecipe::CoffeePowder => "咖啡粉",
            EquipmentRecipe::GinsengSlice => "参片",
            EquipmentRecipe::LingzhiPowder => "灵芝粉",
            EquipmentRecipe::CaneJuice => "甘蔗汁",
            EquipmentRecipe::StrawberryJuice => "草莓汁",
            EquipmentRecipe::GrapeJuice => "葡萄汁",
            EquipmentRecipe::WatermelonJuice => "西瓜汁",
            EquipmentRecipe::Sugar => "白糖",
            EquipmentRecipe::Cream => "奶油",
            EquipmentRecipe::StrawberryJam => "草莓酱",
        }
    }

    pub fn inputs(self) -> &'static [(&'static str, u32)] {
        match self {
            EquipmentRecipe::CornStarch => &[("crop:yumi", 3)],
            EquipmentRecipe::CoffeePowder => &[("crop:kafei", 2)],
            EquipmentRecipe::GinsengSlice => &[("crop:renshen", 1)],
            EquipmentRecipe::LingzhiPowder => &[("crop:lingzhi", 1)],
            EquipmentRecipe::CaneJuice => &[("crop:ganzhe", 4)],
            EquipmentRecipe::StrawberryJuice => &[("crop:caomei", 4)],
            EquipmentRecipe::GrapeJuice => &[("crop:putao", 3)],
            EquipmentRecipe::WatermelonJuice => &[("crop:xigua", 1)],
            EquipmentRecipe::Sugar => &[("craft:cane_juice", 2)],
            EquipmentRecipe::Cream => &[("ranch:milk", 2)],
            EquipmentRecipe::StrawberryJam => &[
                ("craft:strawberry_juice", 1),
                ("craft:sugar", 1),
                ("craft:corn_starch", 1),
            ],
        }
    }

    pub fn duration_ms(self) -> i64 {
        match self {
            EquipmentRecipe::CornStarch
            | EquipmentRecipe::CoffeePowder
            | EquipmentRecipe::GinsengSlice
            | EquipmentRecipe::LingzhiPowder
            | EquipmentRecipe::Sugar
            | EquipmentRecipe::StrawberryJam => 2 * 60 * 60 * 1000,
            EquipmentRecipe::CaneJuice
            | EquipmentRecipe::StrawberryJuice
            | EquipmentRecipe::GrapeJuice
            | EquipmentRecipe::WatermelonJuice
            | EquipmentRecipe::Cream => 90 * 60 * 1000,
        }
    }

    pub fn output_key(self) -> &'static str {
        match self {
            EquipmentRecipe::CornStarch => "craft:corn_starch",
            EquipmentRecipe::CoffeePowder => "craft:coffee_powder",
            EquipmentRecipe::GinsengSlice => "craft:ginseng_slice",
            EquipmentRecipe::LingzhiPowder => "craft:lingzhi_powder",
            EquipmentRecipe::CaneJuice => "craft:cane_juice",
            EquipmentRecipe::StrawberryJuice => "craft:strawberry_juice",
            EquipmentRecipe::GrapeJuice => "craft:grape_juice",
            EquipmentRecipe::WatermelonJuice => "craft:watermelon_juice",
            EquipmentRecipe::Sugar => "craft:sugar",
            EquipmentRecipe::Cream => "craft:cream",
            EquipmentRecipe::StrawberryJam => "craft:strawberry_jam",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CraftItemDef {
    pub key: &'static str,
    pub name: &'static str,
    pub price: u64,
}

pub const CRAFT_ITEMS: &[CraftItemDef] = &[
    CraftItemDef {
        key: "craft:corn_starch",
        name: "玉米淀粉",
        price: 260,
    },
    CraftItemDef {
        key: "craft:coffee_powder",
        name: "咖啡粉",
        price: 490,
    },
    CraftItemDef {
        key: "craft:ginseng_slice",
        name: "参片",
        price: 560,
    },
    CraftItemDef {
        key: "craft:lingzhi_powder",
        name: "灵芝粉",
        price: 440,
    },
    CraftItemDef {
        key: "craft:cane_juice",
        name: "甘蔗汁",
        price: 238,
    },
    CraftItemDef {
        key: "craft:strawberry_juice",
        name: "草莓汁",
        price: 210,
    },
    CraftItemDef {
        key: "craft:grape_juice",
        name: "葡萄汁",
        price: 270,
    },
    CraftItemDef {
        key: "craft:watermelon_juice",
        name: "西瓜汁",
        price: 195,
    },
    CraftItemDef {
        key: "craft:sugar",
        name: "白糖",
        price: 676,
    },
    CraftItemDef {
        key: "craft:cream",
        name: "奶油",
        price: 590,
    },
    CraftItemDef {
        key: "craft:strawberry_jam",
        name: "草莓酱",
        price: 1346,
    },
    CraftItemDef {
        key: "craft:fish_jerky",
        name: "鱼干",
        price: 45,
    },
    CraftItemDef {
        key: "craft:fish_ball",
        name: "鱼丸",
        price: 35,
    },
    CraftItemDef {
        key: "craft:fish_soup",
        name: "鱼汤",
        price: 60,
    },
    CraftItemDef {
        key: "craft:fish_oil",
        name: "鱼油",
        price: 120,
    },
];

/// 制造屋成品/中间品的固定价格表；礼盒走 `crop_box_price`。
pub fn craft_item(key: &str) -> Option<&'static CraftItemDef> {
    CRAFT_ITEMS.iter().find(|item| item.key == key)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WorkshopJob {
    Ranch {
        recipe: RanchRecipe,
        started_at: i64,
        ready_at: i64,
        quality: u8,
    },
    CropBox {
        crop_id: String,
        started_at: i64,
        ready_at: i64,
    },
    Equipment {
        recipe: EquipmentRecipe,
        started_at: i64,
        ready_at: i64,
    },
    Bait {
        recipe: BaitRecipe,
        started_at: i64,
        ready_at: i64,
    },
    Fish {
        recipe: FishRecipe,
        started_at: i64,
        ready_at: i64,
    },
}

impl WorkshopJob {
    pub fn started_at(&self) -> i64 {
        match self {
            WorkshopJob::Ranch { started_at, .. }
            | WorkshopJob::CropBox { started_at, .. }
            | WorkshopJob::Equipment { started_at, .. }
            | WorkshopJob::Bait { started_at, .. }
            | WorkshopJob::Fish { started_at, .. } => *started_at,
        }
    }

    pub fn ready_at(&self) -> i64 {
        match self {
            WorkshopJob::Ranch { ready_at, .. }
            | WorkshopJob::CropBox { ready_at, .. }
            | WorkshopJob::Equipment { ready_at, .. }
            | WorkshopJob::Bait { ready_at, .. }
            | WorkshopJob::Fish { ready_at, .. } => *ready_at,
        }
    }

    pub fn quality(&self) -> u8 {
        match self {
            WorkshopJob::Ranch { quality, .. } => *quality,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkshopState {
    pub built: bool,
    pub slots: u8,
    pub equipment: Vec<Option<EquipmentKind>>,
    pub jobs: Vec<Option<WorkshopJob>>,
}

impl Default for WorkshopState {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkshopState {
    pub fn new() -> Self {
        let mut state = Self {
            built: false,
            slots: WORKSHOP_BASE_SLOTS,
            equipment: Vec::new(),
            jobs: Vec::new(),
        };
        state.ensure();
        state
    }

    pub fn ensure(&mut self) {
        self.slots = self.slots.clamp(WORKSHOP_BASE_SLOTS, WORKSHOP_MAX_SLOTS);
        let slots = usize::from(self.slots);
        self.equipment.resize(slots, None);
        self.jobs.resize(slots, None);
        if self.equipment[0].is_none() {
            self.equipment[0] = Some(EquipmentKind::Manual);
        }
    }

    pub fn slot_count(&self) -> u8 {
        self.slots
    }

    pub fn equipment(&self, slot: u8) -> Option<EquipmentKind> {
        self.equipment.get(usize::from(slot)).copied().flatten()
    }

    pub fn job(&self, slot: u8) -> Option<&WorkshopJob> {
        self.jobs
            .get(usize::from(slot))
            .and_then(|job| job.as_ref())
    }

    pub fn job_mut(&mut self, slot: u8) -> Option<&mut Option<WorkshopJob>> {
        self.jobs.get_mut(usize::from(slot))
    }

    pub fn has_idle_equipment(&self, kind: EquipmentKind) -> bool {
        (0..self.slots).any(|slot| self.equipment(slot) == Some(kind) && self.job(slot).is_none())
    }

    pub fn first_idle_slot(&self, kind: EquipmentKind) -> Option<u8> {
        (0..self.slots)
            .find(|slot| self.equipment(*slot) == Some(kind) && self.job(*slot).is_none())
    }

    pub fn can_expand(&self) -> bool {
        self.slots < WORKSHOP_MAX_SLOTS
    }

    pub fn next_expand_cost(&self) -> u64 {
        let step = usize::from(self.slots.saturating_sub(WORKSHOP_BASE_SLOTS));
        WORKSHOP_SLOT_COSTS
            .get(step)
            .copied()
            .unwrap_or(*WORKSHOP_SLOT_COSTS.last().unwrap_or(&0))
    }

    pub fn next_expand_level(&self) -> u32 {
        let step = usize::from(self.slots.saturating_sub(WORKSHOP_BASE_SLOTS));
        WORKSHOP_SLOT_LEVELS
            .get(step)
            .copied()
            .unwrap_or(*WORKSHOP_SLOT_LEVELS.last().unwrap_or(&0))
    }

    pub fn expand(&mut self) -> bool {
        if !self.can_expand() {
            return false;
        }
        self.slots += 1;
        self.ensure();
        true
    }

    pub fn install(&mut self, slot: u8, kind: EquipmentKind) -> bool {
        if slot == 0 || slot >= self.slots || kind == EquipmentKind::Manual {
            return false;
        }
        let Some(target) = self.equipment.get_mut(usize::from(slot)) else {
            return false;
        };
        if target.is_some() {
            return false;
        }
        *target = Some(kind);
        true
    }
}

#[cfg(test)]
mod balance_tests {
    use super::*;
    use crate::content::ContentPack;
    use crate::ranch::{item_def, RanchRecipe};

    fn crop_price(id: &str) -> u64 {
        ContentPack::starter()
            .crop(id)
            .map(|crop| crop.sell_price)
            .unwrap_or(0)
    }

    fn input_value(inputs: &[(&str, u32)]) -> u64 {
        inputs
            .iter()
            .map(|(key, need)| {
                let price = if let Some(def) = craft_item(key) {
                    def.price
                } else if let Some(def) = item_def(key) {
                    def.base_price
                } else {
                    crop_price(key.strip_prefix("crop:").unwrap_or(key))
                };
                price.saturating_mul(u64::from(*need))
            })
            .sum()
    }

    fn assert_target(input: u64, duration_ms: i64, output: u64, label: &str) {
        assert_eq!(
            output,
            workshop_price(input, duration_ms),
            "{label}：输出价应等于原料 + 工时边距"
        );
        let margin = output.saturating_sub(input);
        let per_hour = margin
            .saturating_mul(3_600_000)
            .checked_div(duration_ms.max(1) as u64)
            .unwrap_or(0);
        assert_eq!(
            per_hour, WORKSHOP_PROFIT_PER_HOUR,
            "{label}：每小时净赚应为 {WORKSHOP_PROFIT_PER_HOUR}"
        );
    }

    #[test]
    fn ranch_recipes_earn_target_per_hour() {
        for recipe in RanchRecipe::ALL {
            let output = item_def(recipe.output_key())
                .map(|item| item.base_price)
                .unwrap_or(0);
            assert_target(
                input_value(recipe.inputs()),
                recipe.duration_ms(),
                output,
                recipe.label(),
            );
        }
    }

    #[test]
    fn equipment_recipes_earn_target_per_hour() {
        for recipe in EquipmentRecipe::ALL {
            let output = craft_item(recipe.output_key())
                .map(|item| item.price)
                .unwrap_or(0);
            assert_target(
                input_value(recipe.inputs()),
                recipe.duration_ms(),
                output,
                recipe.label(),
            );
        }
    }

    #[test]
    fn crop_boxes_keep_early_margin_and_cap_late_margin() {
        // 5 份小青菜 30 → 90（最低 60 边距）
        assert_eq!(crop_box_price(6), 90);
        // 5 份玉米 100 → 160（最低 60 边距）
        assert_eq!(crop_box_price(20), 160);
        // 5 份月见草 2250 → 2550（封顶 300 边距）
        assert_eq!(crop_box_price(450), 2550);
    }
}
