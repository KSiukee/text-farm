//! NPC 的人物档案、每日一句话与好感小故事。
//!
//! 只放静态文案与选择规则，不碰存档；状态在 `NpcState` 里。

#[derive(Debug, Clone)]
pub struct NpcProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub role: &'static str,
    pub tags: &'static [&'static str],
    pub birthday: &'static str,
    pub likes: &'static str,
    pub dislikes: &'static str,
    pub routine: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct NpcEvent {
    pub id: &'static str,
    pub threshold: i32,
    pub title: &'static str,
    pub text: &'static str,
    pub reward: u64,
    pub favor: i32,
}

const PROFILES: &[NpcProfile] = &[
    NpcProfile {
        id: "ahe",
        name: "阿禾",
        emoji: "🌾",
        role: "村东的菜农",
        tags: &["慢热", "手巧", "嘴硬心软"],
        birthday: "04-12",
        likes: "小青菜、清晨的露水",
        dislikes: "空口客套",
        routine: "天不亮下地，午后在田埂上修农具",
    },
    NpcProfile {
        id: "xiaoman",
        name: "小满",
        emoji: "🍓",
        role: "做果酱的姑娘",
        tags: &["爱笑", "急性子", "记性好"],
        birthday: "06-05",
        likes: "草莓、蜂蜜、热闹",
        dislikes: "糟蹋食物",
        routine: "上午熬果酱，下午去看蜂箱",
    },
    NpcProfile {
        id: "qingxing",
        name: "青杏",
        emoji: "🌿",
        role: "村里的赤脚医生",
        tags: &["温和", "较真", "夜猫子"],
        birthday: "09-18",
        likes: "药草、安静的夜",
        dislikes: "逞强不吃药",
        routine: "白天晒药，夜里整理药柜",
    },
    NpcProfile {
        id: "laotao",
        name: "老陶",
        emoji: "🪚",
        role: "老木匠",
        tags: &["嘴碎", "手稳", "念旧"],
        birthday: "11-02",
        likes: "好木料、浓茶",
        dislikes: "毛躁的年轻人",
        routine: "在院里刨木头，傍晚坐在门口",
    },
    NpcProfile {
        id: "liuyue",
        name: "六月",
        emoji: "🌙",
        role: "看天吃饭的渔人",
        tags: &["寡言", "心细", "认路"],
        birthday: "07-21",
        likes: "河风、月亮、鲫鱼",
        dislikes: "浪费鱼获",
        routine: "清晨收网，下午补网",
    },
];

const AHE_LOW: &[&str] = &[
    "早啊。田里露水重，你鞋别湿了。",
    "这批菜苗长得慢，不过根扎得稳。",
    "锄头又松了……等会儿我自己紧一紧。",
];
const AHE_HIGH: &[&str] = &[
    "你来得正好，帮我看看这垄是不是该间苗了。",
    "我留了两棵最好的小青菜，等会儿给你带走。",
    "以前我爹说，地不会骗人。你信吗？",
];
const XIAOMAN_LOW: &[&str] = &[
    "来得巧，果酱刚出锅，尝一勺？",
    "今天蜂箱那边有点吵，大概要分群了。",
    "你别看我笑，我熬糊过三锅。",
];
const XIAOMAN_HIGH: &[&str] = &[
    "我给你留了一小罐，别告诉别人啊。",
    "你说，草莓是不是天生就该做果酱？",
    "等忙完这阵，我想把摊子摆到村口去。",
];
const QINGXING_LOW: &[&str] = &[
    "先坐，手伸出来我看看。",
    "夜里凉，回去把窗关上。",
    "药得按时吃，别嫌苦。",
];
const QINGXING_HIGH: &[&str] = &[
    "你今天气色好多了，别逞强。",
    "老陶的咳嗽总不好，我还在试方子。",
    "有空的话，帮我晒一筛子药草。",
];
const LAOTAO_LOW: &[&str] = &[
    "木头有脾气，你得顺着它。",
    "这刨子跟我三十年了，比人听话。",
    "年轻人，别急着下锯。",
];
const LAOTAO_HIGH: &[&str] = &[
    "坐，喝茶。别嫌浓。",
    "我年轻时也想过出去，后来还是留下了。",
    "你手稳，要不要试试刨这块板？",
];
const LIUYUE_LOW: &[&str] = &[
    "风向不对，今天鱼口浅。",
    "网补好了，明早能下。",
    "河上雾大，走路看着点。",
];
const LIUYUE_HIGH: &[&str] = &[
    "月亮好的时候，鱼会浮上来。",
    "这条鲫鱼给你，拿回去炖汤。",
    "船老了，划起来倒还顺手。",
];

