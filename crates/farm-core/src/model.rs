use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::action::FacilityKind;
use crate::buffs::Buff;
use crate::content::ContentPack;
use crate::fishing::FishingState;
use crate::npc::{self, NpcMessage, NpcState};
use crate::pet::{PetCare, PetState};
use crate::ranch::RanchState;
use crate::mystery::MysteryShopState;
use crate::orders::FarmOrderState;
use crate::room::RoomState;
use crate::skills::PlayerSkills;
use crate::workshop::WorkshopState;

pub const SAVE_SCHEMA: u32 = 37;
pub const MAX_LEVEL: u32 = 30;
pub const PLOT_COUNT: usize = 8;
pub const PLOT_MAX_TIER: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub at: i64,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    #[default]
    North,
    South,
}

impl Region {
    pub fn label(self) -> &'static str {
        match self {
            Region::North => "北方温带",
            Region::South => "南方亚热带",
        }
    }

    pub fn off_season_growth(self) -> u32 {
        match self {
            Region::North => 75,
            Region::South => 85,
        }
    }

    pub fn off_season_penalty(self) -> u32 {
        match self {
            Region::North => 10,
            Region::South => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Season {
    #[default]
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn from_month(month: u32) -> Self {
        match month {
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            9..=11 => Season::Autumn,
            _ => Season::Winter,
        }
    }

    pub fn from_today(today: &str) -> Self {
        today
            .get(5..7)
            .and_then(|month| month.parse::<u32>().ok())
            .map(Season::from_month)
            .unwrap_or(Season::Spring)
    }

    pub fn label(self) -> &'static str {
        match self {
            Season::Spring => "春季",
            Season::Summer => "夏季",
            Season::Autumn => "秋季",
            Season::Winter => "冬季",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            Season::Spring => "🌱",
            Season::Summer => "☀️",
            Season::Autumn => "🍂",
            Season::Winter => "❄️",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PestKind {
    Grass,
    Bug,
}

impl PestKind {
    pub fn label(self) -> &'static str {
        match self {
            PestKind::Grass => "草",
            PestKind::Bug => "虫",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pest {
    pub kind: PestKind,
    pub since: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DisasterKind {
    Drought,
    Flood,
    Hail,
    Frost,
    Blight,
    Typhoon,
}

impl DisasterKind {
    pub fn label(self) -> &'static str {
        match self {
            DisasterKind::Drought => "旱灾",
            DisasterKind::Flood => "涝灾",
            DisasterKind::Hail => "冰雹",
            DisasterKind::Frost => "霜冻",
            DisasterKind::Blight => "病害",
            DisasterKind::Typhoon => "台风",
        }
    }

    pub fn yield_penalty(self) -> u32 {
        match self {
            DisasterKind::Drought => 10,
            DisasterKind::Flood => 20,
            DisasterKind::Hail => 0,
            DisasterKind::Frost => 15,
            DisasterKind::Blight => 20,
            DisasterKind::Typhoon => 25,
        }
    }

    /// 生长暂停比例：100 = 完全暂停，50 = 半速。
    pub fn growth_slow(self) -> u32 {
        match self {
            DisasterKind::Drought
            | DisasterKind::Flood
            | DisasterKind::Frost
            | DisasterKind::Typhoon => 100,
            DisasterKind::Blight => 50,
            DisasterKind::Hail => 0,
        }
    }

    pub fn is_instant(self) -> bool {
        matches!(self, DisasterKind::Hail)
    }

    pub fn mitigation_label(self) -> &'static str {
        match self {
            DisasterKind::Drought => "浇水",
            DisasterKind::Flood => "排水",
            DisasterKind::Frost => "覆盖",
            DisasterKind::Blight => "救治",
            DisasterKind::Hail => "—",
            DisasterKind::Typhoon => "加固",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Disaster {
    pub kind: DisasterKind,
    pub since: i64,
    pub ends_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropInstance {
    pub crop_id: String,
    pub planted_at: i64,
    pub ready_at: i64,
    pub season: u8,
    pub seasons: u8,
    pub remaining: u32,
    pub watered: bool,
    #[serde(default)]
    pub pest: Option<Pest>,
    #[serde(default)]
    pub disaster: Option<Disaster>,
    #[serde(default)]
    pub off_season: bool,
    #[serde(default = "default_growth_factor")]
    pub growth_factor: u32,
    #[serde(default)]
    pub greenhouse: bool,
    #[serde(default)]
    pub withered: bool,
}

fn default_growth_factor() -> u32 {
    1000
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlotKind {
    #[default]
    Field,
    Greenhouse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plot {
    pub index: u8,
    pub unlocked: bool,
    #[serde(default)]
    pub tier: u8,
    #[serde(default)]
    pub kind: PlotKind,
    pub crop: Option<CropInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Facilities {
    pub sprinkler: bool,
    pub harvester: bool,
    pub windmill: bool,
    #[serde(default)]
    pub windmills: u8,
    #[serde(default)]
    pub damaged_windmills: u8,
    pub greenhouse: bool,
    #[serde(default)]
    pub greenhouses: u8,
    #[serde(default)]
    pub training_center: bool,
}

impl Facilities {
    pub fn active_windmills(&self) -> u32 {
        u32::from(self.windmills.saturating_sub(self.damaged_windmills))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub exp_rate: u32,
    pub font_scale: u32,
    pub skin: String,
    pub onboarded: bool,
    #[serde(default)]
    pub region: Region,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            exp_rate: 1,
            font_scale: 100,
            skin: "classic".into(),
            onboarded: false,
            region: Region::North,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Stats {
    pub planted: u64,
    pub harvests: u64,
    pub coins_earned: u64,
    pub level_ups: u64,
    pub pests_cleared: u64,
    pub pests_spawned: u64,
    pub items_sold: u64,
    pub sign_ins: u64,
    pub plots_unlocked: u64,
    pub npc_steals: u64,
    pub npc_helps: u64,
    pub npc_visits: u64,
    pub daily_claimed: u64,
    pub seeds_bought: u64,
    pub plot_upgrades: u64,
    pub disasters_spawned: u64,
    pub disasters_mitigated: u64,
    pub specimens_crafted: u64,
    pub rare_seeds_gained: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DailyState {
    pub day: String,
    pub harvests: u32,
    pub waterings: u32,
    pub pests_cleared: u32,
    pub steals: u32,
    pub helps: u32,
    pub ranch_steals: u32,
    pub ranch_helps: u32,
    pub ranch_collects: u32,
    pub fish_catches: u32,
    #[serde(default)]
    pub orders: u32,
    pub sign_in: bool,
    pub claimed_harvest: bool,
    pub claimed_water: bool,
    pub claimed_pest: bool,
    pub claimed_ranch: bool,
    pub claimed_fish: bool,
    #[serde(default)]
    pub claimed_order: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexEntry {
    pub first_at: i64,
    pub total: u64,
    pub best: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub schema: u32,
    pub farm_name: String,
    pub owner_name: String,
    pub level: u32,
    pub exp: u64,
    pub coins: u64,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub rev: u64,
    pub plots: Vec<Plot>,
    pub inventory: BTreeMap<String, u32>,
    pub log: Vec<LogEntry>,
    #[serde(default)]
    pub facilities: Facilities,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub stats: Stats,
    #[serde(default)]
    pub last_tick_at: i64,
    #[serde(default)]
    pub daily: DailyState,
    #[serde(default)]
    pub codex: BTreeMap<String, CodexEntry>,
    #[serde(default)]
    pub achievements: BTreeMap<String, bool>,
    /// 每种作物做过多少份标本；键是 crop id。
    #[serde(default)]
    pub specimen_collection: BTreeMap<String, u32>,
    #[serde(default)]
    pub specimen_milestones_claimed: BTreeSet<u32>,
    #[serde(default)]
    pub npcs: Vec<NpcState>,
    #[serde(default)]
    pub npc_messages: Vec<NpcMessage>,
    #[serde(default)]
    pub last_npc_tick_at: i64,
    #[serde(default)]
    pub season: Season,
    #[serde(default)]
    pub pets: Vec<PetState>,
    #[serde(default)]
    pub pet_care: PetCare,
    #[serde(default)]
    pub pet_seq: u64,
    #[serde(default)]
    pub boarding_debt: u64,
    #[serde(default)]
    pub last_boarding_month: String,
    #[serde(default)]
    pub last_raid_day: String,
    #[serde(default)]
    pub ranch: RanchState,
    #[serde(default)]
    pub workshop: WorkshopState,
    #[serde(default)]
    pub fishing: FishingState,
    /// 玩家侧四专业（种植/畜牧/钓鱼/加工）。宠物有自己的等级，不走这里。
    #[serde(default)]
    pub skills: PlayerSkills,
    /// 临时加成（食物/饮品）；过期由 sync 清掉。
    #[serde(default)]
    pub buffs: Vec<Buff>,
    /// 已领取过奖励的等级；老存档默认为空，达到过的等级可以补领。
    #[serde(default)]
    pub level_rewards_claimed: BTreeSet<u32>,
    /// 自己的小屋：房间布局按房间 id 存；当前只有 living。
    #[serde(default)]
    pub room: RoomState,
    /// 每日村庄订单；旧档默认空，第一次 sync 生成。
    #[serde(default)]
    pub orders: FarmOrderState,
    #[serde(default)]
    pub mystery: MysteryShopState,
}

pub struct Game {
    pub(crate) state: GameState,
    pub(crate) content: ContentPack,
}

impl Game {
    pub fn new(now: i64) -> Self {
        let content = ContentPack::starter();
        let npcs = npc::starter(now, &content);
        let plots = (1..=PLOT_COUNT as u8)
            .map(|index| Plot {
                index,
                unlocked: index <= 4,
                tier: 0,
                kind: PlotKind::Field,
                crop: None,
            })
            .collect();

        let mut inventory = BTreeMap::new();
        inventory.insert("seed:xiaobaicai".to_string(), 3);
        inventory.insert("seed:luobo".to_string(), 2);

        let state = GameState {
            schema: SAVE_SCHEMA,
            farm_name: content.nouns.farm.clone(),
            owner_name: content.nouns.owner.clone(),
            level: 1,
            exp: 0,
            coins: 200,
            created_at: now,
            last_seen_at: now,
            rev: 0,
            plots,
            inventory,
            log: vec![LogEntry {
                at: now,
                text: "你来到了自己的农场。".into(),
            }],
            facilities: Facilities::default(),
            settings: Settings::default(),
            stats: Stats {
                plots_unlocked: 4,
                ..Stats::default()
            },
            last_tick_at: now,
            daily: DailyState::default(),
            codex: BTreeMap::new(),
            achievements: BTreeMap::new(),
            specimen_collection: BTreeMap::new(),
            specimen_milestones_claimed: BTreeSet::new(),
            npcs,
            npc_messages: Vec::new(),
            last_npc_tick_at: now,
            season: Season::Spring,
            pets: Vec::new(),
            pet_care: PetCare::default(),
            pet_seq: 0,
            boarding_debt: 0,
            last_boarding_month: String::new(),
            last_raid_day: String::new(),
            ranch: RanchState::new(),
            workshop: WorkshopState::new(),
            fishing: FishingState::new(),
            skills: PlayerSkills::new(),
            buffs: Vec::new(),
            level_rewards_claimed: BTreeSet::new(),
            room: RoomState::default(),
            orders: FarmOrderState::default(),
            mystery: MysteryShopState::default(),
        };

        Self { state, content }
    }

    pub fn from_parts(mut state: GameState, content: ContentPack) -> Self {
        state.room.ensure_defaults();
        Self { state, content }
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// 调试/测试用可变状态入口；正常玩法仍走 dispatch。
    pub fn state_mut(&mut self) -> &mut GameState {
        &mut self.state
    }

    pub fn content(&self) -> &ContentPack {
        &self.content
    }

    /// 旧档迁移后没有邻居时，按当前内容包补一组；顺手修正历史脏地块编号。
    pub fn ensure_npcs(&mut self, now: i64) {
        if self.state.npcs.is_empty() {
            self.state.npcs = npc::starter(now, &self.content);
            self.state.last_npc_tick_at = now;
        }
        for npc in &mut self.state.npcs {
            if npc.ranch.is_empty() {
                npc.ranch = npc::starter_ranch(now);
            }
            for (index, plot) in npc.plots.iter_mut().enumerate() {
                plot.index = (index + 1) as u8;
            }
        }
    }

    /// 旧档已建温室但没有独立地块时补足；每个温室 3 块地。
    pub fn ensure_greenhouses(&mut self) {
        if self.state.facilities.greenhouses == 0 && self.state.facilities.greenhouse {
            self.state.facilities.greenhouses = 1;
        }
        if self.state.facilities.greenhouses == 0 {
            return;
        }
        self.state.facilities.greenhouse = true;
        let desired = usize::from(self.state.facilities.greenhouses) * 3;
        let mut current = self
            .state
            .plots
            .iter()
            .filter(|plot| plot.kind == PlotKind::Greenhouse)
            .count();
        while current < desired {
            self.state.plots.push(Plot {
                index: (PLOT_COUNT + 1 + current) as u8,
                unlocked: true,
                tier: 0,
                kind: PlotKind::Greenhouse,
                crop: None,
            });
            current += 1;
        }
    }

    /// 旧档补宠物 id，并清理空 id。
    pub fn ensure_ranch(&mut self) {
        self.state.ranch.ensure();
    }

    pub fn ensure_workshop(&mut self) {
        self.state.workshop.ensure();
    }

    pub fn ensure_fishing(&mut self) {
        self.state.fishing.ensure();
    }

    pub fn ensure_pets(&mut self) {
        let mut seq = self.state.pet_seq;
        for pet in &mut self.state.pets {
            if pet.id.is_empty() {
                seq += 1;
                pet.id = format!("pet_{seq}");
            }
        }
        self.state.pet_seq = seq;
    }

    /// 兼容旧调用。
    pub fn ensure_greenhouse(&mut self) {
        self.ensure_greenhouses();
    }

    /// 调试用时间加速：把所有游戏内时间戳整体前移 delta 毫秒。
    ///
    /// 这不是正常玩法入口；调用方负责在之后 `sync()` 并落盘。
    pub fn shift_time(&mut self, delta_ms: i64) {
        if delta_ms <= 0 {
            return;
        }
        let shift = |at: &mut i64| {
            *at = at.saturating_sub(delta_ms);
        };

        for plot in &mut self.state.plots {
            if let Some(crop) = plot.crop.as_mut() {
                shift(&mut crop.planted_at);
                shift(&mut crop.ready_at);
                if let Some(pest) = crop.pest.as_mut() {
                    shift(&mut pest.since);
                }
                if let Some(disaster) = crop.disaster.as_mut() {
                    shift(&mut disaster.since);
                    shift(&mut disaster.ends_at);
                }
            }
        }
        for npc in &mut self.state.npcs {
            shift(&mut npc.last_greeting_at);
            for plot in &mut npc.plots {
                if let Some(crop) = plot.crop.as_mut() {
                    shift(&mut crop.planted_at);
                    shift(&mut crop.ready_at);
                }
            }
        }
        shift(&mut self.state.last_tick_at);
        shift(&mut self.state.last_npc_tick_at);
        shift(&mut self.state.last_seen_at);

        let days = delta_ms / 86_400_000;
        if days > 0 {
            for pet in &mut self.state.pets {
                pet.adopted_at = pet.adopted_at.saturating_sub(delta_ms);
                pet.last_tick_at = pet.last_tick_at.saturating_sub(delta_ms);
                pet.last_pet_at = pet.last_pet_at.saturating_sub(delta_ms);
                pet.last_play_at = pet.last_play_at.saturating_sub(delta_ms);
                pet.last_dig_at = pet.last_dig_at.saturating_sub(delta_ms);
                pet.last_skill_at = pet.last_skill_at.saturating_sub(delta_ms);
                pet.last_sick_day = shift_date_back(&pet.last_sick_day, days);
                pet.last_mischief_day = shift_date_back(&pet.last_mischief_day, days);
                pet.last_activity_day = shift_date_back(&pet.last_activity_day, days);
                pet.last_job_day = shift_date_back(&pet.last_job_day, days);
            }
            for pen in &mut self.state.ranch.pens {
                if let Some(animal) = pen.animal.as_mut() {
                    animal.adopted_at = animal.adopted_at.saturating_sub(delta_ms);
                    animal.last_tick_at = animal.last_tick_at.saturating_sub(delta_ms);
                    animal.product_ready_at = animal.product_ready_at.saturating_sub(delta_ms);
                }
            }
            for job in self.state.workshop.jobs.iter_mut().flatten() {
                match job {
                    crate::workshop::WorkshopJob::Ranch {
                        started_at,
                        ready_at,
                        ..
                    }
                    | crate::workshop::WorkshopJob::CropBox {
                        started_at,
                        ready_at,
                        ..
                    }
                    | crate::workshop::WorkshopJob::Equipment {
                        started_at,
                        ready_at,
                        ..
                    }
                    | crate::workshop::WorkshopJob::Bait {
                        started_at,
                        ready_at,
                        ..
                    }
                    | crate::workshop::WorkshopJob::Fish {
                        started_at,
                        ready_at,
                        ..
                    } => {
                        *started_at = started_at.saturating_sub(delta_ms);
                        *ready_at = ready_at.saturating_sub(delta_ms);
                    }
                }
            }
            self.state.last_raid_day = shift_date_back(&self.state.last_raid_day, days);
            if let Some(session) = self.state.fishing.session.as_mut() {
                shift(&mut session.started_at);
                shift(&mut session.expires_at);
            }
            shift(&mut self.state.fishing.auto.last_at);
            for entry in self.state.fishing.codex.values_mut() {
                shift(&mut entry.first_at);
            }
            if let Some(catch) = self.state.fishing.last_catch.as_mut() {
                shift(&mut catch.at);
            }
            if let Some(report) = self.state.fishing.last_auto_report.as_mut() {
                shift(&mut report.at);
            }
            if !self.state.last_boarding_month.is_empty() {
                let first = format!("{}-01", self.state.last_boarding_month);
                self.state.last_boarding_month = shift_date(&first, days)
                    .get(0..7)
                    .unwrap_or(&self.state.last_boarding_month)
                    .to_string();
            }
        }
        if delta_ms >= 24 * 60 * 60 * 1000 {
            self.state.daily = DailyState::default();
        }
        self.state.rev += 1;
    }
}

fn shift_date(date: &str, days: i64) -> String {
    if date.is_empty() {
        return String::new();
    }
    let mut parts = date.split("-");
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return date.to_string();
    };
    let (Ok(mut year), Ok(mut month), Ok(mut day)) = (
        year.parse::<i32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return date.to_string();
    };
    for _ in 0..days.max(0) {
        day += 1;
        if day > days_in_month(year, month) {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    format!("{year:04}-{month:02}-{day:02}")
}

fn shift_date_back(date: &str, days: i64) -> String {
    if date.is_empty() {
        return String::new();
    }
    let mut parts = date.split("-");
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return date.to_string();
    };
    let (Ok(mut year), Ok(mut month), Ok(mut day)) = (
        year.parse::<i32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return date.to_string();
    };
    for _ in 0..days.max(0) {
        if day > 1 {
            day -= 1;
        } else {
            if month > 1 {
                month -= 1;
            } else {
                month = 12;
                year -= 1;
            }
            day = days_in_month(year, month);
        }
    }
    format!("{year:04}-{month:02}-{day:02}")
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 30,
    }
}

/// 本级升到下一级需要的经验；满级返回 0。
pub fn level_need(level: u32) -> u64 {
    const TABLE: [u64; 30] = [
        30, 84, 156, 240, 336, 555, 680, 810, 948, 1460, 1662, 1874, 2096, 2324, 2560, 2804, 3054,
        3312, 3578, 4813, 5160, 5515, 5880, 6250, 6630, 7015, 7408, 7808, 8215, 0,
    ];
    if level == 0 {
        return TABLE[0];
    }
    if level >= MAX_LEVEL {
        return 0;
    }
    TABLE[(level - 1) as usize]
}

pub fn greenhouse_tier(tier: u8) -> PlotTier {
    match tier {
        1 => PlotTier {
            name: "温室·改良",
            exp_tenths: 40,
            coin_tenths: 40,
            yield_pct: 10,
            luck_pct: 2,
            upgrade_cost: 8_000,
            upgrade_level: 23,
        },
        2 => PlotTier {
            name: "温室·恒温",
            exp_tenths: 60,
            coin_tenths: 60,
            yield_pct: 15,
            luck_pct: 3,
            upgrade_cost: 40_000,
            upgrade_level: 26,
        },
        3 => PlotTier {
            name: "温室·智能",
            exp_tenths: 80,
            coin_tenths: 80,
            yield_pct: 20,
            luck_pct: 4,
            upgrade_cost: 0,
            upgrade_level: 99,
        },
        _ => PlotTier {
            name: "温室",
            exp_tenths: 20,
            coin_tenths: 20,
            yield_pct: 5,
            luck_pct: 1,
            upgrade_cost: 1_000,
            upgrade_level: 20,
        },
    }
}

pub fn greenhouse_growth_factor(tier: u8) -> u32 {
    match tier.min(3) {
        0 => 950,
        1 => 940,
        2 => 920,
        _ => 900,
    }
}

pub fn greenhouse_off_multiplier(tier: u8) -> u32 {
    match tier.min(3) {
        0 => 80,
        1 => 60,
        2 => 45,
        _ => 30,
    }
}

pub fn greenhouse_wither_chance(tier: u8) -> u32 {
    match tier.min(3) {
        0 => 25,
        1 => 21,
        2 => 17,
        _ => 13,
    }
}

pub fn plot_unlock_level(index: u8) -> u32 {
    match index {
        1..=4 => 1,
        5 => 3,
        6 => 6,
        7 => 12,
        8 => 21,
        _ => 1,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlotTier {
    pub name: &'static str,
    /// 经验加成，单位 0.1%：50 = 5%，85 = 8.5%。
    pub exp_tenths: u32,
    /// 金币加成，单位 0.1%。
    pub coin_tenths: u32,
    pub yield_pct: u32,
    pub luck_pct: u32,
    /// 升到下一档需要的金币；满级为 0。
    pub upgrade_cost: u64,
    /// 升到下一档需要的农场等级；满级为 99。
    pub upgrade_level: u32,
}

pub fn plot_tier(tier: u8) -> PlotTier {
    match tier {
        1 => PlotTier {
            name: "肥沃",
            exp_tenths: 50,
            coin_tenths: 50,
            yield_pct: 10,
            luck_pct: 2,
            upgrade_cost: 30_000,
            upgrade_level: 12,
        },
        2 => PlotTier {
            name: "红土",
            exp_tenths: 85,
            coin_tenths: 85,
            yield_pct: 15,
            luck_pct: 4,
            upgrade_cost: 200_000,
            upgrade_level: 21,
        },
        3 => PlotTier {
            name: "黑土",
            exp_tenths: 100,
            coin_tenths: 100,
            yield_pct: 20,
            luck_pct: 6,
            upgrade_cost: 0,
            upgrade_level: 99,
        },
        _ => PlotTier {
            name: "普通",
            exp_tenths: 0,
            coin_tenths: 0,
            yield_pct: 0,
            luck_pct: 0,
            upgrade_cost: 2_000,
            upgrade_level: 5,
        },
    }
}

pub fn plot_unlock_cost(index: u8) -> u64 {
    match index {
        1..=4 => 0,
        5 => 300,
        6 => 1_200,
        7 => 5_000,
        8 => 30_000,
        _ => 0,
    }
}

pub fn facility_name(kind: FacilityKind) -> &'static str {
    match kind {
        FacilityKind::Sprinkler => "灌溉机",
        FacilityKind::Harvester => "收割机",
        FacilityKind::Windmill => "风车",
        FacilityKind::Greenhouse => "温室",
        FacilityKind::TrainingCenter => "训练中心",
    }
}

pub fn facility_unlock_level(kind: FacilityKind) -> u32 {
    match kind {
        FacilityKind::Sprinkler => 8,
        FacilityKind::Harvester => 12,
        FacilityKind::Windmill => 15,
        FacilityKind::Greenhouse => 20,
        FacilityKind::TrainingCenter => 12,
    }
}

pub fn facility_cost(kind: FacilityKind) -> u64 {
    match kind {
        FacilityKind::Sprinkler => 2_000,
        FacilityKind::Harvester => 8_000,
        FacilityKind::Windmill => 15_000,
        FacilityKind::Greenhouse => 60_000,
        FacilityKind::TrainingCenter => 20_000,
    }
}

pub fn facility_max_count(kind: FacilityKind) -> u32 {
    match kind {
        FacilityKind::Windmill => 15,
        FacilityKind::Greenhouse => 3,
        _ => 1,
    }
}

pub fn facility_next_cost(kind: FacilityKind, count: u32) -> u64 {
    match kind {
        FacilityKind::Windmill => 15_000 + u64::from(count) * 5_000,
        FacilityKind::Greenhouse => 60_000 + u64::from(count) * 30_000,
        _ => facility_cost(kind),
    }
}
