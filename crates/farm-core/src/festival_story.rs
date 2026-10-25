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

pub const STORIES: [FestivalStoryDef; 10] = [
    FestivalStoryDef {
        festival: "播种节",
        title: "第一把种子",
        text: "春分前后，土刚化开。\
阿禾蹲在田埂上捏了一把土，说今年墒情好，可以下种了。\
他把一小袋种子倒在你手心，说第一把要自己撒。",
        choices: &[
            FestivalChoiceDef {
                id: "sow_with_ahe",
                label: "跟阿禾下地",
                hint: "他教你怎么把种子撒匀",
                memory: "播种节跟阿禾下了第一块地，他说手要低，种子才落得匀。",
                coins: 60,
                item_key: None,
                item_qty: 0,
                npc_id: Some("ahe"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "raise_at_home",
                label: "自己在家育苗",
                hint: "在窗边摆一排小盆",
                memory: "播种节在窗边育了一排小苗，每天看一眼，长得比想象中慢。",
                coins: 100,
                item_key: Some("seed:xiaobaicai"),
                item_qty: 2,
                npc_id: None,
                favor: 0,
                contribution: 0,
            },
        ],
    },
    FestivalStoryDef {
        festival: "凉夏宴",
        title: "井里的瓜",
        text: "夏至这天，日头最长。\
六月把一只西瓜吊进井里，说等傍晚再提上来。\
小满在灶边熬果酱，锅里的草莓翻着细小的泡。",
        choices: &[
            FestivalChoiceDef {
                id: "river_melon",
                label: "和六月去河边冰瓜",
                hint: "河水比井水凉",
                memory: "凉夏宴和六月把瓜浸在河湾里，傍晚切开，还是温的。",
                coins: 60,
                item_key: None,
                item_qty: 0,
                npc_id: Some("liuyue"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "jam_with_xiaoman",
                label: "帮小满做冰果酱",
                hint: "她一个人搅不过来",
                memory: "凉夏宴帮小满搅了一锅果酱，她说过几天就能抹面包。",
                coins: 100,
                item_key: Some("seed:caomei"),
                item_qty: 1,
                npc_id: Some("xiaoman"),
                favor: 3,
                contribution: 0,
            },
        ],
    },
    FestivalStoryDef {
        festival: "丰收节",
        title: "最好的一篮",
        text: "秋分，村里在晒场摆开一溜竹篮。\
阿禾说今年不评谁家最大，只评谁家最用心。\
你的仓库里也有几样拿得出手的东西。",
        choices: &[
            FestivalChoiceDef {
                id: "contest_basket",
                label: "交一篮菜去评比",
                hint: "不一定要最多，要挑得干净",
                memory: "丰收节交了一篮菜去评比，阿禾看了半天，说这篮有样子。",
                coins: 150,
                item_key: None,
                item_qty: 0,
                npc_id: None,
                favor: 0,
                contribution: 3,
            },
            FestivalChoiceDef {
                id: "guard_granary",
                label: "陪阿禾守谷仓",
                hint: "最后一车谷子还没入库",
                memory: "丰收节陪阿禾守到天黑，最后一车谷子进了仓，他才坐下喝水。",
                coins: 80,
                item_key: None,
                item_qty: 0,
                npc_id: Some("ahe"),
                favor: 5,
                contribution: 0,
            },
        ],
    },
    FestivalStoryDef {
        festival: "围炉日",
        title: "一锅热汤",
        text: "冬至，天黑得早。\
青杏把药柜后面的小桌支起来，说今天不诊脉，只喝汤。\
阿禾提了一篮菜，小满端来米糕，老陶抱着一捆柴，六月拎着两条鱼。",
        choices: &[
            FestivalChoiceDef {
                id: "qingxing_table",
                label: "去青杏家喝汤",
                hint: "五个人挤一张小桌",
                memory: "围炉日去青杏家喝了汤，五个人挤一张小桌，谁也没先走。",
                coins: 80,
                item_key: None,
                item_qty: 0,
                npc_id: Some("qingxing"),
                favor: 5,
                contribution: 0,
            },
            FestivalChoiceDef {
                id: "stay_farm_pot",
                label: "在农场支一口锅",
                hint: "把这些年攒的东西都放一点",
                memory: "围炉日在农场支了一口锅，汤滚了很久，屋里的灯一直亮着。",
                coins: 120,
                item_key: Some("seed:luobo"),
                item_qty: 1,
                npc_id: None,
                favor: 0,
                contribution: 0,
            },
        ],
    },
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