const AHE_EVENTS: &[NpcEvent] = &[
    NpcEvent {
        id: "ahe_hoe",
        threshold: 30,
        title: "老锄头",
        text: "阿禾把那把旧锄头递给你，木柄被磨得发亮。「我爹留下的，柄松了。你帮我扶一下，我紧一紧。」修好后他试了试，嘴角松下来。",
        reward: 300,
        favor: 4,
    },
    NpcEvent {
        id: "ahe_field",
        threshold: 60,
        title: "田埂上的话",
        text: "收工后阿禾坐在田埂上，忽然说：「以前我总觉得，种地是没本事的人才留下的。后来我爹走了，地还在。你来了以后，我才觉得这块地有点意思。」",
        reward: 600,
        favor: 5,
    },
];
const XIAOMAN_EVENTS: &[NpcEvent] = &[
    NpcEvent {
        id: "xiaoman_jam",
        threshold: 30,
        title: "第一锅果酱",
        text: "小满从柜子最里面摸出一罐颜色发暗的果酱。「第一锅，糊了。我一直没扔，提醒自己别再糟蹋东西。」她挖了一勺递给你，「这次不糊，你尝。」",
        reward: 300,
        favor: 4,
    },
    NpcEvent {
        id: "xiaoman_bees",
        threshold: 60,
        title: "蜂箱",
        text: "小满戴着手套，让你听蜂箱里的声音。「听见没？它们在商量搬家。」她笑了一下，「我以前怕蜂，现在觉得它们跟我挺像，忙忙碌碌，就想把日子过甜一点。」",
        reward: 600,
        favor: 5,
    },
];
const QINGXING_EVENTS: &[NpcEvent] = &[
    NpcEvent {
        id: "qingxing_lamp",
        threshold: 30,
        title: "夜里的灯",
        text: "青杏把一盏小灯推到你面前。「夜里出诊，路不好走。这灯借你，下次来还我。」她顿了顿，「别嫌我啰嗦，你们这些人，总把身体当别人的。」",
        reward: 300,
        favor: 4,
    },
    NpcEvent {
        id: "qingxing_cabinet",
        threshold: 60,
        title: "药柜",
        text: "青杏打开药柜，里面每一格都贴着细小的字。「老陶的咳嗽，我配了七版。」她轻轻合上柜门，「当医生最怕的不是治不好，是人家不把自己的命当回事。你替我盯着他点。」",
        reward: 600,
        favor: 5,
    },
];
const LAOTAO_EVENTS: &[NpcEvent] = &[
    NpcEvent {
        id: "laotao_chair",
        threshold: 30,
        title: "没送出的椅子",
        text: "老陶指着墙角一把小椅子。「给我闺女做的，她嫁到外村，一直没来拿。」他用袖子擦了擦椅背，「你要是不嫌弃，先拿去坐。木头放着也是放着。」",
        reward: 300,
        favor: 4,
    },
    NpcEvent {
        id: "laotao_apprentice",
        threshold: 60,
        title: "徒弟",
        text: "老陶把刨子塞到你手里。「我收过两个徒弟，都走了。手艺这东西，留不住人。」他看着你推出一层薄薄的木花，「你手稳。要是愿意，以后常来。」",
        reward: 600,
        favor: 5,
    },
];
const LIUYUE_EVENTS: &[NpcEvent] = &[
    NpcEvent {
        id: "liuyue_boat",
        threshold: 30,
        title: "漏水的船",
        text: "六月蹲在船边，拿一块旧布堵住裂缝。「补了三次，还是漏。」她抬头看你，「你帮我扶着，我把这块板换上。」船修好后，她把一条鲫鱼塞给你。",
        reward: 300,
        favor: 4,
    },
    NpcEvent {
        id: "liuyue_moon",
        threshold: 60,
        title: "月亮",
        text: "月亮升起来的时候，六月把船停在河心。「我爹说，看月亮能知道明天有没有鱼。」她沉默了一会儿，「他走后，我还是每晚都看。」",
        reward: 600,
        favor: 5,
    },
];

