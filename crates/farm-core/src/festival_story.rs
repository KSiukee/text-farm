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

pub const STORIES: [FestivalStoryDef; 6] = [
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
    FestivalStoryDef {
        festival: "清明节",
        title: "田埂上的青团",
        text: "清明前后，田埂上冒出新艾。\
青杏挎着竹篮从坡上下来，篮里一半是艾草，一半是刚挖的春笋。\
她说今天的雨不大，正好走一走。",
        choices: &[
            FestivalChoiceDef {
                id: "walk_ridge",
                label: "陪青杏去田埂采艾",
                hint: "她走得很慢，像在等谁",
                memory: "清明和青杏沿着田埂采了一篮艾草，她说春笋晚上炖汤。",
                coins: 60,
                item_key: None,
                item_qty: 0,
                npc_id: Some("qingxing"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "make_qingtuan",
                label: "在家做一屉青团",
                hint: "把春天揉进糯米里",
                memory: "清明在家蒸了一屉青团，给阿禾和小满各送了几个。",
                coins: 120,
                item_key: Some("seed:xiaobaicai"),
                item_qty: 1,
                npc_id: None,
                favor: 0,
                contribution: 0,
            },
        ],
    },
    FestivalStoryDef {
        festival: "端午节",
        title: "河边的粽子",
        text: "端午前一天，小满就把糯米泡上了。\
六月蹲在河边洗粽叶，水从指缝里流过去。\
上游传来试锣鼓的声音，龙舟还没到，岸上已经站了人。",
        choices: &[
            FestivalChoiceDef {
                id: "wrap_zongzi",
                label: "帮小满包粽子",
                hint: "甜的咸的各包一挂",
                memory: "端午帮小满包了两挂粽子，她往你手里塞了一个刚出锅的。",
                coins: 80,
                item_key: None,
                item_qty: 0,
                npc_id: Some("xiaoman"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "watch_boat",
                label: "去河边看龙舟",
                hint: "六月在岸边给你留了位置",
                memory: "端午在河边看龙舟从上游下来，六月说今年的鼓点比去年急。",
                coins: 120,
                item_key: None,
                item_qty: 0,
                npc_id: Some("liuyue"),
                favor: 3,
                contribution: 0,
            },
        ],
    },
    FestivalStoryDef {
        festival: "中秋节",
        title: "晒场上的月亮",
        text: "中秋的月亮升得早，晒场上的谷子已经收拢成一座座小丘。\
小满把月饼切成一角一角，阿禾搬来几把椅子，说今晚不急着收工。\
风里有新稻草的味道。",
        choices: &[
            FestivalChoiceDef {
                id: "moon_cake",
                label: "和小满分月饼",
                hint: "她留了最甜的那一块",
                memory: "中秋和小满在晒场上分月饼，她把最甜的一块推到你面前。",
                coins: 100,
                item_key: None,
                item_qty: 0,
                npc_id: Some("xiaoman"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "stay_harvest",
                label: "陪阿禾守晒场",
                hint: "谷子还没盖好",
                memory: "中秋陪阿禾守到月亮偏西，把最后一块油布盖在谷子上。",
                coins: 80,
                item_key: None,
                item_qty: 0,
                npc_id: Some("ahe"),
                favor: 5,
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
