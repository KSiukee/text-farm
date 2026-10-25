use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::content::{ContentPack, CropDef};
use crate::model::DailyState;
use crate::ranch::RanchAnimalKind;

pub const STEAL_DAILY_LIMIT: u32 = 5;
pub const HELP_DAILY_LIMIT: u32 = 10;
pub const NPC_VISIT_CAP: usize = 3;
pub const RANCH_STEAL_DAILY_LIMIT: u32 = 5;
pub const RANCH_HELP_DAILY_LIMIT: u32 = 10;
/// 邻居两次串门之间至少隔 4 小时，避免开着游戏被留言刷屏。
pub const NPC_VISIT_GAP_MS: i64 = 4 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcCrop {
    pub crop_id: String,
    pub planted_at: i64,
    pub ready_at: i64,
    pub remaining: u32,
    pub helped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcPlot {
    pub index: u8,
    pub crop: Option<NpcCrop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRanchAnimal {
    pub kind: RanchAnimalKind,
    pub ready_at: i64,
    pub remaining: u32,
    pub helped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRanchAnimalView {
    pub index: u8,
    pub kind: RanchAnimalKind,
    pub label: String,
    pub emoji: String,
    pub product: String,
    pub ready_at: i64,
    pub remaining_ms: i64,
    pub can_help: bool,
    pub can_steal: bool,
    pub helped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcState {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub friendship: i32,
    pub plots: Vec<NpcPlot>,
    #[serde(default)]
    pub ranch: Vec<NpcRanchAnimal>,
    pub last_greeting_at: i64,
    /// 已看过的好感小故事 id。
    #[serde(default)]
    pub seen_events: Vec<String>,
    /// 最近一次聊天的日期（`daily.day`），同一天只加一次好感。
    #[serde(default)]
    pub last_chat_day: String,
    /// 已发生的故事记忆，用于主动想起和人物志。
    #[serde(default)]
    pub memories: Vec<NpcMemory>,
    /// 上一次小请求所属的周（`now / 7 天`）；-1 表示还没帮过。
    #[serde(default)]
    pub request_week: i64,
    #[serde(default)]
    pub request_done: bool,
    /// 小请求完成后留下的小屋纪念物。
    #[serde(default)]
    pub souvenirs: Vec<String>,
    /// 已触发但还没做选择的故事。
    #[serde(default)]
    pub pending_event: String,
    /// 每个故事选过的选项（event id -> choice id）。
    #[serde(default)]
    pub choices: BTreeMap<String, String>,
    /// 上一次送生日礼物的年份（0 = 没送过）。
    #[serde(default)]
    pub birthday_gift_year: i32,
    /// 最近一次随手送礼的日期；每天一次。
    #[serde(default)]
    pub last_gift_day: String,
    /// 最近一次请这位邻居搭把手的日期；每天一次。
    #[serde(default)]
    pub last_help_day: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcMessage {
    pub at: i64,
    pub npc_id: String,
    pub text: String,
    pub from_player: bool,
}

/// 一段发生过的旧事；只存事件 id/日期/季节，不存正文。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcMemory {
    pub event_id: String,
    pub day: i64,
    pub season: String,
    pub last_recall_day: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcPlotView {
    pub index: u8,
    pub label: String,
    pub crop_name: String,
    pub icon: String,
    pub status: String,
    pub ready_at: i64,
    pub remaining: u32,
    pub can_help: bool,
    pub can_steal: bool,
    pub helped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcView {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub friendship: i32,
    pub relation: String,
    pub address: String,
    pub emoji: String,
    pub role: String,
    pub tags: Vec<String>,
    pub likes: String,
    pub dislikes: String,
    pub routine: String,
    pub today_line: String,
    pub today_note: Option<String>,
    pub next_event: Option<String>,
    pub memories: Vec<NpcMemoryView>,
    pub request: Option<NpcRequestView>,
    pub request_done: bool,
    pub last_souvenir: Option<String>,
    pub birthday_gift: Option<NpcBirthdayGiftView>,
    pub free_gift: Option<NpcFreeGiftView>,
    pub help: NpcHelpView,
    pub perks: Vec<NpcPerkView>,
    pub plots: Vec<NpcPlotView>,
    pub ranch: Vec<NpcRanchAnimalView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcHelpView {
    pub can_help: bool,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcPerkView {
    pub threshold: u32,
    pub title: String,
    pub desc: String,
    pub unlocked: bool,
}

pub fn perk_bonus(npcs: &[NpcState], id: &str) -> u32 {
    npcs.iter()
        .find(|npc| npc.id == id)
        .map(|npc| {
            if npc.friendship >= 70 {
                4
            } else if npc.friendship >= 40 {
                2
            } else {
                0
            }
        })
        .unwrap_or(0)
}

pub fn plant_yield_pct(npcs: &[NpcState]) -> u32 {
    perk_bonus(npcs, "ahe")
}

pub fn craft_speed_pct(npcs: &[NpcState]) -> u32 {
    perk_bonus(npcs, "xiaoman")
}

pub fn ranch_quality_pct(npcs: &[NpcState]) -> u32 {
    perk_bonus(npcs, "qingxing")
}

pub fn coin_gain_pct(npcs: &[NpcState]) -> u32 {
    perk_bonus(npcs, "laotao")
}

pub fn fishing_exp_pct(npcs: &[NpcState]) -> u32 {
    perk_bonus(npcs, "liuyue")
}

fn perk_defs(id: &str) -> [(&'static str, &'static str); 2] {
    match id {
        "ahe" => [("厚土", "作物产量 +2%"), ("老把式", "作物产量 +4%")],
        "xiaoman" => [("顺手", "加工速度 +2%"), ("一锅出", "加工速度 +4%")],
        "qingxing" => [("看顾", "畜产品品质 +2%"), ("老方子", "畜产品品质 +4%")],
        "laotao" => [("会算", "卖出收益 +2%"), ("老账本", "卖出收益 +4%")],
        "liuyue" => [("看水", "钓鱼经验 +2%"), ("河路", "钓鱼经验 +4%")],
        _ => [("搭把手", ""), ("老相识", "")],
    }
}

pub fn perks(id: &str, friendship: i32) -> Vec<NpcPerkView> {
    perk_defs(id)
        .into_iter()
        .enumerate()
        .map(|(index, (title, desc))| {
            let threshold = if index == 0 { 40 } else { 70 };
            NpcPerkView {
                threshold,
                title: title.to_string(),
                desc: desc.to_string(),
                unlocked: friendship >= threshold as i32,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcFreeGiftView {
    pub item_name: String,
    pub have: u32,
    pub can_give: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcBirthdayGiftView {
    pub item_name: String,
    pub qty: u32,
    pub have: u32,
    pub can_give: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRequestView {
    pub title: String,
    pub item_name: String,
    pub qty: u32,
    pub have: u32,
    pub can_help: bool,
    pub reason: String,
    pub reward: u64,
    pub souvenir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcMemoryView {
    pub title: String,
    pub summary: String,
    pub season: String,
    pub choice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcChoiceView {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcSouvenirView {
    pub name: String,
    pub icon: String,
}

pub fn souvenir_icon(name: &str) -> &'static str {
    match name {
        "一把扎好的小青菜" => "🥬",
        "一小罐草莓酱" => "🍓",
        "一只晒药筛" => "🌿",
        "一小块打磨过的木料" => "🪵",
        "一枚旧船钉" => "⚓",
        _ => "🎁",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcMessageView {
    pub at: i64,
    pub npc_name: String,
    pub text: String,
    pub from_player: bool,
}

fn help_hint(id: &str) -> &'static str {
    match id {
        "ahe" => "帮你把正在长的地浇一遍",
        "xiaoman" => "今天加工会顺手一点",
        "qingxing" => "帮你看看牲畜",
        "laotao" => "帮你把设备调顺一点",
        "liuyue" => "帮你看看鱼塘",
        _ => "帮你搭把手",
    }
}

pub fn relation(friendship: i32) -> &'static str {    if friendship >= 70 {
        "老友"
    } else if friendship >= 40 {
        "熟识"
    } else if friendship >= 15 {
        "点头之交"
    } else {
        "陌生"
    }
}

fn choose_crop(crops: &[CropDef], level: u32, seed: usize) -> Option<&CropDef> {
    let eligible: Vec<&CropDef> = crops
        .iter()
        .filter(|crop| crop.unlock_level <= level)
        .collect();
    if eligible.is_empty() {
        return None;
    }
    Some(eligible[seed % eligible.len()])
}

fn make_plot(
    crops: &[CropDef],
    level: u32,
    seed: usize,
    now: i64,
    ready_now: bool,
    index: u8,
) -> NpcPlot {
    let Some(def) = choose_crop(crops, level, seed) else {
        return NpcPlot { index, crop: None };
    };
    let grow = i64::from(def.grow_minutes) * 60_000;
    NpcCrop {
        crop_id: def.id.clone(),
        planted_at: now,
        ready_at: if ready_now { now } else { now + grow },
        remaining: def.yield_per_season,
        helped: false,
    }
    .into_plot(index)
}

impl NpcCrop {
    fn into_plot(self, index: u8) -> NpcPlot {
        NpcPlot {
            index,
            crop: Some(self),
        }
    }
}

pub fn starter(now: i64, content: &ContentPack) -> Vec<NpcState> {
    let profiles = [
        ("ahe", "阿禾", 4),
        ("xiaoman", "小满", 8),
        ("qingxing", "青杏", 12),
        ("laotao", "老陶", 18),
        ("liuyue", "六月", 25),
    ];
    profiles
        .iter()
        .enumerate()
        .map(|(npc_index, (id, name, level))| {
            let plots = (0..5)
                .map(|plot_index| {
                    make_plot(
                        &content.crops,
                        *level,
                        npc_index * 7 + plot_index,
                        now,
                        npc_index == 0 && plot_index == 0,
                        (plot_index + 1) as u8,
                    )
                })
                .collect();
            NpcState {
                id: (*id).into(),
                name: (*name).into(),
                level: *level,
                friendship: 10 + npc_index as i32 * 5,
                plots,
                ranch: starter_ranch(now),
                last_greeting_at: 0,
                seen_events: Vec::new(),
                last_chat_day: String::new(),
                memories: Vec::new(),
                request_week: -1,
                request_done: false,
                souvenirs: Vec::new(),
                pending_event: String::new(),
                choices: BTreeMap::new(),
                birthday_gift_year: 0,
                last_gift_day: String::new(),
                last_help_day: String::new(),
            }
        })
        .collect()
}

pub fn starter_ranch(now: i64) -> Vec<NpcRanchAnimal> {
    (0..2)
        .map(|ranch_index| {
            let kind = match ranch_index % 4 {
                0 => RanchAnimalKind::Chicken,
                1 => RanchAnimalKind::Duck,
                2 => RanchAnimalKind::Cow,
                _ => RanchAnimalKind::Sheep,
            };
            NpcRanchAnimal {
                kind,
                ready_at: now + i64::from(ranch_index as u32 + 1) * 3_600_000,
                remaining: 1,
                helped: false,
            }
        })
        .collect()
}

pub fn view(
    npc: &NpcState,
    content: &ContentPack,
    inventory: &std::collections::BTreeMap<String, u32>,
    today: &str,
    farm_days: u32,
    owner_name: &str,
    now: i64,
) -> NpcView {
    let profile = crate::npc_story::profile(&npc.id);
    let (today_line, today_note) = crate::npc_story::line_for(
        &npc.id,
        npc.friendship,
        now / 86_400_000,
        today,
        farm_days,
        owner_name,
    );
    let next_event = crate::npc_story::upcoming_event(&npc.id, &npc.seen_events)
        .map(|event| format!("{} 好感 · {}", event.threshold, event.title));
    let week = now / 86_400_000 / 7;
    let request_done = npc.request_week == week;
    let request = if request_done {
        None
    } else {
        crate::npc_story::request(&npc.id).map(|request| {
            let have = *inventory.get(request.item_key).unwrap_or(&0);
            NpcRequestView {
                title: request.title.to_string(),
                item_name: request.item_name.to_string(),
                qty: request.qty,
                have,
                can_help: have >= request.qty,
                reason: request.reason.to_string(),
                reward: request.reward,
                souvenir: request.souvenir.to_string(),
            }
        })
    };
    let birthday_gift = if crate::npc_story::is_birthday(&npc.id, today)
        && npc.birthday_gift_year != crate::npc_story::year_of(today)
    {
        crate::npc_story::birthday_gift(&npc.id).map(|gift| {
            let have = *inventory.get(gift.item_key).unwrap_or(&0);
            NpcBirthdayGiftView {
                item_name: gift.item_name.to_string(),
                qty: gift.qty,
                have,
                can_give: have >= gift.qty,
            }
        })
    } else {
        None
    };
    let free_gift = if birthday_gift.is_none() && npc.last_gift_day != today {
        crate::npc_story::free_gift(&npc.id).map(|gift| {
            let have = *inventory.get(gift.item_key).unwrap_or(&0);
            NpcFreeGiftView {
                item_name: gift.item_name.to_string(),
                have,
                can_give: have >= gift.qty,
            }
        })
    } else {
        None
    };
    let help = NpcHelpView {
        can_help: npc.last_help_day != today,
        hint: help_hint(&npc.id).to_string(),
    };
    let memories = npc
        .memories
        .iter()
        .filter_map(|memory| {
            let event = crate::npc_story::event_by_id(&memory.event_id)?;
            Some(NpcMemoryView {
                title: event.title.to_string(),
                summary: crate::npc_story::summary(&memory.event_id).to_string(),
                season: memory.season.clone(),
                choice: npc
                    .choices
                    .get(&memory.event_id)
                    .and_then(|choice_id| {
                        crate::npc_story::choice_label(&memory.event_id, choice_id)
                    })
                    .map(|label| label.to_string()),
            })
        })
        .collect();
    let plots = npc
        .plots
        .iter()
        .map(|plot| {
            let def = plot
                .crop
                .as_ref()
                .and_then(|crop| content.crop(&crop.crop_id));
            let crop = plot.crop.as_ref();
            let ready = crop.is_some_and(|c| c.ready_at <= now);
            NpcPlotView {
                index: plot.index,
                label: format!("{}{}", content.nouns.plot, plot.index),
                crop_name: def
                    .map(|d| d.name.clone())
                    .or_else(|| crop.map(|c| c.crop_id.clone()))
                    .unwrap_or_else(|| "空地".into()),
                icon: def.map(|d| d.icon.clone()).unwrap_or_else(|| "🌱".into()),
                status: if crop.is_none() {
                    "empty".into()
                } else if ready {
                    "ready".into()
                } else {
                    "growing".into()
                },
                ready_at: crop.map(|c| c.ready_at).unwrap_or(0),
                remaining: crop.map(|c| c.remaining).unwrap_or(0),
                can_help: crop.is_some_and(|c| !c.helped && c.ready_at > now),
                can_steal: crop.is_some_and(|c| c.ready_at <= now && c.remaining > 1),
                helped: crop.is_some_and(|c| c.helped),
            }
        })
        .collect();

    let ranch = npc
        .ranch
        .iter()
        .enumerate()
        .map(|(index, animal)| NpcRanchAnimalView {
            index: (index + 1) as u8,
            kind: animal.kind,
            label: animal.kind.label().into(),
            emoji: animal.kind.emoji().into(),
            product: animal.kind.product_name().into(),
            ready_at: animal.ready_at,
            remaining_ms: (animal.ready_at - now).max(0),
            can_help: !animal.helped,
            can_steal: animal.ready_at <= now && animal.remaining > 0,
            helped: animal.helped,
        })
        .collect();

    NpcView {
        id: npc.id.clone(),
        name: profile
            .map(|profile| profile.name.to_string())
            .unwrap_or_else(|| npc.name.clone()),
        level: npc.level,
        friendship: npc.friendship,
        relation: relation(npc.friendship).into(),
        address: crate::npc_story::address_for(npc.friendship, owner_name),
        emoji: profile.map(|profile| profile.emoji.to_string()).unwrap_or_default(),
        role: profile.map(|profile| profile.role.to_string()).unwrap_or_default(),
        tags: profile
            .map(|profile| profile.tags.iter().map(|tag| tag.to_string()).collect())
            .unwrap_or_default(),
        likes: profile.map(|profile| profile.likes.to_string()).unwrap_or_default(),
        dislikes: profile.map(|profile| profile.dislikes.to_string()).unwrap_or_default(),
        routine: profile.map(|profile| profile.routine.to_string()).unwrap_or_default(),
        today_line: today_line.to_string(),
        today_note,
        next_event,
        memories,
        request,
        request_done,
        last_souvenir: npc.souvenirs.last().cloned(),
        birthday_gift,
        free_gift,
        help,
        perks: perks(&npc.id, npc.friendship),
        plots,
        ranch,
    }
}

pub fn steal_left(daily: &DailyState) -> u32 {
    STEAL_DAILY_LIMIT.saturating_sub(daily.steals)
}

pub fn help_left(daily: &DailyState) -> u32 {
    HELP_DAILY_LIMIT.saturating_sub(daily.helps)
}

pub fn ranch_steal_left(daily: &DailyState) -> u32 {
    RANCH_STEAL_DAILY_LIMIT.saturating_sub(daily.ranch_steals)
}

pub fn ranch_help_left(daily: &DailyState) -> u32 {
    RANCH_HELP_DAILY_LIMIT.saturating_sub(daily.ranch_helps)
}