pub fn profile(id: &str) -> Option<&'static NpcProfile> {
    PROFILES.iter().find(|profile| profile.id == id)
}

fn lines_for(id: &str, high: bool) -> &'static [&'static str] {
    match (id, high) {
        ("ahe", true) => AHE_HIGH,
        ("ahe", false) => AHE_LOW,
        ("xiaoman", true) => XIAOMAN_HIGH,
        ("xiaoman", false) => XIAOMAN_LOW,
        ("qingxing", true) => QINGXING_HIGH,
        ("qingxing", false) => QINGXING_LOW,
        ("laotao", true) => LAOTAO_HIGH,
        ("laotao", false) => LAOTAO_LOW,
        ("liuyue", true) => LIUYUE_HIGH,
        ("liuyue", false) => LIUYUE_LOW,
        _ => &[],
    }
}

/// 每日一句话：同一天同一人固定，好感高了换一套更亲近的说法。
pub fn daily_line(id: &str, friendship: i32, day: i64) -> &'static str {
    let lines = lines_for(id, friendship >= 40);
    if lines.is_empty() {
        return "今天也在忙。";
    }
    let index = day.rem_euclid(lines.len() as i64) as usize;
    lines[index]
}

pub fn events(id: &str) -> &'static [NpcEvent] {
    match id {
        "ahe" => AHE_EVENTS,
        "xiaoman" => XIAOMAN_EVENTS,
        "qingxing" => QINGXING_EVENTS,
        "laotao" => LAOTAO_EVENTS,
        "liuyue" => LIUYUE_EVENTS,
        _ => &[],
    }
}

pub fn event_by_id(event_id: &str) -> Option<&'static NpcEvent> {
    PROFILES
        .iter()
        .flat_map(|profile| events(profile.id))
        .find(|event| event.id == event_id)
}

/// 给人看的一行摘要（人物志用）。
pub fn summary(event_id: &str) -> &'static str {
    match event_id {
        "ahe_hoe" => "你帮他修好了父亲留下的旧锄头",
        "ahe_field" => "他第一次说起父亲和这块地",
        "xiaoman_jam" => "你尝了她第一锅没糊的果酱",
        "xiaoman_bees" => "你陪她听了蜂箱里的动静",
        "qingxing_lamp" => "她把出诊的小灯借给了你",
        "qingxing_cabinet" => "她让你帮忙盯着老陶的咳嗽",
        "laotao_chair" => "你坐上了他没送出去的小椅子",
        "laotao_apprentice" => "他让你试着刨了一块板",
        "liuyue_boat" => "你帮她换好了漏水的船板",
        "liuyue_moon" => "她和你说起父亲看月亮的习惯",
        _ => "一件小事",
    }
}

/// 主动想起时的一句旧话；不展示原文，只提一个细节。
pub fn recall_line(event_id: &str) -> &'static str {
    match event_id {
        "ahe_hoe" => "上次你帮我紧的那把锄头，今年使着顺手。",
        "ahe_field" => "田埂上说的那些话，我还记着。",
        "xiaoman_jam" => "那罐糊掉的果酱，我还留着。",
        "xiaoman_bees" => "蜂箱今年又分群了，我想起你那天听蜂的样子。",
        "qingxing_lamp" => "那盏灯还在你那儿吧？夜里走路当心。",
        "qingxing_cabinet" => "老陶的咳嗽好多了，上次托你的事，我没忘。",
        "laotao_chair" => "那把椅子你坐着还稳吗？",
        "laotao_apprentice" => "你上回推的那层木花，薄得正好。",
        "liuyue_boat" => "船今年没再漏，上次你扶的那块板还牢。",
        "liuyue_moon" => "今晚月亮好，我想起你上次在船上说的话。",
        _ => "上次那件事，我还记得。",
    }
}

