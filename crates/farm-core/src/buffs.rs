//! 临时加成（食物/饮品）与叠加规则。
//!
//! 规则：同一条 buff 重复使用只刷新时间；不同 buff 的同一项相加后封顶；
//! 永久专精不在这里，应用时「专精 + buff」相加再按各自上限收口。

use serde::{Deserialize, Serialize};

use crate::skills::DisciplineKind;

/// 单项加成上限（百分比点）。
pub const EXP_CAP: u32 = 50;
pub const PLANT_YIELD_CAP: u32 = 30;
pub const CRAFT_SPEED_CAP: u32 = 30;
pub const RANCH_QUALITY_CAP: u32 = 30;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum BuffStat {
    /// 四专业经验都加。
    #[default]
    ExpAll,
    /// 只加种植经验。
    ExpPlanting,
    /// 只加钓鱼经验。
    ExpFishing,
    /// 种植产量。
    PlantYield,
    /// 加工速度（缩短工时）。
    CraftSpeed,
    /// 畜产品升品质的概率。
    RanchQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Buff {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub stat: BuffStat,
    pub value: u32,
    pub expires_at: i64,
    pub source: String,
}

#[derive(Debug, Clone, Copy)]
pub struct BuffDef {
    pub item_key: &'static str,
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub stat: BuffStat,
    pub value: u32,
    pub duration_ms: i64,
}

const MIN: i64 = 60_000;

/// 可吃喝的加成食物；只列已经存在的成品。
pub const BUFF_ITEMS: [BuffDef; 9] = [
    BuffDef {
        item_key: "craft:strawberry_juice",
        id: "buff_plant_exp",
        name: "草莓汁",
        desc: "种植经验 +20%，30 分钟",
        stat: BuffStat::ExpPlanting,
        value: 20,
        duration_ms: 30 * MIN,
    },
    BuffDef {
        item_key: "craft:grape_juice",
        id: "buff_plant_yield",
        name: "葡萄汁",
        desc: "种植产量 +10%，30 分钟",
        stat: BuffStat::PlantYield,
        value: 10,
        duration_ms: 30 * MIN,
    },
    BuffDef {
        item_key: "craft:watermelon_juice",
        id: "buff_fish_exp",
        name: "西瓜汁",
        desc: "钓鱼经验 +20%，30 分钟",
        stat: BuffStat::ExpFishing,
        value: 20,
        duration_ms: 30 * MIN,
    },
    BuffDef {
        item_key: "craft:cane_juice",
        id: "buff_craft_speed",
        name: "甘蔗汁",
        desc: "加工工时 −10%，30 分钟",
        stat: BuffStat::CraftSpeed,
        value: 10,
        duration_ms: 30 * MIN,
    },
    BuffDef {
        item_key: "ranch:cheese",
        id: "buff_ranch_quality",
        name: "奶酪",
        desc: "畜产品升品质概率 +10%，30 分钟",
        stat: BuffStat::RanchQuality,
        value: 10,
        duration_ms: 30 * MIN,
    },
    BuffDef {
        item_key: "ranch:cake",
        id: "buff_all_exp",
        name: "蛋糕",
        desc: "四专业经验 +15%，60 分钟",
        stat: BuffStat::ExpAll,
        value: 15,
        duration_ms: 60 * MIN,
    },
    BuffDef {
        item_key: "craft:fish_soup",
        id: "buff_fish_soup",
        name: "鱼汤",
        desc: "钓鱼经验 +15%，30 分钟",
        stat: BuffStat::ExpFishing,
        value: 15,
        duration_ms: 30 * MIN,
    },
    BuffDef {
        item_key: "craft:mystery_elixir",
        id: "buff_mystery_elixir",
        name: "奥秘药剂",
        desc: "四专业经验 +30%，2 小时",
        stat: BuffStat::ExpAll,
        value: 30,
        duration_ms: 120 * MIN,
    },
    BuffDef {
        item_key: "craft:mystery_tonic",
        id: "buff_mystery_tonic",
        name: "丰饶补剂",
        desc: "种植产量 +20%，1 小时",
        stat: BuffStat::PlantYield,
        value: 20,
        duration_ms: 60 * MIN,
    },
];

pub fn buff_item(key: &str) -> Option<&'static BuffDef> {
    if let Some(def) = BUFF_ITEMS.iter().find(|def| def.item_key == key) {
        return Some(def);
    }
    // 奶酪/蛋糕有 .fine / .prime 品质后缀；品质不改变 buff 数值，按基础物品算。
    let (base, _) = crate::ranch::split_quality(key);
    BUFF_ITEMS.iter().find(|def| def.item_key == base)
}

