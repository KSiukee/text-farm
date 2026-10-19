use serde::Serialize;

use crate::model::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Planted,
    Harvests,
    CoinsEarned,
    Level,
    PlotsUnlocked,
    PestsCleared,
    ItemsSold,
    SignIns,
    FacilitiesOwned,
}

#[derive(Debug, Clone, Copy)]
pub struct AchievementDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub target: u64,
    pub reward_coins: u64,
    pub reward_exp: u32,
    pub metric: Metric,
}

pub const ACHIEVEMENTS: [AchievementDef; 24] = [
    AchievementDef {
        id: "first_plant",
        name: "第一粒种子",
        desc: "第一次播种",
        target: 1,
        reward_coins: 20,
        reward_exp: 2,
        metric: Metric::Planted,
    },
    AchievementDef {
        id: "first_harvest",
        name: "第一次收获",
        desc: "第一次把作物收进仓库",
        target: 1,
        reward_coins: 30,
        reward_exp: 3,
        metric: Metric::Harvests,
    },
    AchievementDef {
        id: "harvest_10",
        name: "小有收成",
        desc: "累计收获 10 次",
        target: 10,
        reward_coins: 100,
        reward_exp: 10,
        metric: Metric::Harvests,
    },
    AchievementDef {
        id: "harvest_100",
        name: "百次收获",
        desc: "累计收获 100 次",
        target: 100,
        reward_coins: 500,
        reward_exp: 50,
        metric: Metric::Harvests,
    },
    AchievementDef {
        id: "harvest_1000",
        name: "千次收获",
        desc: "累计收获 1000 次",
        target: 1000,
        reward_coins: 3_000,
        reward_exp: 300,
        metric: Metric::Harvests,
    },
    AchievementDef {
        id: "coins_1000",
        name: "千金",
        desc: "靠卖果实累计赚到 1000 金币",
        target: 1_000,
        reward_coins: 100,
        reward_exp: 10,
        metric: Metric::CoinsEarned,
    },
    AchievementDef {
        id: "coins_10000",
        name: "万金",
        desc: "靠卖果实累计赚到 10000 金币",
        target: 10_000,
        reward_coins: 1_000,
        reward_exp: 100,
        metric: Metric::CoinsEarned,
    },
    AchievementDef {
        id: "coins_100000",
        name: "十万金",
        desc: "靠卖果实累计赚到 100000 金币",
        target: 100_000,
        reward_coins: 10_000,
        reward_exp: 1_000,
        metric: Metric::CoinsEarned,
    },
    AchievementDef {
        id: "level_5",
        name: "五级农场",
        desc: "农场达到 5 级",
        target: 5,
        reward_coins: 200,
        reward_exp: 20,
        metric: Metric::Level,
    },
    AchievementDef {
        id: "level_10",
        name: "十级农场",
        desc: "农场达到 10 级",
        target: 10,
        reward_coins: 500,
        reward_exp: 50,
        metric: Metric::Level,
    },
    AchievementDef {
        id: "level_20",
        name: "二十级农场",
        desc: "农场达到 20 级",
        target: 20,
        reward_coins: 2_000,
        reward_exp: 200,
        metric: Metric::Level,
    },
    AchievementDef {
        id: "level_30",
        name: "满级农场",
        desc: "农场达到 30 级",
        target: 30,
        reward_coins: 10_000,
        reward_exp: 1_000,
        metric: Metric::Level,
    },
    AchievementDef {
        id: "plots_5",
        name: "五块地",
        desc: "开垦到 5 块地",
        target: 5,
        reward_coins: 300,
        reward_exp: 30,
        metric: Metric::PlotsUnlocked,
    },
    AchievementDef {
        id: "plots_8",
        name: "八块地",
        desc: "开垦到 8 块地",
        target: 8,
        reward_coins: 3_000,
        reward_exp: 300,
        metric: Metric::PlotsUnlocked,
    },
    AchievementDef {
        id: "pests_10",
        name: "除草小能手",
        desc: "清理 10 次草虫",
        target: 10,
        reward_coins: 300,
        reward_exp: 30,
        metric: Metric::PestsCleared,
    },
    AchievementDef {
        id: "pests_100",
        name: "田园卫士",
        desc: "清理 100 次草虫",
        target: 100,
        reward_coins: 2_000,
        reward_exp: 200,
        metric: Metric::PestsCleared,
    },
    AchievementDef {
        id: "planted_100",
        name: "播种百次",
        desc: "累计播种 100 次",
        target: 100,
        reward_coins: 500,
        reward_exp: 50,
        metric: Metric::Planted,
    },
    AchievementDef {
        id: "planted_1000",
        name: "播种千次",
        desc: "累计播种 1000 次",
        target: 1_000,
        reward_coins: 3_000,
        reward_exp: 300,
        metric: Metric::Planted,
    },
    AchievementDef {
        id: "sold_100",
        name: "卖果百个",
        desc: "累计卖出 100 个果实",
        target: 100,
        reward_coins: 300,
        reward_exp: 30,
        metric: Metric::ItemsSold,
    },
    AchievementDef {
        id: "sold_1000",
        name: "卖果千个",
        desc: "累计卖出 1000 个果实",
        target: 1_000,
        reward_coins: 3_000,
        reward_exp: 300,
        metric: Metric::ItemsSold,
    },
    AchievementDef {
        id: "facility_1",
        name: "第一件设施",
        desc: "建设任意一件设施",
        target: 1,
        reward_coins: 200,
        reward_exp: 20,
        metric: Metric::FacilitiesOwned,
    },
    AchievementDef {
        id: "facility_2",
        name: "两件设施",
        desc: "建设两件设施",
        target: 2,
        reward_coins: 500,
        reward_exp: 50,
        metric: Metric::FacilitiesOwned,
    },
    AchievementDef {
        id: "facility_3",
        name: "设施齐全",
        desc: "三件设施全部建设",
        target: 3,
        reward_coins: 1_000,
        reward_exp: 100,
        metric: Metric::FacilitiesOwned,
    },
    AchievementDef {
        id: "sign_in_7",
        name: "签到七天",
        desc: "累计签到 7 天",
        target: 7,
        reward_coins: 300,
        reward_exp: 30,
        metric: Metric::SignIns,
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementView {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub progress: u64,
    pub target: u64,
    pub earned: bool,
    pub reward_coins: u64,
    pub reward_exp: u32,
}

pub fn progress(state: &GameState, def: &AchievementDef) -> u64 {
    match def.metric {
        Metric::Planted => state.stats.planted,
        Metric::Harvests => state.stats.harvests,
        Metric::CoinsEarned => state.stats.coins_earned,
        Metric::Level => u64::from(state.level),
        Metric::PlotsUnlocked => state.stats.plots_unlocked,
        Metric::PestsCleared => state.stats.pests_cleared,
        Metric::ItemsSold => state.stats.items_sold,
        Metric::SignIns => state.stats.sign_ins,
        Metric::FacilitiesOwned => {
            u64::from(state.facilities.sprinkler)
                + u64::from(state.facilities.harvester)
                + u64::from(state.facilities.windmills > 0)
                + u64::from(state.facilities.greenhouses > 0)
        }
    }
}

pub fn views(state: &GameState) -> Vec<AchievementView> {
    ACHIEVEMENTS
        .iter()
        .map(|def| AchievementView {
            id: def.id.into(),
            name: def.name.into(),
            desc: def.desc.into(),
            progress: progress(state, def).min(def.target),
            target: def.target,
            earned: state.achievements.get(def.id).copied().unwrap_or(false),
            reward_coins: def.reward_coins,
            reward_exp: def.reward_exp,
        })
        .collect()
}
