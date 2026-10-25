use serde::{Deserialize, Serialize};

use crate::fishing::BaitRecipe;
use crate::ranch::RanchRecipe;

pub const WORKSHOP_BASE_SLOTS: u8 = 3;
pub const WORKSHOP_MAX_SLOTS: u8 = 5;
/// 设备最高 5 级；每级 +4% 加工速度、+3% 取货增产概率。
pub const WORKSHOP_EQUIPMENT_MAX_LEVEL: u8 = 5;
pub const WORKSHOP_LEVEL_SPEED_PCT: u32 = 4;
pub const WORKSHOP_LEVEL_YIELD_PCT: u32 = 3;
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
    Oven,
    Mixer,
    OilPress,
    Loom,
    Still,
}

impl EquipmentKind {
    pub const INSTALLABLE: [EquipmentKind; 8] = [
        EquipmentKind::StoneMill,
        EquipmentKind::Juicer,
        EquipmentKind::BoilingPot,
        EquipmentKind::Oven,
        EquipmentKind::Mixer,
        EquipmentKind::OilPress,
        EquipmentKind::Loom,
        EquipmentKind::Still,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EquipmentKind::Manual => "手工台",
            EquipmentKind::StoneMill => "石磨",
            EquipmentKind::Juicer => "榨汁机",
            EquipmentKind::BoilingPot => "熬煮锅",
            EquipmentKind::Oven => "烤箱",
            EquipmentKind::Mixer => "搅拌台",
            EquipmentKind::OilPress => "榨油机",
            EquipmentKind::Loom => "纺织机",
            EquipmentKind::Still => "蒸馏器",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            EquipmentKind::Manual => "🧰",
            EquipmentKind::StoneMill => "🪨",
            EquipmentKind::Juicer => "🧃",
            EquipmentKind::BoilingPot => "🍯",
            EquipmentKind::Oven => "🍞",
            EquipmentKind::Mixer => "🥤",
            EquipmentKind::OilPress => "🌻",
            EquipmentKind::Loom => "🧵",
            EquipmentKind::Still => "⚗️",
        }
    }

    pub fn cost(self) -> u64 {
        match self {
            EquipmentKind::Manual => 0,
            EquipmentKind::StoneMill => 4_000,
            EquipmentKind::Juicer => 6_000,
            EquipmentKind::BoilingPot => 8_000,
            EquipmentKind::Oven => 12_000,
            EquipmentKind::Mixer => 10_000,
            EquipmentKind::OilPress => 10_000,
            EquipmentKind::Loom => 15_000,
            EquipmentKind::Still => 20_000,
        }
    }

    pub fn level(self) -> u32 {
        match self {
            EquipmentKind::Manual => 1,
            EquipmentKind::StoneMill => 10,
            EquipmentKind::Juicer => 12,
            EquipmentKind::BoilingPot => 14,
            EquipmentKind::Oven => 16,
            EquipmentKind::Mixer => 16,
            EquipmentKind::OilPress => 18,
            EquipmentKind::Loom => 20,
            EquipmentKind::Still => 24,
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
            EquipmentKind::Oven => &[EquipmentRecipe::CornPudding],
            EquipmentKind::Mixer => &[
                EquipmentRecipe::StrawberryShake,
                EquipmentRecipe::Latte,
                EquipmentRecipe::Mayonnaise,
            ],
            EquipmentKind::OilPress => &[EquipmentRecipe::SunflowerOil],
            EquipmentKind::Loom => &[EquipmentRecipe::Sweater],
            EquipmentKind::Still => &[EquipmentRecipe::EssentialOil],
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
    CornPudding,
    StrawberryShake,
    Latte,
    Mayonnaise,
    SunflowerOil,
    Sweater,
    EssentialOil,
    Dumpling,
    Tangyuan,
    Qingtuan,
    Zongzi,
    Mooncake,
    WinterStew,
}

impl EquipmentRecipe {
    pub const ALL: [EquipmentRecipe; 24] = [
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
        EquipmentRecipe::CornPudding,
        EquipmentRecipe::StrawberryShake,
        EquipmentRecipe::Latte,
        EquipmentRecipe::Mayonnaise,
        EquipmentRecipe::SunflowerOil,
        EquipmentRecipe::Sweater,
        EquipmentRecipe::EssentialOil,
        EquipmentRecipe::Dumpling,
        EquipmentRecipe::Tangyuan,
        EquipmentRecipe::Qingtuan,
        EquipmentRecipe::Zongzi,
        EquipmentRecipe::Mooncake,
        EquipmentRecipe::WinterStew,
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
            EquipmentRecipe::CornPudding => EquipmentKind::Oven,
            EquipmentRecipe::StrawberryShake
            | EquipmentRecipe::Latte
            | EquipmentRecipe::Mayonnaise => EquipmentKind::Mixer,
            EquipmentRecipe::SunflowerOil => EquipmentKind::OilPress,
            EquipmentRecipe::Sweater => EquipmentKind::Loom,
            EquipmentRecipe::EssentialOil => EquipmentKind::Still,
            EquipmentRecipe::Dumpling
            | EquipmentRecipe::Tangyuan
            | EquipmentRecipe::Qingtuan
            | EquipmentRecipe::Zongzi
            | EquipmentRecipe::WinterStew => EquipmentKind::BoilingPot,
            EquipmentRecipe::Mooncake => EquipmentKind::Oven,
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
            EquipmentRecipe::CornPudding => "玉米布丁",
            EquipmentRecipe::StrawberryShake => "草莓奶昔",
            EquipmentRecipe::Latte => "拿铁",
            EquipmentRecipe::Mayonnaise => "蛋黄酱",
            EquipmentRecipe::SunflowerOil => "葵花籽油",
            EquipmentRecipe::Sweater => "毛衣",
            EquipmentRecipe::EssentialOil => "月见草精油",
            EquipmentRecipe::Dumpling => "饺子",
            EquipmentRecipe::Tangyuan => "汤圆",
            EquipmentRecipe::Qingtuan => "青团",
            EquipmentRecipe::Zongzi => "粽子",
            EquipmentRecipe::Mooncake => "月饼",
            EquipmentRecipe::WinterStew => "围炉汤",
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
            EquipmentRecipe::CornPudding => &[
                ("craft:corn_starch", 1),
                ("craft:sugar", 1),
                ("ranch:egg", 2),
                ("ranch:milk", 1),
            ],
            EquipmentRecipe::StrawberryShake => {
                &[("craft:strawberry_jam", 1), ("ranch:milk", 1)]
            }
            EquipmentRecipe::Latte => &[("craft:coffee_powder", 1), ("ranch:milk", 1)],
            EquipmentRecipe::Mayonnaise => &[("craft:sunflower_oil", 1), ("ranch:egg", 2)],
            EquipmentRecipe::SunflowerOil => &[("crop:xiangrikui", 5)],
            EquipmentRecipe::Sweater => &[("ranch:yarn", 3)],
            EquipmentRecipe::EssentialOil => &[("crop:yuejiancao", 2)],
            EquipmentRecipe::Dumpling => &[
                ("crop:xiaobaicai", 2),
                ("ranch:egg", 2),
                ("crop:yumi", 1),
            ],
            EquipmentRecipe::Tangyuan => &[("crop:yumi", 2), ("ranch:milk", 1)],
            EquipmentRecipe::Qingtuan => &[("crop:xiaobaicai", 2), ("crop:yumi", 1)],
            EquipmentRecipe::Zongzi => &[("crop:yumi", 3), ("crop:xiaobaicai", 1)],
            EquipmentRecipe::Mooncake => &[
                ("crop:yumi", 2),
                ("ranch:egg", 1),
                ("ranch:milk", 1),
            ],
            EquipmentRecipe::WinterStew => &[
                ("crop:luobo", 2),
                ("crop:xiaobaicai", 2),
                ("ranch:egg", 1),
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
            | EquipmentRecipe::StrawberryJam
            | EquipmentRecipe::CornPudding
            | EquipmentRecipe::SunflowerOil => 2 * 60 * 60 * 1000,
            EquipmentRecipe::CaneJuice
            | EquipmentRecipe::StrawberryJuice
            | EquipmentRecipe::GrapeJuice
            | EquipmentRecipe::WatermelonJuice
            | EquipmentRecipe::Cream => 90 * 60 * 1000,
            EquipmentRecipe::StrawberryShake | EquipmentRecipe::Mayonnaise => 60 * 60 * 1000,
            EquipmentRecipe::Latte => 30 * 60 * 1000,
            EquipmentRecipe::Sweater => 4 * 60 * 60 * 1000,
            EquipmentRecipe::EssentialOil => 3 * 60 * 60 * 1000,
            EquipmentRecipe::Dumpling
            | EquipmentRecipe::Tangyuan
            | EquipmentRecipe::Qingtuan
            | EquipmentRecipe::Zongzi
            | EquipmentRecipe::Mooncake
            | EquipmentRecipe::WinterStew => 60 * 60 * 1000,
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
            EquipmentRecipe::CornPudding => "craft:corn_pudding",
            EquipmentRecipe::StrawberryShake => "craft:strawberry_shake",
            EquipmentRecipe::Latte => "craft:latte",
            EquipmentRecipe::Mayonnaise => "craft:mayonnaise",
            EquipmentRecipe::SunflowerOil => "craft:sunflower_oil",
            EquipmentRecipe::Sweater => "craft:sweater",
            EquipmentRecipe::EssentialOil => "craft:essential_oil",
            EquipmentRecipe::Dumpling => "craft:dumpling",
            EquipmentRecipe::Tangyuan => "craft:tangyuan",
            EquipmentRecipe::Qingtuan => "craft:qingtuan",
            EquipmentRecipe::Zongzi => "craft:zongzi",
            EquipmentRecipe::Mooncake => "craft:mooncake",
            EquipmentRecipe::WinterStew => "craft:winter_stew",
        }
    }

    /// 节日限定配方；None 表示平时也能做。
    pub fn festival(self) -> Option<&'static str> {
        match self {
            EquipmentRecipe::Dumpling => Some("春节"),
            EquipmentRecipe::Tangyuan => Some("元宵节"),
            EquipmentRecipe::Qingtuan => Some("清明节"),
            EquipmentRecipe::Zongzi => Some("端午节"),
            EquipmentRecipe::Mooncake => Some("中秋节"),
            EquipmentRecipe::WinterStew => Some("围炉日"),
            _ => None,
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
        key: "craft:corn_pudding",
        name: "玉米布丁",
        price: 1446,
    },
    CraftItemDef {
        key: "craft:strawberry_shake",
        name: "草莓奶昔",
        price: 1666,
    },
    CraftItemDef {
        key: "craft:latte",
        name: "拿铁",
        price: 760,
    },
    CraftItemDef {
        key: "craft:mayonnaise",
        name: "蛋黄酱",
        price: 510,
    },
    CraftItemDef {
        key: "craft:sunflower_oil",
        name: "葵花籽油",
        price: 320,
    },
    CraftItemDef {
        key: "craft:sweater",
        name: "毛衣",
        price: 3100,
    },
    CraftItemDef {
        key: "craft:essential_oil",
        name: "月见草精油",
        price: 1200,
    },
    CraftItemDef {
        key: "craft:dumpling",
        name: "饺子",
        price: 222,
    },
    CraftItemDef {
        key: "craft:tangyuan",
        name: "汤圆",
        price: 360,
    },
    CraftItemDef {
        key: "craft:qingtuan",
        name: "青团",
        price: 132,
    },
    CraftItemDef {
        key: "craft:zongzi",
        name: "粽子",
        price: 166,
    },
    CraftItemDef {
        key: "craft:mooncake",
        name: "月饼",
        price: 405,
    },
    CraftItemDef {
        key: "craft:winter_stew",
        name: "围炉汤",
        price: 173,
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
    CraftItemDef {
        key: "craft:mystery_elixir",
        name: "奥秘药剂",
        price: 2400,
    },
    CraftItemDef {
        key: "craft:mystery_tonic",
        name: "丰饶补剂",
        price: 1600,
    },
    CraftItemDef {
        key: "craft:ranch_elixir",
        name: "牧场秘药",
        price: 2000,
    },
    CraftItemDef {
        key: "craft:fishing_elixir",
        name: "江海秘药",
        price: 2200,
    },
    CraftItemDef {
        key: "craft:craft_elixir",
        name: "工匠秘药",
        price: 2000,
    },
    CraftItemDef {
        key: "craft:coin_elixir",
        name: "点金秘药",
        price: 2600,
    },
    CraftItemDef {
        key: "craft:farm_card",
        name: "农场一键卡",
        price: 8000,
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
        /// 由哪笔自动排程订单启动；空 = 玩家手动开工。
        #[serde(default)]
        auto_order: String,
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
    /// 与 equipment 对齐的 1–5 级；旧档默认 1。
    #[serde(default)]
    pub equipment_levels: Vec<u8>,
    /// 制造屋“自动做”当前任务。
    #[serde(default)]
    pub auto_task: Option<WorkshopAutoTask>,
    /// 最近一次自动做完成报告；给自动汇报卡用。
    #[serde(default)]
    pub auto_report: Option<WorkshopAutoReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopAutoTask {
    pub recipe: EquipmentRecipe,
    pub qty: u32,
    pub done: u32,
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopAutoReport {
    pub at: i64,
    pub product_name: String,
    pub qty: u32,
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
            equipment_levels: Vec::new(),
            auto_task: None,
            auto_report: None,
        };
        state.ensure();
        state
    }

    pub fn ensure(&mut self) {
        self.slots = self.slots.clamp(WORKSHOP_BASE_SLOTS, WORKSHOP_MAX_SLOTS);
        let slots = usize::from(self.slots);
        self.equipment.resize(slots, None);
        self.jobs.resize(slots, None);
        self.equipment_levels.resize(slots, 1);
        for level in &mut self.equipment_levels {
            *level = (*level).clamp(1, WORKSHOP_EQUIPMENT_MAX_LEVEL);
        }
        if self.equipment[0].is_none() {
            self.equipment[0] = Some(EquipmentKind::Manual);
        }
        self.equipment_levels[0] = 1;
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
        if let Some(level) = self.equipment_levels.get_mut(usize::from(slot)) {
            *level = 1;
        }
        true
    }

    pub fn equipment_level(&self, slot: u8) -> u8 {
        self.equipment_levels
            .get(usize::from(slot))
            .copied()
            .unwrap_or(1)
            .clamp(1, WORKSHOP_EQUIPMENT_MAX_LEVEL)
    }

    /// 加工速度加成（百分点）：2 级 +4%，5 级 +16%。
    pub fn equipment_speed_pct(&self, slot: u8) -> u32 {
        u32::from(self.equipment_level(slot).saturating_sub(1)) * WORKSHOP_LEVEL_SPEED_PCT
    }

    /// 取货增产概率加成（百分点）：2 级 +3%，5 级 +12%。
    pub fn equipment_yield_pct(&self, slot: u8) -> u32 {
        u32::from(self.equipment_level(slot).saturating_sub(1)) * WORKSHOP_LEVEL_YIELD_PCT
    }

    /// 下一级升级费用；手工台/空槽/满级返回 None。
    pub fn equipment_upgrade_cost(&self, slot: u8) -> Option<u64> {
        let kind = self.equipment(slot)?;
        if slot == 0 || kind == EquipmentKind::Manual {
            return None;
        }
        let level = self.equipment_level(slot);
        if level >= WORKSHOP_EQUIPMENT_MAX_LEVEL {
            return None;
        }
        Some(kind.cost().max(2_000) * u64::from(level) / 2)
    }

    pub fn upgrade_equipment(&mut self, slot: u8) -> bool {
        if self.equipment_upgrade_cost(slot).is_none() {
            return false;
        }
        let Some(level) = self.equipment_levels.get_mut(usize::from(slot)) else {
            return false;
        };
        *level += 1;
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
