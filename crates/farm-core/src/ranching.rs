//! 畜牧专业的专精效果与经验公式。

const QUALITY_CHANCE: [u32; 4] = [10, 20, 30, 40];
const YIELD_CHANCE: [u32; 4] = [8, 16, 24, 32];
const FEED_SAVE_CHANCE: [u32; 4] = [8, 16, 24, 32];
const CARE_PCT: [u32; 4] = [10, 20, 30, 40];

fn at(level: u32, table: &[u32; 4]) -> u32 {
    if level == 0 {
        0
    } else {
        table[(level - 1).min(3) as usize]
    }
}

/// 优育：畜产品品质 +1 的概率。
pub fn quality_chance(level: u32) -> u32 {
    at(level, &QUALITY_CHANCE)
}

/// 增产：收牧产额外 +1 的概率。
pub fn yield_chance(level: u32) -> u32 {
    at(level, &YIELD_CHANCE)
}

/// 省料：喂食不消耗饲料的概率。
pub fn feed_save_chance(level: u32) -> u32 {
    at(level, &FEED_SAVE_CHANCE)
}

/// 照料：喂食/治疗效果的加成百分比。
pub fn care_pct(level: u32) -> u32 {
    at(level, &CARE_PCT)
}

/// 收一次牧产的经验：`15 + 品质×8`（品质 1–3 → 23/31/39）。
pub fn collect_exp(quality: u8) -> u64 {
    15 + u64::from(quality.clamp(1, 3)) * 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perk_tables_scale_one_to_four() {
        assert_eq!(quality_chance(0), 0);
        assert_eq!(quality_chance(1), 10);
        assert_eq!(quality_chance(4), 40);
        assert_eq!(yield_chance(4), 32);
        assert_eq!(feed_save_chance(4), 32);
        assert_eq!(care_pct(1), 10);
        assert_eq!(care_pct(4), 40);
    }

    #[test]
    fn collect_exp_scales_with_quality() {
        assert_eq!(collect_exp(1), 23);
        assert_eq!(collect_exp(2), 31);
        assert_eq!(collect_exp(3), 39);
        assert_eq!(collect_exp(9), 39); // 越界按精品算
    }
}