/// 日期字符串（`YYYY-MM-DD`）对应的季节；解析失败时返回“未知”。
pub fn season_of(day: &str) -> &'static str {
    let month = day
        .split('-')
        .nth(1)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    match month {
        3..=5 => "春",
        6..=8 => "夏",
        9..=11 => "秋",
        12 | 1 | 2 => "冬",
        _ => "未知",
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NpcRequest {
    pub title: &'static str,
    pub item_key: &'static str,
    pub item_name: &'static str,
    pub qty: u32,
    pub reason: &'static str,
    pub reward: u64,
    pub souvenir: &'static str,
}

/// 每周一个小请求：要的东西和理由都跟这个人当前的心事有关。
pub fn request(id: &str) -> Option<NpcRequest> {
    match id {
        "ahe" => Some(NpcRequest {
            title: "给青杏带点新鲜菜",
            item_key: "crop:xiaobaicai",
            item_name: "小青菜",
            qty: 5,
            reason: "青杏说最近村里咳嗽的人多，他想送点新鲜菜过去。",
            reward: 200,
            souvenir: "一把扎好的小青菜",
        }),
        "xiaoman" => Some(NpcRequest {
            title: "熬一锅新果酱",
            item_key: "crop:caomei",
            item_name: "草莓",
            qty: 5,
            reason: "她想试一锅不加糖的果酱，缺一篮熟透的草莓。",
            reward: 200,
            souvenir: "一小罐草莓酱",
        }),
        "qingxing" => Some(NpcRequest {
            title: "给老陶熬润肺的汤",
            item_key: "crop:luobo",
            item_name: "萝卜",
            qty: 4,
            reason: "老陶咳了半个月，她想用萝卜和蜂蜜试个方子。",
            reward: 250,
            souvenir: "一只晒药筛",
        }),
        "laotao" => Some(NpcRequest {
            title: "给椅子做个垫子",
            item_key: "ranch:wool",
            item_name: "羊毛",
            qty: 2,
            reason: "那把没人坐的小椅子，他想垫软一点。",
            reward: 300,
            souvenir: "一小块打磨过的木料",
        }),
        "liuyue" => Some(NpcRequest {
            title: "修船前喝碗鱼汤",
            item_key: "fish:crucian",
            item_name: "鲫鱼",
            qty: 2,
            reason: "船板换好了，她想先喝碗热汤再下水。",
            reward: 250,
            souvenir: "一枚旧船钉",
        }),
        _ => None,
    }
}

const FESTIVALS: [(&str, &str); 4] = [
    ("03-20", "播种节"),
    ("06-21", "凉夏宴"),
    ("09-23", "丰收节"),
    ("12-21", "围炉日"),
];

fn month_day(day: &str) -> &str {
    day.get(5..).unwrap_or("")
}

pub fn festival_of(day: &str) -> Option<&'static str> {
    let md = month_day(day);
    FESTIVALS
        .iter()
        .find(|(date, _)| *date == md)
        .map(|(_, name)| *name)
}

fn birthday_line(id: &str) -> &'static str {
    match id {
        "ahe" => "今天是我生日。没什么讲究，你能来坐坐就好。",
        "xiaoman" => "今天我生日！我给自己留了块最大的蛋糕。",
        "qingxing" => "今天我生日。别送药，送把青菜就好。",
        "laotao" => "今天我生日。又老一岁，刨子倒还趁手。",
        "liuyue" => "今天我生日。给自己放了半天假。",
        _ => "今天是我生日。",
    }
}

