//! 每周牧场评比：提交一只动物，按品质/代数/特性/健康评分，和 5 个 NPC 排名次。
//!
//! 不参加不损失；每周只能报一只；奖章累计进牧场图鉴。

use serde::{Deserialize, Serialize};

use crate::ranch::{RanchAnimal, RanchTrait};

pub const RANCH_CONTEST_NPCS: usize = 5;
/// 名次奖励：第 1/2/3 名（金币、奖章）。
pub const RANCH_CONTEST_REWARDS: [(u64, u32); 3] = [(3_000, 3), (1_500, 2), (800, 1)];
/// 未进前三的参与奖。
pub const RANCH_CONTEST_OTHER_REWARD: (u64, u32) = (200, 0);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchContestResult {
    pub week: String,
    pub animal_name: String,
    pub kind_label: String,
    pub score: u32,
    pub rank: u32,
    pub reward_coins: u64,
    pub medal: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RanchContestState {
    /// 本周周一（YYYY-MM-DD）。
    pub week: String,
    pub entered: bool,
    pub result: Option<RanchContestResult>,
    /// 累计奖章。
    pub medals: u32,
    /// 历史最好名次；0 = 还没参加过。
    pub best_rank: u32,
}

impl RanchContestState {
    /// 跨周重置报名与本周结果（奖章保留）。返回是否换了周。
    pub fn ensure_week(&mut self, week: &str) -> bool {
        if self.week == week {
            return false;
        }
        self.week = week.to_string();
        self.entered = false;
        self.result = None;
        true
    }
}

/// 评分：品质 0/30/60 + 代数（封顶 5 代）0–50 + 健康 0–10 + 特性 8–12；近亲 −20。
pub fn score(animal: &RanchAnimal) -> u32 {
    let mut value = u32::from(animal.quality.clamp(1, 3).saturating_sub(1)) * 30;
    value += animal.generation.min(5) * 10;
    value += animal.health / 10;
    value += animal.temper.contest_score();
    if animal.inbred {
        value = value.saturating_sub(20);
    }
    value
}

/// 本周 5 个 NPC 的分数（40–125），按周确定性生成。
pub fn npc_scores(week: &str) -> [u32; RANCH_CONTEST_NPCS] {
    std::array::from_fn(|index| 40 + (roll(week, 0x900 + index as u64 * 17) % 86) as u32)
}

/// 名次 = 1 + 分数高于你的 NPC 数（1–6）。
pub fn rank_for(week: &str, player_score: u32) -> u32 {
    1 + npc_scores(week)
        .iter()
        .filter(|npc| **npc > player_score)
        .count() as u32
}

pub fn reward_for(rank: u32) -> (u64, u32) {
    match rank {
        1..=3 => RANCH_CONTEST_REWARDS[(rank - 1) as usize],
        _ => RANCH_CONTEST_OTHER_REWARD,
    }
}

fn roll(day: &str, salt: u64) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ salt;
    for byte in day.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^ (hash >> 33)
}

impl RanchTrait {
    /// 评比里的特性分：偏生产的特性分高；娇气的品质加成也算分。
    pub fn contest_score(self) -> u32 {
        match self {
            RanchTrait::HighYield => 12,
            RanchTrait::Efficient => 10,
            RanchTrait::Hardy => 8,
            RanchTrait::Delicate => 12,
            RanchTrait::TwinYield => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranch::RanchAnimalKind;

    fn animal(quality: u8, generation: u32) -> RanchAnimal {
        let mut animal = RanchAnimal::new(RanchAnimalKind::Chicken, 0);
        animal.quality = quality;
        animal.generation = generation;
        animal.health = 100;
        animal
    }

    #[test]
    fn score_uses_quality_generation_health_and_trait() {
        let mut animal = animal(3, 3);
        animal.temper = RanchTrait::HighYield;
        assert_eq!(score(&animal), 60 + 30 + 10 + 12);
        animal.inbred = true;
        assert_eq!(score(&animal), 112 - 20);
        animal.generation = 9;
        assert_eq!(score(&animal), 60 + 50 + 10 + 12 - 20);
    }

    #[test]
    fn rank_and_rewards_follow_npc_scores() {
        let week = "2026-10-19";
        let scores = npc_scores(week);
        assert_eq!(scores.len(), RANCH_CONTEST_NPCS);
        assert!(scores.iter().all(|score| (40..=125).contains(score)));
        let worst = *scores.iter().max().unwrap();
        let best = *scores.iter().min().unwrap();
        assert_eq!(rank_for(week, worst + 1), 1);
        assert_eq!(rank_for(week, best.saturating_sub(1)), 6);
        assert_eq!(reward_for(1), (3_000, 3));
        assert_eq!(reward_for(3), (800, 1));
        assert_eq!(reward_for(4), (200, 0));
    }

    #[test]
    fn week_change_resets_entry_but_keeps_medals() {
        let mut state = RanchContestState::default();
        assert!(state.ensure_week("2026-10-19"));
        state.entered = true;
        state.medals = 4;
        state.best_rank = 2;
        assert!(!state.ensure_week("2026-10-19"));
        assert!(state.ensure_week("2026-10-26"));
        assert!(!state.entered);
        assert!(state.result.is_none());
        assert_eq!(state.medals, 4);
        assert_eq!(state.best_rank, 2);
    }
}
