//! 节日专属小剧情：一年一次，二选一，选完记进节日回忆。
//!
//! 不新增货币；奖励只有金币/种子/好感/集市贡献。

use serde::Serialize;

use crate::model::GameState;

#[derive(Debug, Clone, Copy)]
pub struct FestivalStoryDef {
    pub festival: &'static str,
    pub title: &'static str,
    pub text: &'static str,
    pub choices: &'static [FestivalChoiceDef],
}

#[derive(Debug, Clone, Copy)]
pub struct FestivalChoiceDef {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub memory: &'static str,
    pub coins: u64,
    pub item_key: Option<&'static str>,
    pub item_qty: u32,
    pub npc_id: Option<&'static str>,
    pub favor: i32,
    pub contribution: u32,
}

pub const STORIES: [FestivalStoryDef; 3] = [
    FestivalStoryDef {
        festival: "春节",
        title: "年夜饭",
        text: "下午阿禾就来喊你，说今年人多，桌子支在院子里。\
小满端着一屉米糕从巷口过来，老陶把最后一盏灯挂上。\
天还没黑，村里已经有人放起了零星的炮仗。",
        choices: &[
            FestivalChoiceDef {
                id: "ahe_dinner",
                label: "去阿禾家吃年夜饭",
                hint: "和阿禾、村里人一起守岁",
                memory: "在阿禾家的院子里吃了年夜饭，桌上有米糕和热汤。",
                coins: 100,
                item_key: None,
                item_qty: 0,
                npc_id: Some("ahe"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "stay_farm",
                label: "留在农场守岁",
                hint: "把灯留着，给自己煮一碗面",
                memory: "一个人守在农场，把院里的灯留到了很晚。",
                coins: 150,
                item_key: Some("seed:caomei"),
                item_qty: 1,
                npc_id: None,
                favor: 0,
                contribution: 0,
            },
        ],
    },
    FestivalStoryDef {
        festival: "元宵节",
        title: "河边的灯",
        text: "天刚擦黑，河边就亮起来了。\
六月把一艘小纸船放进水里，船头压着一小截蜡烛。\
岸上有人挂灯谜，风一吹，红纸簌簌地响。",
        choices: &[
            FestivalChoiceDef {
                id: "riddle",
                label: "去猜灯谜",
                hint: "猜中了摊主会给一份小礼",
                memory: "在灯会上猜中了一条灯谜，摊主多送了一把萝卜种子。",
                coins: 120,
                item_key: Some("seed:luobo"),
                item_qty: 2,
                npc_id: None,
                favor: 0,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "river_lantern",
                label: "陪六月放河灯",
                hint: "把愿放在水上",
                memory: "和六月在河边放了一盏灯，看着它慢慢漂远。",
                coins: 50,
                item_key: None,
                item_qty: 0,
                npc_id: Some("liuyue"),
                favor: 5,
                contribution: 0,
            },
        ],
    },
    FestivalStoryDef {
        festival: "国庆节",
        title: "晒谷场上的旗",
        text: "晒谷场今天扫得干干净净，旗杆底下围了一圈人。\
老陶搬来梯子，说这旗要挂正，偏一点都不行。\
远处有人试锣鼓，敲两下，又停下来笑。",
        choices: &[
            FestivalChoiceDef {
                id: "laotao_story",
                label: "听老陶讲年轻时的事",
                hint: "他难得开口",
                memory: "在晒谷场听老陶讲了年轻时修桥的事，他说那桥现在还在。",
                coins: 80,
                item_key: None,
                item_qty: 0,
                npc_id: Some("laotao"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "help_flag",
                label: "帮村里挂旗",
                hint: "梯子有点晃，扶一把",
                memory: "帮村里把旗挂上了，风一吹，整面旗都展开。",
                coins: 120,
                item_key: None,
                item_qty: 0,
                npc_id: None,
                favor: 0,
                contribution: 3,
            },
        ],
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FestivalStoryView {
    pub festival: String,
    pub title: String,
    pub text: String,
    pub choices: Vec<FestivalChoiceView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FestivalChoiceView {
    pub id: String,
    pub label: String,
    pub hint: String,
}

pub fn story(festival: &str) -> Option<&'static FestivalStoryDef> {
    STORIES.iter().find(|story| story.festival == festival)
}

pub fn story_key(year: i32, festival: &str) -> String {
    format!("{year}:{festival}")
}

pub fn view(state: &GameState) -> Option<FestivalStoryView> {
    let festival = crate::npc_story::festival_of(&state.daily.day)?;
    let story = story(festival)?;
    let key = story_key(crate::npc_story::year_of(&state.daily.day), festival);
    if state.festival_stories.contains_key(&key) {
        return None;
    }
    Some(FestivalStoryView {
        festival: festival.to_string(),
        title: story.title.to_string(),
        text: story.text.to_string(),
        choices: story
            .choices
            .iter()
            .map(|choice| FestivalChoiceView {
                id: choice.id.to_string(),
                label: choice.label.to_string(),
                hint: choice.hint.to_string(),
            })
            .collect(),
    })
}