fn festival_line(id: &str, festival: &str) -> &'static str {
    match (id, festival) {
        ("ahe", "播种节") => "播种节。今天下种，土是暖的。",
        ("ahe", "凉夏宴") => "凉夏宴。井里镇着瓜，等会儿切。",
        ("ahe", "丰收节") => "丰收节。今年这垄地，没白忙。",
        ("ahe", "围炉日") => "围炉日。地里没活，手倒闲不住。",
        ("xiaoman", "播种节") => "播种节我熬了锅甜粥，等会儿给你盛一碗。",
        ("xiaoman", "凉夏宴") => "凉夏宴我做了冰果酱，酸甜的。",
        ("xiaoman", "丰收节") => "丰收节我把最甜的一篮留出来了。",
        ("xiaoman", "围炉日") => "围炉日我煮了果茶，给你留了位子。",
        ("qingxing", "播种节") => "播种节别光顾着下地，晌午记得歇。",
        ("qingxing", "凉夏宴") => "凉夏宴别贪凉，晚上还要盖薄被。",
        ("qingxing", "丰收节") => "丰收节家家都忙，我得多备点药。",
        ("qingxing", "围炉日") => "围炉日别坐太久，起来走动走动。",
        ("laotao", "播种节") => "播种节我给锄头换了个新柄。",
        ("laotao", "凉夏宴") => "凉夏宴我在门口支了张桌子，来坐。",
        ("laotao", "丰收节") => "丰收节木匠也歇一天，喝口茶。",
        ("laotao", "围炉日") => "围炉日火旺，来烤烤手。",
        ("liuyue", "播种节") => "播种节河风软，鱼也肯开口。",
        ("liuyue", "凉夏宴") => "凉夏宴船上有风，比岸上凉快。",
        ("liuyue", "丰收节") => "丰收节河里的鱼肥，今晚有汤。",
        ("liuyue", "围炉日") => "围炉日河面起雾，船就不出了。",
        _ => "今天是个好日子。",
    }
}

fn anniversary_line(id: &str, days: u32) -> String {
    match id {
        "ahe" => format!("你来农场第 {days} 天了。地里的菜，也算认识你了。"),
        "xiaoman" => format!("你来农场第 {days} 天了。我给你留了罐果酱。"),
        "qingxing" => format!("你来农场第 {days} 天了。气色比刚来的时候好。"),
        "laotao" => format!("你来农场第 {days} 天了。时间过得比刨花还快。"),
        "liuyue" => format!("你来农场第 {days} 天了。河还是那条河，你倒像常客了。"),
        _ => format!("你来农场第 {days} 天了。"),
    }
}

fn other_birthday_note(id: &str) -> &'static str {
    match id {
        "ahe" => "我早上给她送了两棵青菜。",
        "xiaoman" => "我留了一小块果酱蛋糕给她。",
        "qingxing" => "我提醒她今天别熬太晚。",
        "laotao" => "我给她削了个小木勺。",
        "liuyue" => "我把最大的那条鱼留给她。",
        _ => "我去送了点东西。",
    }
}

fn cross_line(id: &str, day: i64) -> (&'static str, &'static str) {
    let pick = (day / 5).rem_euclid(2) as usize;
    match id {
        "ahe" => [
            ("青杏", "青杏今早来拿了把青菜，说给老陶熬汤。"),
            ("小满", "小满的果酱香，隔着一条田埂都闻得到。"),
        ][pick],
        "xiaoman" => [
            ("老陶", "老陶给我修了个木架，我给了他一罐果酱当工钱。"),
            ("六月", "六月今早从河边过，给我带了条鲫鱼。"),
        ][pick],
        "qingxing" => [
            ("阿禾", "阿禾送来的菜还带着露水，我分了一半给老陶。"),
            ("小满", "小满熬果酱熬到半夜，我让她早点睡。"),
        ][pick],
        "laotao" => [
            ("六月", "六月那船修好了，我给她削了个新桨柄。"),
            ("阿禾", "阿禾的锄头又松了，他宁可自己敲。"),
        ][pick],
        "liuyue" => [
            ("青杏", "青杏托我留意河边的药草。"),
            ("小满", "小满说要做鱼汤，我给她留了两条鲫鱼。"),
        ][pick],
        _ => ("村里", "村里今天很安静。"),
    }
}

fn is_anniversary(days: u32) -> bool {
    days == 30 || days == 100 || (days >= 365 && days % 365 == 0)
}

