use serde::Serialize;

use crate::model::DailyState;

/// 日常任务的开放条件：牧场任务要等牧场里有动物才显示，
/// 免得刚开局就摆一条做不了的任务。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyUnlock {
    Always,
    Ranch,
}

#[derive(Debug, Clone, Copy)]
pub struct DailyTaskDef {
    pub id: u8,
    pub name: &'static str,
    pub desc: &'static str,
    pub target: u32,
    pub reward_coins: u64,
    pub reward_exp: u32,
    pub unlock: DailyUnlock,
}

pub const DAILY_TASKS: [DailyTaskDef; 6] = [
    DailyTaskDef {
        id: 1,
        name: "收获 10 次",
        desc: "今天累计收获 10 次",
        target: 10,
        reward_coins: 50,
        reward_exp: 30,
        unlock: DailyUnlock::Always,
    },
    DailyTaskDef {
        id: 2,
        name: "浇水 5 次",
        desc: "今天累计浇水 5 次（灌溉机自动浇水也算）",
        target: 5,
        reward_coins: 20,
        reward_exp: 15,
        unlock: DailyUnlock::Always,
    },
    DailyTaskDef {
        id: 3,
        name: "清理 3 次草虫",
        desc: "今天累计清理 3 次草虫",
        target: 3,
        reward_coins: 30,
        reward_exp: 20,
        unlock: DailyUnlock::Always,
    },
    DailyTaskDef {
        id: 4,
        name: "牧场收 3 次",
        desc: "今天累计收取 3 次牧场产品",
        target: 3,
        reward_coins: 60,
        reward_exp: 35,
        unlock: DailyUnlock::Ranch,
    },
    DailyTaskDef {
        id: 5,
        name: "钓上 3 条鱼",
        desc: "今天累计钓上 3 条鱼（主动和自动都算）",
        target: 3,
        reward_coins: 50,
        reward_exp: 30,
        unlock: DailyUnlock::Always,
    },
    DailyTaskDef {
        id: 6,
        name: "交 1 笔村庄订单",
        desc: "今天累计交付 1 笔村庄订单",
        target: 1,
        reward_coins: 40,
        reward_exp: 25,
        unlock: DailyUnlock::Always,
    },
];

pub fn daily_progress(daily: &DailyState, id: u8) -> u32 {
    match id {
        1 => daily.harvests,
        2 => daily.waterings,
        3 => daily.pests_cleared,
        4 => daily.ranch_collects,
        5 => daily.fish_catches,
        6 => daily.orders,
        _ => 0,
    }
}

pub fn daily_claimed(daily: &DailyState, id: u8) -> bool {
    match id {
        1 => daily.claimed_harvest,
        2 => daily.claimed_water,
        3 => daily.claimed_pest,
        4 => daily.claimed_ranch,
        5 => daily.claimed_fish,
        6 => daily.claimed_order,
        _ => false,
    }
}

pub fn mark_claimed(daily: &mut DailyState, id: u8) {
    match id {
        1 => daily.claimed_harvest = true,
        2 => daily.claimed_water = true,
        3 => daily.claimed_pest = true,
        4 => daily.claimed_ranch = true,
        5 => daily.claimed_fish = true,
        6 => daily.claimed_order = true,
        _ => {}
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyTaskView {
    pub id: u8,
    pub name: String,
    pub desc: String,
    pub progress: u32,
    pub target: u32,
    pub claimed: bool,
    pub claimable: bool,
    pub reward_coins: u64,
    pub reward_exp: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyView {
    pub day: String,
    pub sign_in: bool,
    pub tasks: Vec<DailyTaskView>,
}

pub fn view(daily: &DailyState, ranch_available: bool) -> DailyView {
    DailyView {
        day: daily.day.clone(),
        sign_in: daily.sign_in,
        tasks: DAILY_TASKS
            .iter()
            .filter(|task| match task.unlock {
                DailyUnlock::Always => true,
                DailyUnlock::Ranch => ranch_available,
            })
            .map(|task| {
                let progress = daily_progress(daily, task.id).min(task.target);
                let claimed = daily_claimed(daily, task.id);
                DailyTaskView {
                    id: task.id,
                    name: task.name.into(),
                    desc: task.desc.into(),
                    progress,
                    target: task.target,
                    claimed,
                    claimable: !claimed && progress >= task.target,
                    reward_coins: task.reward_coins,
                    reward_exp: task.reward_exp,
                }
            })
            .collect(),
    }
}
