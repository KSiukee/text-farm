//! 第一周农场手册：只读现有进度，完成打勾，奖励手动领。
//!
//! 不新增货币，也不按天强制；中后期目标留给玩家自己摸。

use crate::model::GameState;

#[derive(Debug, Clone, Copy)]
pub struct HandbookGoal {
    pub id: &'static str,
    pub title: &'static str,
    pub hint: &'static str,
    pub reward_text: &'static str,
    pub coins: u64,
    pub seeds: &'static [(&'static str, u32)],
}

pub const GOALS: [HandbookGoal; 8] = [
    HandbookGoal {
        id: "first_harvest",
        title: "收下第一轮菜",
        hint: "菜熟了，点收获。",
        reward_text: "金币+50",
        coins: 50,
        seeds: &[],
    },
    HandbookGoal {
        id: "four_plots",
        title: "种下四块地",
        hint: "把四块地都种上，不用一次种完。",
        reward_text: "金币+50 · 小青菜种子+1",
        coins: 50,
        seeds: &[("seed:xiaobaicai", 1)],
    },
    HandbookGoal {
        id: "first_order",
        title: "交第一单",
        hint: "接一单村庄订单，种出来再交付。",
        reward_text: "金币+100",
        coins: 100,
        seeds: &[],
    },
    HandbookGoal {
        id: "fifth_plot",
        title: "开第五块地",
        hint: "农场 3 级后，花 300 金币开地。",
        reward_text: "金币+100 · 萝卜种子+2",
        coins: 100,
        seeds: &[("seed:luobo", 2)],
    },
    HandbookGoal {
        id: "five_neighbors",
        title: "和五个邻居聊过",
        hint: "每天找一个人聊一句，不用刷。",
        reward_text: "金币+100",
        coins: 100,
        seeds: &[],
    },
    HandbookGoal {
        id: "first_help",
        title: "帮邻居一次",
        hint: "浇水、喂牲畜，或者顺手帮个小忙。",
        reward_text: "金币+80",
        coins: 80,
        seeds: &[],
    },
    HandbookGoal {
        id: "first_animal",
        title: "养第一只动物",
        hint: "去畜牧场领一只，慢慢养。",
        reward_text: "金币+150",
        coins: 150,
        seeds: &[],
    },
    HandbookGoal {
        id: "first_craft",
        title: "做出第一件加工品",
        hint: "建制造屋，做一件加工品。",
        reward_text: "金币+200 · 草莓种子+1",
        coins: 200,
        seeds: &[("seed:caomei", 1)],
    },
];

pub fn goal(id: &str) -> Option<&'static HandbookGoal> {
    GOALS.iter().find(|goal| goal.id == id)
}

pub fn is_done(goal: &HandbookGoal, state: &GameState) -> bool {
    match goal.id {
        "first_harvest" => state.stats.harvests >= 1,
        "four_plots" => state.stats.planted >= 4,
        "first_order" => state.stats.orders_delivered >= 1,
        "fifth_plot" => state
            .plots
            .iter()
            .any(|plot| plot.index >= 5 && plot.unlocked),
        "five_neighbors" => {
            state.npcs.len() >= 5 && state.npcs.iter().all(|npc| !npc.last_chat_day.is_empty())
        }
        "first_help" => state.stats.npc_helps >= 1,
        "first_animal" => state.ranch.pens.iter().any(|pen| pen.animal.is_some()),
        "first_craft" => {
            state.workshop.built && state.skills.processing.exp > 0
        }
        _ => false,
    }
}