/// 今天对这个 NPC 有没有特别意义：自己生日 > 节日 > 农场纪念日 > 别人生日。
pub fn special_note(id: &str, day: &str, farm_days: u32) -> Option<(String, String)> {
    let profile = profile(id)?;
    let md = month_day(day);
    if profile.birthday == md {
        return Some(("生日".to_string(), birthday_line(id).to_string()));
    }
    if let Some(festival) = festival_of(day) {
        return Some((
            festival.to_string(),
            festival_line(id, festival).to_string(),
        ));
    }
    if is_anniversary(farm_days) {
        return Some((
            format!("来农场第 {farm_days} 天"),
            anniversary_line(id, farm_days),
        ));
    }
    if let Some(other) = PROFILES
        .iter()
        .find(|other| other.id != id && other.birthday == md)
    {
        return Some((
            format!("今天是{}生日", other.name),
            format!("今天是{}生日。{}", other.name, other_birthday_note(id)),
        ));
    }
    None
}

/// 卡片与聊天共用的一句话：特别日子 > 偶尔提起别人 > 日常台词。
pub fn line_for(
    id: &str,
    friendship: i32,
    day: i64,
    today: &str,
    farm_days: u32,
) -> (String, Option<String>) {
    if let Some((tag, line)) = special_note(id, today, farm_days) {
        return (line, Some(tag));
    }
    if day > 0 && day.rem_euclid(5) == 0 {
        let (other, line) = cross_line(id, day);
        return (line.to_string(), Some(format!("提到{other}")));
    }
    (daily_line(id, friendship, day).to_string(), None)
}

pub fn next_event(
    id: &str,
    friendship: i32,
    seen: &[String],
) -> Option<&'static NpcEvent> {
    events(id)
        .iter()
        .find(|event| friendship >= event.threshold && !seen.iter().any(|seen| seen == event.id))
}

/// 还没看过的下一段故事（不管好感够不够），用于界面提示。
pub fn upcoming_event(id: &str, seen: &[String]) -> Option<&'static NpcEvent> {
    events(id)
        .iter()
        .find(|event| !seen.iter().any(|seen| seen == event.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_line_is_stable_for_same_day() {
        assert_eq!(daily_line("ahe", 10, 42), daily_line("ahe", 10, 42));
        assert_ne!(daily_line("ahe", 10, 42), daily_line("ahe", 10, 43));
    }

    #[test]
    fn high_friendship_uses_different_pool() {
        let low = (0..30)
            .map(|day| daily_line("xiaoman", 10, day))
            .collect::<Vec<_>>();
        let high = (0..30)
            .map(|day| daily_line("xiaoman", 60, day))
            .collect::<Vec<_>>();
        assert!(low.iter().all(|line| !high.contains(line)));
    }

    #[test]
    fn next_event_respects_threshold_and_seen() {
        let seen = vec!["ahe_hoe".to_string()];
        let next = next_event("ahe", 60, &seen).unwrap();
        assert_eq!(next.id, "ahe_field");
        assert!(next_event("ahe", 29, &[]).is_none());
        assert!(next_event("ahe", 60, &seen[..]).is_some());
    }

    #[test]
    fn special_days_have_priority_and_lines() {
        let (tag, line) = special_note("ahe", "2026-04-12", 1).unwrap();
        assert_eq!(tag, "生日");
        assert!(line.contains("生日"));

        let (tag, _) = special_note("ahe", "2026-09-23", 1).unwrap();
        assert_eq!(tag, "丰收节");

        let (tag, line) = special_note("ahe", "2026-10-25", 30).unwrap();
        assert_eq!(tag, "来农场第 30 天");
        assert!(line.contains("第 30 天"));

        let (tag, _) = special_note("ahe", "2026-06-05", 1).unwrap();
        assert_eq!(tag, "今天是小满生日");
    }

    #[test]
    fn line_for_mentions_other_villagers_on_some_days() {
        let (line, tag) = line_for("ahe", 35, 10, "2026-10-25", 1);
        assert_eq!(tag.as_deref(), Some("提到青杏"));
        assert!(line.contains("青杏"));
    }
}
