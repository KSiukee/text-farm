//! 种植专业的专精效果与经验公式。

use crate::content::CropDef;

const YIELD_PCT: [u32; 4] = [4, 8, 12, 16];
const GROWTH_PCT: [u32; 4] = [3, 6, 9, 12];
const WATER_EXTRA_PCT: [u32; 4] = [2, 4, 6, 8];
const SEED_RETURN_PCT: [u32; 4] = [10, 20, 30, 40];

fn at(level: u32, table: &[u32; 4]) -> u32 {
    if level == 0 {
        0
    } else {
        table[(level - 1).min(3) as usize]
    }
}

/// 精耕：收获数量加成百分比。
pub fn yield_pct(level: u32) -> u32 {
    at(level, &YIELD_PCT)
}

/// 精耕带来的额外数量（向下取整）。
pub fn yield_bonus(qty: u32, level: u32) -> u32 {
    qty.saturating_mul(yield_pct(level)) / 100
}

/// 时令：生长时间缩短百分比。
pub fn growth_pct(level: u32) -> u32 {
    at(level, &GROWTH_PCT)
}

/// 保墒：浇水在基础 10% 之外再缩短的百分比。
pub fn water_extra_pct(level: u32) -> u32 {
    at(level, &WATER_EXTRA_PCT)
}

/// 留种：收获时返还 1 颗种子的概率。
pub fn seed_return_pct(level: u32) -> u32 {
    at(level, &SEED_RETURN_PCT)
}

/// 单次收获的种植经验：`2 + 作物等级/2 + 数量/2`，单次上限 20。
/// 当天第 41 次收获起减半（防刷最便宜作物）。
pub fn harvest_exp(def: &CropDef, qty: u32, harvests_today: u32) -> u64 {
    let base = 2u64 + u64::from(def.unlock_level) / 2 + u64::from(qty) / 2;
    let base = base.min(20);
    if harvests_today > 40 {
        base / 2
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentPack;

    fn def(id: &str) -> CropDef {
        ContentPack::starter().crop(id).unwrap().clone()
    }

    #[test]
    fn perk_tables_scale_one_to_four() {
        assert_eq!(yield_pct(0), 0);
        assert_eq!(yield_pct(1), 4);
        assert_eq!(yield_pct(4), 16);
        assert_eq!(growth_pct(4), 12);
        assert_eq!(water_extra_pct(4), 8);
        assert_eq!(seed_return_pct(1), 10);
        assert_eq!(seed_return_pct(4), 40);
    }

    #[test]
    fn yield_bonus_is_floor_of_percentage() {
        assert_eq!(yield_bonus(20, 4), 3); // 20 × 16% = 3.2
        assert_eq!(yield_bonus(4, 4), 0);
        assert_eq!(yield_bonus(100, 2), 8);
    }

    #[test]
    fn harvest_exp_formula_and_cap() {
        let radish = def("luobo"); // unlock 1
        assert_eq!(harvest_exp(&radish, 4, 1), 4); // 2 + 0 + 2
        assert_eq!(harvest_exp(&radish, 4, 40), 4);
        assert_eq!(harvest_exp(&radish, 4, 41), 2);

        let herb = def("renshen"); // unlock 18
        assert_eq!(harvest_exp(&herb, 10, 1), 16); // 2 + 9 + 5
        assert_eq!(harvest_exp(&herb, 999, 1), 20); // 单次上限
    }
}
