//! 加工专业的专精效果与经验公式。

const YIELD_CHANCE: [u32; 4] = [8, 16, 24, 32];
const QUALITY_CHANCE: [u32; 4] = [10, 20, 30, 40];
const SPEED_PCT: [u32; 4] = [4, 8, 12, 16];
const SAVE_CHANCE: [u32; 4] = [8, 16, 24, 32];

fn at(level: u32, table: &[u32; 4]) -> u32 {
    if level == 0 {
        0
    } else {
        table[(level - 1).min(3) as usize]
    }
}

/// 出品：取出成品时额外 +1 的概率。
pub fn yield_chance(level: u32) -> u32 {
    at(level, &YIELD_CHANCE)
}

/// 精制：牧场加工品品质 +1 的概率（只对有品质的加工品生效）。
pub fn quality_chance(level: u32) -> u32 {
    at(level, &QUALITY_CHANCE)
}

/// 熟练：加工工时缩短百分比。
pub fn speed_pct(level: u32) -> u32 {
    at(level, &SPEED_PCT)
}

/// 省料：开工时返还 1 份主料的概率。
pub fn save_chance(level: u32) -> u32 {
    at(level, &SAVE_CHANCE)
}

/// 取一次成品的经验：`10 + 品质×8`（无品质的加工按普通算，18）。
pub fn collect_exp(quality: u8) -> u64 {
    10 + u64::from(quality.clamp(1, 3)) * 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perk_tables_scale_one_to_four() {
        assert_eq!(yield_chance(0), 0);
        assert_eq!(yield_chance(1), 8);
        assert_eq!(yield_chance(4), 32);
        assert_eq!(quality_chance(4), 40);
        assert_eq!(speed_pct(4), 16);
        assert_eq!(save_chance(4), 32);
    }

    #[test]
    fn collect_exp_scales_with_quality() {
        assert_eq!(collect_exp(1), 18);
        assert_eq!(collect_exp(2), 26);
        assert_eq!(collect_exp(3), 34);
        assert_eq!(collect_exp(0), 18);
    }
}