fn active<'a>(buffs: &'a [Buff], now: i64) -> impl Iterator<Item = &'a Buff> + 'a {
    buffs.iter().filter(move |b| b.expires_at > now)
}

/// 使用/吃喝一份加成食物；同 id 只刷新时间，不叠层。
pub fn apply(buffs: &mut Vec<Buff>, def: &BuffDef, now: i64) {
    buffs.retain(|b| b.expires_at > now);
    if let Some(buff) = buffs.iter_mut().find(|b| b.id == def.id) {
        buff.name = def.name.into();
        buff.desc = def.desc.into();
        buff.stat = def.stat;
        buff.value = def.value;
        buff.expires_at = now + def.duration_ms;
        buff.source = def.name.into();
    } else {
        buffs.push(Buff {
            id: def.id.into(),
            name: def.name.into(),
            desc: def.desc.into(),
            stat: def.stat,
            value: def.value,
            expires_at: now + def.duration_ms,
            source: def.name.into(),
        });
    }
}

/// 清掉已经过期的 buff。
pub fn prune(buffs: &mut Vec<Buff>, now: i64) {
    buffs.retain(|b| b.expires_at > now);
}

/// 某个专业的经验加成（全专业 + 该专业专属），封顶 50%。
pub fn exp_pct(buffs: &[Buff], now: i64, kind: DisciplineKind) -> u32 {
    let mut total = 0;
    for buff in active(buffs, now) {
        match buff.stat {
            BuffStat::ExpAll => total += buff.value,
            BuffStat::ExpPlanting if kind == DisciplineKind::Planting => total += buff.value,
            BuffStat::ExpFishing if kind == DisciplineKind::Fishing => total += buff.value,
            _ => {}
        }
    }
    total.min(EXP_CAP)
}

pub fn plant_yield_pct(buffs: &[Buff], now: i64) -> u32 {
    total(buffs, now, BuffStat::PlantYield).min(PLANT_YIELD_CAP)
}

pub fn craft_speed_pct(buffs: &[Buff], now: i64) -> u32 {
    total(buffs, now, BuffStat::CraftSpeed).min(CRAFT_SPEED_CAP)
}

pub fn ranch_quality_pct(buffs: &[Buff], now: i64) -> u32 {
    total(buffs, now, BuffStat::RanchQuality).min(RANCH_QUALITY_CAP)
}

fn total(buffs: &[Buff], now: i64, stat: BuffStat) -> u32 {
    active(buffs, now)
        .filter(|b| b.stat == stat)
        .map(|b| b.value)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn food_table_is_wired_to_existing_items() {
        assert_eq!(BUFF_ITEMS.len(), 9);
        assert_eq!(buff_item("craft:strawberry_juice").unwrap().value, 20);
        assert!(buff_item("craft:not_a_food").is_none());
        for def in BUFF_ITEMS {
            assert!(def.value > 0);
            assert!(def.duration_ms >= 30 * MIN);
        }
    }

    #[test]
    fn same_buff_refreshes_not_stacks() {
        let def = buff_item("ranch:cake").unwrap();
        let mut buffs = Vec::new();
        apply(&mut buffs, def, 0);
        apply(&mut buffs, def, 10 * MIN);
        assert_eq!(buffs.len(), 1);
        assert_eq!(buffs[0].expires_at, 70 * MIN);
    }

    #[test]
    fn same_stat_stacks_and_caps() {
        let cake = buff_item("ranch:cake").unwrap(); // ExpAll 15
        let juice = buff_item("craft:strawberry_juice").unwrap(); // ExpPlanting 20
        let mut buffs = Vec::new();
        apply(&mut buffs, cake, 0);
        apply(&mut buffs, juice, 0);
        assert_eq!(exp_pct(&buffs, 0, DisciplineKind::Planting), 35);
        assert_eq!(exp_pct(&buffs, 0, DisciplineKind::Ranching), 15);
        assert_eq!(exp_pct(&buffs, 0, DisciplineKind::Fishing), 15);

        // 手工塞到封顶以上，读出来必须是 50。
        buffs[0].value = 40;
        buffs[1].value = 40;
        assert_eq!(exp_pct(&buffs, 0, DisciplineKind::Planting), EXP_CAP);
    }

    #[test]
    fn expired_buffs_stop_applying_and_prune() {
        let def = buff_item("craft:grape_juice").unwrap();
        let mut buffs = Vec::new();
        apply(&mut buffs, def, 0);
        assert_eq!(plant_yield_pct(&buffs, 0), 10);
        assert_eq!(plant_yield_pct(&buffs, def.duration_ms + 1), 0);
        prune(&mut buffs, def.duration_ms + 1);
        assert!(buffs.is_empty());
    }
}
