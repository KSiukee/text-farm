use serde::Serialize;

use crate::achievements::{self, AchievementView};
use crate::action::FacilityKind;
use crate::content::ContentPack;
use crate::daily::{self, DailyView};
use crate::fishing::{self, FishRarity, FishingSpot, RodKind};
use crate::level_reward::{self, LevelRewardView};
use crate::model::{
    facility_name, facility_unlock_level, greenhouse_tier, level_need, plot_tier, plot_unlock_cost,
    plot_unlock_level, DisasterKind, Game, GameState, LogEntry, PestKind, PlotKind, Region, Season,
    PLOT_MAX_TIER,
};
use crate::npc::{self, NpcMessageView, NpcView};
use crate::pet::{
    PetBreed, PetFoodKind, PetJob, PetLocation, PetSpecies, PetStage, PetState, PetSupplyKind,
    PetTalent,
    PET_BOARD_FEE_PER_MONTH, PET_PLAY_COOLDOWN_MS, PET_TOUCH_COOLDOWN_MS,
};
use crate::error::GameError;
use crate::ranch::{RanchAnimalKind, RanchRecipe, RanchTrait, RANCH_MAX_PENS};
use crate::specimen;
use crate::workshop::{
    craft_item, crop_box_price, EquipmentKind, EquipmentRecipe, WorkshopJob, CROP_BOX_INPUT,
    CROP_BOX_INTERVAL_MS, WORKSHOP_COST, WORKSHOP_EQUIPMENT_MAX_LEVEL, WORKSHOP_LEVEL,
    WORKSHOP_MAX_SLOTS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlotStatus {
    Growing,
    Ready,
    Withered,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PestView {
    pub kind: PestKind,
    pub label: String,
    pub since: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisasterView {
    pub kind: DisasterKind,
    pub label: String,
    pub since: i64,
    pub ends_at: i64,
    pub yield_penalty: u32,
    pub growth_slow: u32,
    pub mitigation: String,
    pub can_mitigate: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CropView {
    pub crop_id: String,
    pub name: String,
    pub icon: String,
    pub status: PlotStatus,
    pub ready_at: i64,
    pub season: u8,
    pub seasons: u8,
    pub remaining: u32,
    pub watered: bool,
    pub can_water: bool,
    pub can_harvest: bool,
    pub pest: Option<PestView>,
    pub can_clear_pest: bool,
    pub disaster: Option<DisasterView>,
    pub can_mitigate: bool,
    pub off_season: bool,
    pub greenhouse: bool,
    pub withered: bool,
    pub region_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotEffectsView {
    pub exp_pct: f64,
    pub coin_pct: f64,
    pub yield_pct: u32,
    pub luck_pct: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotView {
    pub index: u8,
    pub label: String,
    pub unlocked: bool,
    pub kind: PlotKind,
    pub unlock_level: u32,
    pub unlock_cost: u64,
    pub tier: u8,
    pub tier_name: String,
    pub effects: PlotEffectsView,
    pub upgrade_cost: u64,
    pub upgrade_level: u32,
    pub max_tier: bool,
    pub can_upgrade: bool,
    pub crop: Option<CropView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Seed,
    Produce,
    Ranch,
    Craft,
    Fish,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStack {
    pub key: String,
    pub name: String,
    pub qty: u32,
    pub kind: ItemKind,
    /// 能不能卖（专用饲料这类只能用的东西为 false）。
    pub sellable: bool,
    /// 单件售价；不可卖时为 0。
    pub unit_price: u64,
    /// 鱼的均价/重量/品质说明；其它物品为空。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilityView {
    pub kind: FacilityKind,
    pub name: String,
    pub owned: bool,
    pub unlock_level: u32,
    pub cost: u64,
    pub count: u32,
    pub max_count: u32,
    pub next_cost: u64,
    pub damaged: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexView {
    pub crop_id: String,
    pub name: String,
    pub icon: String,
    pub discovered: bool,
    pub first_at: i64,
    pub total: u64,
    pub best: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetTalentView {
    pub id: PetTalent,
    pub label: String,
    pub description: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetTalentTierView {
    pub tier: u8,
    pub threshold: u32,
    pub unlocked: bool,
    pub options: Vec<PetTalentView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetView {
    pub id: String,
    pub name: String,
    pub species: PetSpecies,
    pub species_label: String,
    pub species_emoji: String,
    pub breed: PetBreed,
    pub breed_label: String,
    pub image: String,
    pub temperament: String,
    pub temperament_label: String,
    pub personality: String,
    pub quirk: String,
    pub quirk_line: String,
    pub skill: String,
    pub skill_description: String,
    pub active_skill: String,
    pub active_description: String,
    pub skill_remaining_ms: i64,
    pub can_use_skill: bool,
    pub activity: String,
    pub activity_label: String,
    pub activity_line: String,
    pub activity_streak: u32,
    pub mischief_chance: u32,
    pub behavior: String,
    pub dig_skill: u32,
    pub dig_exp: u64,
    pub dig_need: u64,
    pub guard_skill: u32,
    pub guard_exp: u64,
    pub guard_need: u64,
    pub guard_train: u32,
    pub dig_train: u32,
    pub mischief_train: u32,
    pub companion_train: u32,
    pub training_spent: u32,
    pub training_cap: u32,
    pub training_center: bool,
    pub talent_tiers: Vec<PetTalentTierView>,
    pub boarded: bool,
    pub boarding_fee: u64,
    pub stage: PetStage,
    pub stage_label: String,
    pub age_days: u64,
    pub level: u32,
    pub exp: u64,
    pub exp_need: u64,
    pub bond: u32,
    pub satiety: u32,
    pub energy: u32,
    pub mood: u32,
    pub mood_label: String,
    pub health: u32,
    pub sickness: Option<String>,
    pub location: PetLocation,
    pub location_label: String,
    pub job_level: u32,
    pub job_exp: u64,
    pub job_need: u64,
    pub fishing_job_level: u32,
    pub fishing_job_exp: u64,
    pub fishing_job_need: u64,
    pub can_feed: bool,
    pub can_touch: bool,
    pub can_play: bool,
    pub can_heal: bool,
    pub can_dig: bool,
    pub touch_remaining_ms: i64,
    pub play_remaining_ms: i64,
    pub dig_remaining_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchAnimalView {
    pub kind: RanchAnimalKind,
    pub label: String,
    pub emoji: String,
    pub name: String,
    pub satiety: u32,
    pub health: u32,
    pub sick: bool,
    pub product: String,
    pub product_ready_at: i64,
    pub remaining_ms: i64,
    pub can_collect: bool,
    pub can_feed: bool,
    pub can_heal: bool,
    pub quality: u8,
    pub quality_label: String,
    pub generation: u32,
    /// 生产特性与一句话说明。
    pub trait_label: String,
    pub trait_hint: String,
    pub age_days: u32,
    /// 特殊产物名与概率说明（按最终品质）。
    pub special_name: String,
    pub special_hint: String,
    /// 血统线与幼年/近亲/繁殖状态。
    pub line_id: u32,
    pub line_label: String,
    pub young: bool,
    pub young_remaining_ms: i64,
    pub inbred: bool,
    pub parents: String,
    pub breed_ready: bool,
    pub breed_remaining_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchBreedPreview {
    pub ok: bool,
    pub kind: RanchAnimalKind,
    pub kind_label: String,
    pub quality: u8,
    pub quality_label: String,
    pub generation: u32,
    pub line_id: u32,
    pub line_label: String,
    pub trait_label: String,
    pub trait_hint: String,
    pub inbred: bool,
    pub parents: String,
    pub warnings: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchPenView {
    pub index: u8,
    pub animal: Option<RanchAnimalView>,
    /// 栏舍卫生 0–100。
    pub cleanliness: u32,
    pub can_clean: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchRecipeView {
    pub recipe: RanchRecipe,
    pub label: String,
    pub input_text: String,
    pub duration_ms: i64,
    pub output_name: String,
    pub output_price: u64,
    pub can_start: bool,
    pub missing: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopJobView {
    pub kind: String,
    pub label: String,
    pub is_crop: bool,
    pub output_name: String,
    pub output_price: u64,
    pub quality: u8,
    pub quality_label: String,
    pub ready_at: i64,
    pub remaining_ms: i64,
    pub can_collect: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopSlotView {
    pub index: u8,
    pub equipment: Option<EquipmentKind>,
    pub equipment_label: String,
    pub equipment_emoji: String,
    pub equipment_level: u8,
    pub equipment_max_level: u8,
    pub equipment_speed_pct: u32,
    pub equipment_yield_pct: u32,
    pub upgrade_cost: u64,
    pub can_upgrade: bool,
    pub upgrade_missing: String,
    pub job: Option<WorkshopJobView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopEquipmentView {
    pub kind: EquipmentKind,
    pub label: String,
    pub emoji: String,
    pub cost: u64,
    pub level: u32,
    pub installed: bool,
    pub can_install: bool,
    pub missing: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaitRecipeView {
    pub recipe: crate::fishing::BaitRecipe,
    pub label: String,
    pub input_text: String,
    pub output_text: String,
    pub duration_ms: i64,
    pub can_start: bool,
    pub missing: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishRecipeView {
    pub recipe: crate::workshop::FishRecipe,
    pub label: String,
    pub input_text: String,
    pub output_text: String,
    pub duration_ms: i64,
    pub can_start: bool,
    pub missing: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CropRecipeView {
    pub crop_id: String,
    pub name: String,
    pub qty: u32,
    pub need: u32,
    pub output_name: String,
    pub output_price: u64,
    pub duration_ms: i64,
    pub can_start: bool,
    pub missing: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopRecipeView {
    pub recipe: EquipmentRecipe,
    pub equipment: EquipmentKind,
    pub equipment_label: String,
    pub label: String,
    pub input_text: String,
    pub duration_ms: i64,
    pub output_name: String,
    pub output_price: u64,
    pub can_start: bool,
    pub missing: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopView {
    pub built: bool,
    pub can_build: bool,
    pub cost: u64,
    pub level: u32,
    pub slots: u8,
    pub max_slots: u8,
    pub can_expand: bool,
    pub expand_cost: u64,
    pub expand_level: u32,
    pub slot_views: Vec<WorkshopSlotView>,
    pub equipment_options: Vec<WorkshopEquipmentView>,
    pub manual_recipes: Vec<RanchRecipeView>,
    pub bait_recipes: Vec<BaitRecipeView>,
    pub fish_recipes: Vec<FishRecipeView>,
    pub crop_recipes: Vec<CropRecipeView>,
    pub equipment_recipes: Vec<WorkshopRecipeView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchOrderView {
    pub id: String,
    pub npc_name: String,
    pub base_key: String,
    pub item_name: String,
    pub quality: u8,
    pub quality_label: String,
    pub qty: u32,
    pub expires_day: String,
    pub reward_coins: u64,
    pub reward_exp: u64,
    pub reward_feed: u32,
    pub have: u32,
    pub can_deliver: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchOrdersView {
    pub day: String,
    pub offers: Vec<RanchOrderView>,
    pub active: Vec<RanchOrderView>,
    pub max_active: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchCodexTraitView {
    pub key: String,
    pub label: String,
    pub have: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondWaterView {
    pub temperature_tenths: i32,
    pub temperature_label: String,
    pub oxygen: u32,
    pub cleanliness: u32,
    pub ammonia: u32,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondFishView {
    pub fish_id: String,
    pub name: String,
    pub count: u32,
    pub weight_tenths: u32,
    pub weight_label: String,
    pub quality: u8,
    pub quality_label: String,
    pub market_weight_tenths: u32,
    pub market_weight_label: String,
    pub ready: bool,
    pub progress_pct: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondStockView {
    pub fish_id: String,
    pub name: String,
    pub have: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondOrderView {
    pub fish_name: String,
    pub qty: u32,
    pub min_weight_label: String,
    pub reward_coins: u64,
    pub have: u32,
    pub can_deliver: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondMilestoneView {
    pub key: String,
    pub label: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondView {
    pub built: bool,
    pub build_level: u32,
    pub build_cost: u64,
    pub can_build: bool,
    pub build_missing: String,
    pub water: Option<PondWaterView>,
    pub status_label: String,
    pub hint: String,
    pub fish: Vec<PondFishView>,
    pub stock: Vec<PondStockView>,
    pub capacity: u32,
    pub stock_count: u32,
    pub species_cap: usize,
    pub species_count: usize,
    pub aerator: bool,
    pub plants: bool,
    pub aerator_cost: u64,
    pub plants_cost: u64,
    pub feed_today: u32,
    pub feed_max: u32,
    pub can_feed: bool,
    pub can_clean: bool,
    pub auto_fry: bool,
    pub order: Option<PondOrderView>,
    pub order_done: bool,
    pub milestones: Vec<PondMilestoneView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchCodexKindView {
    pub kind: RanchAnimalKind,
    pub label: String,
    pub emoji: String,
    pub traits: Vec<RanchCodexTraitView>,
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchCodexSpecialView {
    pub key: String,
    pub name: String,
    pub have: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchCodexView {
    pub kinds: Vec<RanchCodexKindView>,
    pub specials: Vec<RanchCodexSpecialView>,
    pub animal_done: u32,
    pub animal_total: u32,
    pub special_done: u32,
    pub special_total: u32,
    /// 评比累计奖章。
    pub medals: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchContestResultView {
    pub week: String,
    pub animal_name: String,
    pub kind_label: String,
    pub score: u32,
    pub rank: u32,
    pub reward_coins: u64,
    pub medal: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchContestView {
    pub week: String,
    pub entered: bool,
    pub result: Option<RanchContestResultView>,
    pub medals: u32,
    pub best_rank: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RanchView {
    pub pens: Vec<RanchPenView>,
    pub can_expand: bool,
    pub expand_cost: u64,
    pub max_pens: u8,
    pub feed_count: u32,
    pub orders: RanchOrdersView,
    pub codex: RanchCodexView,
    pub contest: RanchContestView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetCareView {
    pub dog_bed: bool,
    pub cat_bed: bool,
    pub dog_bed_broken: bool,
    pub cat_bed_broken: bool,
    pub bowls: u8,
    pub dog_toy: bool,
    pub cat_toy: bool,
    pub dog_toy_broken: bool,
    pub cat_toy_broken: bool,
    pub medicine: u32,
    pub cat_food: u32,
    pub dog_food: u32,
    pub dog_bone: u32,
    pub cat_can: u32,
    pub cat_food_cost: u64,
    pub dog_food_cost: u64,
    pub dog_bone_cost: u64,
    pub cat_can_cost: u64,
    pub dog_bed_cost: u64,
    pub cat_bed_cost: u64,
    pub bowl_cost: u64,
    pub dog_toy_cost: u64,
    pub cat_toy_cost: u64,
    pub medicine_cost: u64,
    pub dog_bed_repair: u64,
    pub cat_bed_repair: u64,
    pub dog_toy_repair: u64,
    pub cat_toy_repair: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenView {
    pub crop_id: String,
    pub name: String,
    pub icon: String,
    pub level: u32,
    pub need_qty: u32,
    pub coins: u64,
    pub have_qty: u32,
    pub can_craft: bool,
    pub reward_hint: String,
    pub crafted: u32,
    pub featured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenMilestoneView {
    pub unique: u32,
    pub reward_coins: u64,
    pub claimed: bool,
    pub claimable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingSpotView {
    pub spot: crate::fishing::FishingSpot,
    pub label: String,
    pub emoji: String,
    pub level: u32,
    pub unlocked: bool,
    pub fish_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RodView {
    pub kind: crate::fishing::RodKind,
    pub label: String,
    pub cost: u64,
    pub level: u32,
    pub owned: bool,
    pub current: bool,
    pub can_buy: bool,
    pub frame_pct: u32,
    pub auto_minutes: i64,
    pub wear: u32,
    pub max_wear: u32,
    pub profit_label: String,
    pub maintenance_cost: u64,
    pub needs_maintenance: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaitView {
    pub bait_type: crate::fishing::BaitType,
    pub quality: crate::fishing::BaitQuality,
    pub label: String,
    pub price: u64,
    pub count: u32,
    pub quality_bonus: u32,
    pub weight_bonus: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaitTypeView {
    pub bait_type: crate::fishing::BaitType,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCatchView {
    pub fish_id: String,
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoReportView {
    pub at: i64,
    pub bait_bought: u32,
    pub bait_used: u32,
    pub coins_spent: u64,
    pub catches: Vec<AutoCatchView>,
    pub total_caught: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingSessionView {
    pub id: u64,
    pub spot: crate::fishing::FishingSpot,
    pub spot_label: String,
    pub fish_id: String,
    pub fish_name: String,
    pub faces_right: bool,
    pub price: u64,
    pub difficulty: u32,
    pub is_king: bool,
    pub is_golden: bool,
    pub expires_at: i64,
    pub remaining_ms: i64,
    pub expired: bool,
    pub params: crate::fishing::FishingParams,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingAutoView {
    pub enabled: bool,
    pub spot: crate::fishing::FishingSpot,
    pub spot_label: String,
    pub rod: crate::fishing::RodKind,
    pub rod_label: String,
    pub bait_quality: crate::fishing::BaitQuality,
    pub bait_quality_label: String,
    pub bait_type: crate::fishing::BaitType,
    pub bait_type_label: String,
    pub interval_ms: i64,
    pub interval_label: String,
    pub next_at: i64,
    pub can_toggle: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingCommissionView {
    pub id: String,
    pub npc_name: String,
    pub title: String,
    pub fish_name: String,
    pub spot_label: String,
    pub min_weight_tenths: u32,
    pub qty: u32,
    pub progress: u32,
    pub complete: bool,
    pub reward_coins: u64,
    pub reward_bait: u32,
    pub reward_exp: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingTournamentView {
    pub day: String,
    pub npc_name: String,
    pub npc_best_tenths: u32,
    pub best_tenths: u32,
    pub best_fish_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingTournamentResultView {
    pub day: String,
    pub message: String,
    pub rank: u32,
    pub reward_coins: u64,
    pub reward_bait: u32,
    pub reward_exp: u64,
    pub claimed: bool,
    pub best_tenths: u32,
    pub npc_best_tenths: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingQuestView {
    pub offers: Vec<FishingCommissionView>,
    pub active: Vec<FishingCommissionView>,
    pub tournament: Option<FishingTournamentView>,
    pub results: Vec<FishingTournamentResultView>,
    pub can_refresh: bool,
    pub refresh_used: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingConditionView {
    pub season_label: String,
    pub time_label: String,
    pub weather_label: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishCodexView {
    pub id: String,
    pub name: String,
    pub spot: crate::fishing::FishingSpot,
    pub spot_label: String,
    pub rarity: crate::fishing::FishRarity,
    pub rarity_label: String,
    pub price: u64,
    pub difficulty: u32,
    pub caught: bool,
    pub count: u64,
    pub best_weight_tenths: u32,
    pub best_quality: u32,
    pub best_quality_label: String,
    pub best_price: u64,
    pub golden: bool,
    pub golden_count: u64,
    pub manual_count: u64,
    pub manual_best_weight_tenths: u32,
    pub manual_best_quality: u32,
    pub manual_best_quality_label: String,
    pub manual_best_price: u64,
    pub auto_count: u64,
    pub auto_best_weight_tenths: u32,
    pub auto_best_quality: u32,
    pub auto_best_quality_label: String,
    pub auto_best_price: u64,
    pub condition_hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishCatchView {
    pub fish_id: String,
    pub name: String,
    pub weight_tenths: u32,
    pub quality: u32,
    pub quality_label: String,
    pub price: u64,
    pub market_pct: u32,
    pub is_king: bool,
    pub golden: bool,
    pub is_record: bool,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotProgressView {
    pub spot: crate::fishing::FishingSpot,
    pub label: String,
    pub caught: usize,
    pub total: usize,
    pub reward_coins: u64,
    pub reward_bait_label: String,
    pub claimed: bool,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KingTrophyView {
    pub fish_id: String,
    pub name: String,
    pub spot_label: String,
    pub caught: bool,
    pub count: u64,
    pub best_weight_tenths: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub level: u32,
    pub max_level: u32,
    pub cost: u32,
    pub can_upgrade: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisciplineView {
    pub kind: crate::skills::DisciplineKind,
    pub label: String,
    pub level: u32,
    pub exp: u64,
    pub exp_need: u64,
    pub points: u32,
    pub skills: Vec<SkillView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffView {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub remaining_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffItemView {
    pub key: String,
    pub name: String,
    pub desc: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsView {
    pub disciplines: Vec<DisciplineView>,
    pub buffs: Vec<BuffView>,
    pub buff_items: Vec<BuffItemView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FishingView {
    pub rod: crate::fishing::RodKind,
    pub rod_label: String,
    pub frame_pct: u32,
    pub level: u32,
    pub exp: u64,
    pub exp_need: u64,
    pub skill_points: u32,
    pub skills: Vec<SkillView>,
    pub auto_interval_ms: i64,
    pub auto_interval_label: String,
    pub bait: u32,
    pub bait_price: u64,
    pub active_bait: crate::fishing::BaitQuality,
    pub active_bait_label: String,
    pub active_bait_type: crate::fishing::BaitType,
    pub active_bait_type_label: String,
    pub bait_types: Vec<BaitTypeView>,
    pub bait_counts: Vec<BaitView>,
    pub rod_wear: u32,
    pub rod_max_wear: u32,
    pub maintenance_cost: u64,
    pub needs_maintenance: bool,
    pub locked_fish: Vec<String>,
    pub spots: Vec<FishingSpotView>,
    pub rods: Vec<RodView>,
    pub session: Option<FishingSessionView>,
    pub auto: FishingAutoView,
    pub codex: Vec<FishCodexView>,
    pub codex_caught: usize,
    pub codex_total: usize,
    pub spot_progress: Vec<SpotProgressView>,
    pub golden_caught: usize,
    pub golden_total: usize,
    pub king_trophies: Vec<KingTrophyView>,
    pub full_codex_claimed: bool,
    pub last_catch: Option<FishCatchView>,
    pub last_auto_report: Option<AutoReportView>,
    pub quests: FishingQuestView,
    pub condition: FishingConditionView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomStyleView {
    pub id: String,
    pub name: String,
    pub locked: bool,
    pub unlock_hint: String,
    pub price: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSurfaceDefView {
    pub id: String,
    pub name: String,
    pub sprite: String,
    pub sprite_rotations: u8,
    pub support_kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomWindowStyleView {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomFurnitureDefView {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub depth: u32,
    pub layer: u32,
    pub collision: bool,
    pub kind: String,
    pub capacity: u32,
    pub sprite: Option<String>,
    pub sprite_rotations: u8,
    pub price: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomView {
    pub room_id: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub grid: u32,
    pub layout: crate::room::RoomLayout,
    pub floors: Vec<RoomStyleView>,
    pub walls: Vec<RoomStyleView>,
    pub window_styles: Vec<RoomWindowStyleView>,
    pub catalog: Vec<RoomFurnitureDefView>,
    pub surface_catalog: Vec<RoomSurfaceDefView>,
    pub available_furniture: std::collections::BTreeMap<String, u32>,
    pub owned_furniture: std::collections::BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecorOfferView {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub price: u64,
    pub sold: bool,
    pub owned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecorShopView {
    pub day: String,
    pub offers: Vec<DecorOfferView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcFestivalView {
    pub name: String,
    pub reward: u64,
    pub claimed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRoomView {
    pub npc_id: String,
    pub name: String,
    pub relation: String,
    pub friendship: i32,
    pub threshold: i32,
    pub unlocked: bool,
    pub souvenirs: Vec<String>,
    pub souvenir_items: Vec<crate::npc::NpcSouvenirView>,
    pub room: RoomView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FarmOrderView {
    pub id: String,
    pub npc_name: String,
    pub crop_id: String,
    pub crop_name: String,
    pub item_key: String,
    pub item_name: String,
    pub qty: u32,
    pub expires_day: String,
    pub reward_coins: u64,
    pub reward_exp: u64,
    pub have: u32,
    pub can_deliver: bool,
    pub is_craft: bool,
    pub craft: Option<CraftOrderTrackView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftOrderTrackView {
    pub status: String,
    pub detail: String,
    pub can_start: bool,
    pub auto: bool,
    pub equipment_label: String,
    pub equipment_installed: bool,
    pub job_slot: Option<u8>,
    pub remaining_ms: i64,
    pub inputs: Vec<CraftInputView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftInputView {
    pub name: String,
    pub need: u32,
    pub have: u32,
    pub is_craft: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FarmOrdersView {
    pub day: String,
    pub offers: Vec<FarmOrderView>,
    pub active: Vec<FarmOrderView>,
    pub max_active: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysteryOfferView {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub desc: String,
    pub tag: String,
    pub price: u64,
    pub rarity: String,
    pub rarity_label: String,
    pub featured: bool,
    pub sold: bool,
    pub affordable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysteryShopView {
    pub day: String,
    pub offers: Vec<MysteryOfferView>,
    pub legendary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub rev: u64,
    pub now: i64,
    pub season: Season,
    pub season_label: String,
    pub season_emoji: String,
    pub region: Region,
    pub region_label: String,
    pub farm_name: String,
    pub owner_name: String,
    pub created_at: i64,
    pub level: u32,
    pub exp: u64,
    pub exp_need: u64,
    pub coins: u64,
    pub plots: Vec<PlotView>,
    pub inventory: Vec<ItemStack>,
    pub facilities: Vec<FacilityView>,
    pub onboarded: bool,
    pub daily: DailyView,
    pub orders: FarmOrdersView,
    pub mystery: MysteryShopView,
    pub level_rewards: Vec<LevelRewardView>,
    pub level_rewards_claimable: u32,
    pub codex: Vec<CodexView>,
    pub codex_discovered: usize,
    pub achievements: Vec<AchievementView>,
    pub achievements_earned: usize,
    pub specimens: Vec<SpecimenView>,
    pub specimen_unique: usize,
    pub specimen_featured: Option<String>,
    pub specimen_milestones: Vec<SpecimenMilestoneView>,
    pub pet: Option<PetView>,
    pub pets: Vec<PetView>,
    pub pet_care: PetCareView,
    pub ranch: RanchView,
    pub pond: PondView,
    pub workshop: WorkshopView,
    pub fishing: FishingView,
    pub room: RoomView,
    pub extra_room: Option<RoomView>,
    pub decor_shop: DecorShopView,
    pub npc_rooms: Vec<NpcRoomView>,
    pub skills: SkillsView,
    pub boarding_debt: u64,
    pub exp_rate: u32,
    pub npcs: Vec<NpcView>,
    pub npc_messages: Vec<NpcMessageView>,
    pub village_events: Vec<String>,
    pub festival: Option<NpcFestivalView>,
    pub steals_left: u32,
    pub helps_left: u32,
    pub ranch_steals_left: u32,
    pub ranch_helps_left: u32,
    pub log: Vec<LogEntry>,
    pub content_crops: usize,
}

fn order_item_count(game: &Game, key: &str) -> u32 {
    if key.starts_with("ranch:") {
        (1..=3u8)
            .map(|quality| {
                game.state
                    .inventory
                    .get(&crate::ranch::with_quality(key, quality))
                    .copied()
                    .unwrap_or(0)
            })
            .sum()
    } else {
        game.state.inventory.get(key).copied().unwrap_or(0)
    }
}

fn order_ingredient_name(game: &Game, key: &str) -> String {
    if let Some(id) = key.strip_prefix("crop:") {
        return game
            .content
            .crop(id)
            .map(|crop| crop.name.clone())
            .unwrap_or_else(|| id.to_string());
    }
    if key.starts_with("ranch:") {
        return crate::ranch::display_name(key).unwrap_or_else(|| key.to_string());
    }
    if key.starts_with("craft:") {
        return craft_item(key)
            .map(|item| item.name.to_string())
            .unwrap_or_else(|| key.to_string());
    }
    key.to_string()
}

fn recipe_in_plan(target: EquipmentRecipe, candidate: EquipmentRecipe, depth: u8) -> bool {
    if target == candidate {
        return true;
    }
    if depth >= 4 {
        return false;
    }
    for (key, _) in target.inputs() {
        if !key.starts_with("craft:") {
            continue;
        }
        let Some(sub) = EquipmentRecipe::ALL
            .iter()
            .copied()
            .find(|recipe| recipe.output_key() == *key)
        else {
            continue;
        };
        if recipe_in_plan(sub, candidate, depth + 1) {
            return true;
        }
    }
    false
}

fn craft_order_track(
    game: &Game,
    order: &crate::orders::FarmOrder,
    now: i64,
) -> CraftOrderTrackView {
    let Some(recipe) = order.recipe else {
        return CraftOrderTrackView {
            status: "不可加工".into(),
            detail: String::new(),
            can_start: false,
            auto: false,
            equipment_label: String::new(),
            equipment_installed: false,
            job_slot: None,
            remaining_ms: 0,
            inputs: Vec::new(),
        };
    };
    let equipment = recipe.equipment();
    let installed = game
        .state
        .workshop
        .equipment
        .iter()
        .flatten()
        .any(|kind| *kind == equipment);
    let mut job_slot = None;
    let mut remaining_ms = 0;
    let mut job_ready = false;
    for slot in 0..game.state.workshop.slots {
        if let Some(WorkshopJob::Equipment {
            recipe: active_recipe,
            ready_at,
            ..
        }) = game.state.workshop.job(slot)
        {
            let in_plan = *active_recipe == recipe
                || (order.auto && recipe_in_plan(recipe, *active_recipe, 0));
            if in_plan {
                job_slot = Some(slot);
                remaining_ms = (*ready_at - now).max(0);
                job_ready = now >= *ready_at;
                break;
            }
        }
    }
    let inputs: Vec<CraftInputView> = recipe
        .inputs()
        .iter()
        .map(|(key, need)| {
            let have = order_item_count(game, key);
            CraftInputView {
                name: order_ingredient_name(game, key),
                need: *need,
                have,
                is_craft: key.starts_with("craft:"),
            }
        })
        .collect();
    let missing: Vec<String> = inputs
        .iter()
        .filter(|input| input.have < input.need)
        .map(|input| format!("{}×{}", input.name, input.need - input.have))
        .collect();
    let output_have = order_item_count(game, &order.delivery_key());
    let equipment_label = equipment.label().to_string();
    let (status, detail, can_start) = if output_have >= order.qty {
        (
            "可交付".to_string(),
            "仓库里已有成品，去村庄订单交付".to_string(),
            false,
        )
    } else if let Some(slot) = job_slot {
        if job_ready {
            (
                "可收取".to_string(),
                if order.auto {
                    "加工好了，下次同步会自动收取".to_string()
                } else {
                    format!("加工好了，去 {slot} 号槽取货")
                },
                false,
            )
        } else if order.auto {
            (
                "自动排程中".to_string(),
                format!("{slot} 号槽正在做，做完自动收取并接下一道"),
                false,
            )
        } else {
            (
                "加工中".to_string(),
                format!("{slot} 号槽正在做，到时间取货"),
                false,
            )
        }
    } else if !game.state.workshop.built {
        ("未建制造屋".to_string(), "先去制造屋建一座".to_string(), false)
    } else if !installed {
        (
            "缺设备".to_string(),
            format!("需要先装一台{equipment_label}"),
            false,
        )
    } else if !missing.is_empty() {
        (
            "缺原料".to_string(),
            format!("还缺 {}", missing.join("、")),
            false,
        )
    } else if game.state.workshop.first_idle_slot(equipment).is_none() {
        (
            "加工槽忙".to_string(),
            format!("{equipment_label}槽都在忙，等一会儿"),
            false,
        )
    } else {
        (
            "可加工".to_string(),
            "原料齐了，点「一键送去做」".to_string(),
            true,
        )
    };
    CraftOrderTrackView {
        status,
        detail,
        can_start,
        auto: order.auto,
        equipment_label,
        equipment_installed: installed,
        job_slot,
        remaining_ms,
        inputs,
    }
}

fn style_views(
    styles: &[(&'static str, &'static str)],
    owned: &std::collections::BTreeSet<String>,
    kind: crate::decor::DecorKind,
) -> Vec<RoomStyleView> {    styles
        .iter()
        .map(|(id, name)| {
            let price = match kind {
                crate::decor::DecorKind::Floor => crate::decor::floor_price(id),
                crate::decor::DecorKind::Wall => crate::decor::wall_price(id),
            };
            let locked = !owned.contains(*id);
            RoomStyleView {
                id: (*id).into(),
                name: (*name).into(),
                locked,
                unlock_hint: if locked { "家具店购买".into() } else { String::new() },
                price,
            }
        })
        .collect()
}

fn surface_catalog_views() -> Vec<RoomSurfaceDefView> {
    crate::room::SURFACE_ITEMS
        .iter()
        .map(|def| RoomSurfaceDefView {
            id: def.id.into(),
            name: def.name.into(),
            sprite: def.sprite.into(),
            sprite_rotations: def.sprite_rotations,
            support_kinds: def.support_kinds.iter().map(|kind| (*kind).into()).collect(),
        })
        .collect()
}

fn room_view_data(room_id: &str, label: &str, width: u32, height: u32, layout: crate::room::RoomLayout) -> RoomView {
    RoomView {
        room_id: room_id.into(),
        label: label.into(),
        width,
        height,
        grid: crate::room::ROOM_GRID,
        layout,
        floors: crate::room::floor_styles()
            .iter()
            .map(|(id, name)| RoomStyleView {
                id: (*id).into(),
                name: (*name).into(),
                locked: false,
                unlock_hint: String::new(),
                price: 0,
            })
            .collect(),
        walls: crate::room::wall_styles()
            .iter()
            .map(|(id, name)| RoomStyleView {
                id: (*id).into(),
                name: (*name).into(),
                locked: false,
                unlock_hint: String::new(),
                price: 0,
            })
            .collect(),
        window_styles: crate::room::window_styles()
            .iter()
            .map(|(id, name, width, height)| RoomWindowStyleView {
                id: (*id).into(),
                name: (*name).into(),
                width: *width,
                height: *height,
            })
            .collect(),
        catalog: crate::room::FURNITURE
            .iter()
            .map(|def| RoomFurnitureDefView {
                id: def.id.into(),
                name: def.name.into(),
                width: def.width,
                depth: def.depth,
                layer: def.layer,
                collision: def.collision,
                kind: def.kind.into(),
                capacity: def.capacity,
                sprite: def.sprite.map(|s| s.into()),
                sprite_rotations: def.sprite_rotations,
                price: crate::decor::furniture_price(def.id),
                source: crate::decor::furniture_source(def.id).into(),
            })
            .collect(),
        surface_catalog: surface_catalog_views(),
        available_furniture: std::collections::BTreeMap::new(),
        owned_furniture: std::collections::BTreeMap::new(),
    }
}

fn npc_threshold(id: &str) -> i32 {
    match id {
        "ahe" => 10,
        "xiaoman" => 25,
        _ => 40,
    }
}

fn npc_room_layout(id: &str) -> crate::room::RoomLayout {
    use crate::room::{RoomFurniture, RoomLayout, RoomWindow};
    let f = |id: &str, def: &str, x: u32, y: u32| RoomFurniture {
        id: id.into(),
        def: def.into(),
        x,
        y,
        rot: 0,
        fish: vec![],
    };
    let mut furniture = vec![
        f("npc-bed", "bed", 64, 8),
        f("npc-desk", "desk", 8, 8),
        f("npc-chair", "chair", 24, 16),
        f("npc-rug", "rug", 32, 40),
        f("npc-cabinet", "cabinet", 0, 48),
        f("npc-plant", "plant", 88, 56),
        f("npc-vase", "flower_vase", 88, 24),
        f("npc-planter", "planter", 72, 48),
    ];
    if id == "ahe" {
        furniture.push(f("npc-tank", "aquarium_small", 8, 64));
    } else if id == "qingxing" {
        let mut tank = f("npc-tank", "aquarium_large", 40, 8);
        tank.fish = vec!["carp".into()];
        furniture.push(tank);
    }
    RoomLayout {
        floor: "wood".into(),
        wall: if id == "xiaoman" { "sage".into() } else { "cream".into() },
        door: None,
        windows: vec![RoomWindow {
            id: "npc-window".into(),
            wall: "right".into(),
            x: 28,
            y: 24,
            w: 24,
            h: 20,
            style: "large".into(),
        }],
        furniture,
        surface_items: vec![],
    }
}

impl Game {
    fn room_view(&self, room_id: &str, _now: i64) -> Option<RoomView> {
        let layout = self
            .state
            .room
            .layout(room_id)
            .cloned()
            .or_else(|| (room_id == crate::room::LIVING_ROOM).then(crate::room::default_layout))?;
        let (width, height) = crate::room::room_size(room_id)?;
        Some(RoomView {
            room_id: room_id.into(),
            label: if room_id == crate::room::LIVING_ROOM {
                "小屋".into()
            } else {
                "里屋".into()
            },
            width,
            height,
            grid: crate::room::ROOM_GRID,
            layout,
            floors: style_views(
                &crate::room::floor_styles(),
                &self.state.room.owned_floors,
                crate::decor::DecorKind::Floor,
            ),
            walls: style_views(
                &crate::room::wall_styles(),
                &self.state.room.owned_walls,
                crate::decor::DecorKind::Wall,
            ),
            window_styles: crate::room::window_styles()
                .iter()
                .map(|(id, name, width, height)| RoomWindowStyleView {
                    id: (*id).into(),
                    name: (*name).into(),
                    width: *width,
                    height: *height,
                })
                .collect(),
            catalog: crate::room::FURNITURE
                .iter()
                .map(|def| RoomFurnitureDefView {
                    id: def.id.into(),
                    name: def.name.into(),
                    width: def.width,
                    depth: def.depth,
                    layer: def.layer,
                    collision: def.collision,
                    kind: def.kind.into(),
                    capacity: def.capacity,
                    sprite: def.sprite.map(|s| s.into()),
                    sprite_rotations: def.sprite_rotations,
                    price: crate::decor::furniture_price(def.id),
                    source: crate::decor::furniture_source(def.id).into(),
                })
                .collect(),
            surface_catalog: surface_catalog_views(),
            available_furniture: self.state.room.available_furniture(),
            owned_furniture: self.state.room.owned_furniture.clone(),
        })
    }

    pub fn snapshot(&self, now: i64) -> Snapshot {
        let plots = self
            .state
            .plots
            .iter()
            .map(|plot| {
                let crop = plot.crop.as_ref().map(|c| {
                    let def = self.content.crop(&c.crop_id);
                    let ready = c.ready_at <= now;
                    CropView {
                        crop_id: c.crop_id.clone(),
                        name: def
                            .map(|d| d.name.clone())
                            .unwrap_or_else(|| c.crop_id.clone()),
                        icon: def.map(|d| d.icon.clone()).unwrap_or_else(|| "🌱".into()),
                        status: if c.withered {
                            PlotStatus::Withered
                        } else if ready {
                            PlotStatus::Ready
                        } else {
                            PlotStatus::Growing
                        },
                        ready_at: c.ready_at,
                        season: c.season,
                        seasons: c.seasons,
                        remaining: c.remaining,
                        watered: c.watered,
                        can_water: !c.watered && !ready && !c.withered && c.disaster.is_none(),
                        can_harvest: ready && c.remaining > 0 && c.pest.is_none() && !c.withered,
                        pest: c.pest.as_ref().map(|pest| PestView {
                            kind: pest.kind,
                            label: pest.kind.label().to_string(),
                            since: pest.since,
                        }),
                        can_clear_pest: c.pest.is_some(),
                        disaster: c.disaster.as_ref().map(|disaster| DisasterView {
                            kind: disaster.kind,
                            label: disaster.kind.label().to_string(),
                            since: disaster.since,
                            ends_at: disaster.ends_at,
                            yield_penalty: disaster.kind.yield_penalty(),
                            growth_slow: disaster.kind.growth_slow(),
                            mitigation: disaster.kind.mitigation_label().to_string(),
                            can_mitigate: !disaster.kind.is_instant(),
                        }),
                        can_mitigate: c
                            .disaster
                            .as_ref()
                            .is_some_and(|disaster| !disaster.kind.is_instant()),
                        off_season: c.off_season,
                        greenhouse: c.greenhouse,
                        withered: c.withered,
                        region_label: def
                            .map(|d| d.region.label().to_string())
                            .unwrap_or_default(),
                    }
                });
                let tier = plot.tier.min(PLOT_MAX_TIER);
                let is_greenhouse = plot.kind == PlotKind::Greenhouse;
                let land = if is_greenhouse {
                    greenhouse_tier(tier)
                } else {
                    plot_tier(tier)
                };
                PlotView {
                    index: plot.index,
                    label: if is_greenhouse {
                        format!("温室{}", plot.index.saturating_sub(8))
                    } else {
                        format!("{}{}", self.content.nouns.plot, plot.index)
                    },
                    unlocked: plot.unlocked,
                    kind: plot.kind,
                    unlock_level: plot_unlock_level(plot.index),
                    unlock_cost: plot_unlock_cost(plot.index),
                    tier,
                    tier_name: land.name.into(),
                    effects: PlotEffectsView {
                        exp_pct: f64::from(land.exp_tenths) / 10.0,
                        coin_pct: f64::from(land.coin_tenths) / 10.0,
                        yield_pct: land.yield_pct,
                        luck_pct: land.luck_pct,
                    },
                    upgrade_cost: land.upgrade_cost,
                    upgrade_level: land.upgrade_level,
                    max_tier: tier >= PLOT_MAX_TIER,
                    can_upgrade: plot.unlocked
                        && tier < PLOT_MAX_TIER
                        && self.state.level >= land.upgrade_level
                        && self.state.coins >= land.upgrade_cost,
                    crop,
                }
            })
            .collect();

        let mut inventory = Vec::new();
        for (key, qty) in &self.state.inventory {
            if *qty == 0 {
                continue;
            }
            let mut parts = key.splitn(2, ':');
            let kind_str = parts.next().unwrap_or("");
            let id = parts.next().unwrap_or("");
            let kind = match kind_str {
                "seed" | "rare_seed" => ItemKind::Seed,
                "ranch" => ItemKind::Ranch,
                "craft" => ItemKind::Craft,
                "fish" => ItemKind::Fish,
                _ => ItemKind::Produce,
            };
            let (name, sellable, unit_price) = if kind == ItemKind::Ranch {
                (
                    crate::ranch::display_name(key).unwrap_or_else(|| id.to_string()),
                    crate::ranch::is_sellable(key),
                    crate::ranch::price_of(key).unwrap_or(0),
                )
            } else if kind == ItemKind::Fish {
                let def = fishing::fish(id);
                let price = self
                    .state
                    .fishing
                    .fish_batches
                    .get(id)
                    .map(|batch| batch.unit_price())
                    .or_else(|| def.map(|d| d.price))
                    .unwrap_or(0);
                (
                    def.map(|d| d.name.to_string())
                        .unwrap_or_else(|| id.to_string()),
                    true,
                    price,
                )
            } else if kind == ItemKind::Craft {
                if let Some(crop_id) = id.strip_prefix("box:") {
                    let def = self.content.crop(crop_id);
                    (
                        format!(
                            "{}礼盒",
                            def.map(|d| d.name.clone())
                                .unwrap_or_else(|| crop_id.to_string())
                        ),
                        true,
                        def.map(|d| crop_box_price(d.sell_price)).unwrap_or(0),
                    )
                } else if let Some(item) = craft_item(key) {
                    (item.name.to_string(), true, item.price)
                } else {
                    (id.to_string(), false, 0)
                }
            } else {
                let def = self.content.crop(id);
                let base = def
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| id.to_string());
                let name = if kind_str == "rare_seed" {
                    format!("稀有·{base}种子")
                } else {
                    match kind {
                        ItemKind::Seed => format!("{base}种子"),
                        _ => base,
                    }
                };
                let price = if kind_str == "rare_seed" {
                    0
                } else {
                    def.map(|d| d.sell_price).unwrap_or(0)
                };
                (name, kind == ItemKind::Produce, price)
            };
            let detail = if kind == ItemKind::Fish {
                self.state.fishing.fish_batches.get(id).map(|batch| {
                    format!(
                        "平均 {:.1}kg · {}",
                        f64::from(batch.avg_weight_tenths()) / 10.0,
                        fishing::quality_label(batch.avg_quality())
                    )
                })
            } else {
                None
            };
            inventory.push(ItemStack {
                key: key.clone(),
                name,
                qty: *qty,
                kind,
                sellable,
                unit_price,
                detail,
                locked: kind == ItemKind::Fish && self.state.fishing.locked_fish.contains(id),
            });
        }

        let facilities = [
            (FacilityKind::Sprinkler, self.state.facilities.sprinkler),
            (FacilityKind::Harvester, self.state.facilities.harvester),
            (FacilityKind::Windmill, self.state.facilities.windmills > 0),
            (
                FacilityKind::Greenhouse,
                self.state.facilities.greenhouses > 0,
            ),
            (
                FacilityKind::TrainingCenter,
                self.state.facilities.training_center,
            ),
        ]
        .into_iter()
        .map(|(kind, owned)| {
            let count = match kind {
                FacilityKind::Windmill => u32::from(self.state.facilities.windmills),
                FacilityKind::Greenhouse => u32::from(self.state.facilities.greenhouses),
                _ => u32::from(owned),
            };
            let max_count = crate::model::facility_max_count(kind);
            FacilityView {
                kind,
                name: facility_name(kind).to_string(),
                owned,
                unlock_level: facility_unlock_level(kind),
                cost: crate::model::facility_next_cost(kind, count),
                count,
                max_count,
                next_cost: crate::model::facility_next_cost(kind, count),
                damaged: if kind == FacilityKind::Windmill {
                    u32::from(self.state.facilities.damaged_windmills)
                } else {
                    0
                },
            }
        })
        .collect();

        let codex: Vec<CodexView> = self
            .content
            .crops
            .iter()
            .map(|def| {
                let entry = self.state.codex.get(&def.id);
                CodexView {
                    crop_id: def.id.clone(),
                    name: def.name.clone(),
                    icon: def.icon.clone(),
                    discovered: entry.is_some(),
                    first_at: entry.map(|e| e.first_at).unwrap_or(0),
                    total: entry.map(|e| e.total).unwrap_or(0),
                    best: entry.map(|e| e.best).unwrap_or(0),
                }
            })
            .collect();
        let codex_discovered = codex.iter().filter(|entry| entry.discovered).count();

        let achievement_views = achievements::views(&self.state);
        let achievements_earned = achievement_views.iter().filter(|a| a.earned).count();

        let featured_specimen = specimen::featured_crop(&self.content, &self.state.daily.day)
            .map(|crop| crop.id.clone());
        let specimens: Vec<SpecimenView> = self
            .content
            .crops
            .iter()
            .map(|def| {
                let recipe = specimen::recipe(def);
                let have = *self
                    .state
                    .inventory
                    .get(&format!("crop:{}", def.id))
                    .unwrap_or(&0);
                let upper = (def.unlock_level + 10).min(30);
                SpecimenView {
                    crop_id: def.id.clone(),
                    name: def.name.clone(),
                    icon: def.icon.clone(),
                    level: def.unlock_level,
                    need_qty: recipe.need_qty,
                    coins: recipe.coins,
                    have_qty: have,
                    can_craft: have >= recipe.need_qty && self.state.coins >= recipe.coins,
                    reward_hint: if upper > def.unlock_level {
                        format!("随机 {}–{} 级作物种子 ×1–2", def.unlock_level + 1, upper)
                    } else {
                        "随机同类种子 ×1–2".into()
                    },
                    crafted: self
                        .state
                        .specimen_collection
                        .get(&def.id)
                        .copied()
                        .unwrap_or(0),
                    featured: featured_specimen.as_deref() == Some(def.id.as_str()),
                }
            })
            .collect();
        let specimen_unique = self
            .state
            .specimen_collection
            .values()
            .filter(|count| **count > 0)
            .count();

        let farm_days = if self.state.created_at > 0 {
            ((now - self.state.created_at) / 86_400_000 + 1).max(1) as u32
        } else {
            1
        };
        let village_events = (0..3)
            .map(|offset| {
                crate::npc_story::village_event(now / 86_400_000 - offset)
                    .text
                    .to_string()
            })
            .collect();
        let festival = crate::npc_story::festival_of(&self.state.daily.day).map(|name| {
            let year = crate::npc_story::year_of(&self.state.daily.day);
            NpcFestivalView {
                name: name.to_string(),
                reward: crate::npc_story::festival_reward(),
                claimed: self.state.festival_claims.get(name) == Some(&year),
            }
        });
        let npcs: Vec<NpcView> = self
            .state
            .npcs
            .iter()
            .map(|npc| {
                npc::view(
                    npc,
                    &self.content,
                    &self.state.inventory,
                    &self.state.daily.day,
                    farm_days,
                    &self.state.owner_name,
                    now,
                )
            })
            .collect();
        let npc_messages: Vec<NpcMessageView> = self
            .state
            .npc_messages
            .iter()
            .map(|message| NpcMessageView {
                at: message.at,
                npc_name: self
                    .state
                    .npcs
                    .iter()
                    .find(|npc| npc.id == message.npc_id)
                    .map(|npc| npc.name.clone())
                    .unwrap_or_else(|| "邻居".into()),
                text: message.text.clone(),
                from_player: message.from_player,
            })
            .collect();

        let pets: Vec<PetView> = self
            .state
            .pets
            .iter()
            .map(|pet| {
                let mut view = pet_view(pet, now);
                view.training_center = self.state.facilities.training_center;
                view
            })
            .collect();
        let pet = pets.iter().find(|pet| !pet.boarded).cloned();
        let has_feed = self
            .state
            .inventory
            .iter()
            .any(|(key, qty)| key.starts_with("crop:") && *qty > 0);
        let premium_feed = *self.state.inventory.get("ranch:feed").unwrap_or(&0);
        let manual_recipes: Vec<RanchRecipeView> = RanchRecipe::ALL
            .iter()
            .map(|recipe| {
                let mut parts = Vec::new();
                let mut missing = Vec::new();
                for (base, need) in recipe.inputs() {
                    let have = inventory_count(&self.state, base);
                    let name = crate::ranch::item_def(base)
                        .map(|def| def.name)
                        .unwrap_or(base);
                    parts.push(format!("{name}×{need}"));
                    if have < *need {
                        missing.push(format!("缺{name}×{}", need - have));
                    }
                }
                RanchRecipeView {
                    recipe: *recipe,
                    label: recipe.label().into(),
                    input_text: parts.join(" + "),
                    duration_ms: recipe.duration_ms(),
                    output_name: recipe.output_name().into(),
                    output_price: recipe.output_price(),
                    can_start: self.state.workshop.built
                        && self.state.workshop.job(0).is_none()
                        && missing.is_empty(),
                    missing: missing.join("、"),
                }
            })
            .collect();
        let bait_recipes: Vec<BaitRecipeView> = crate::fishing::BaitRecipe::ALL
            .iter()
            .map(|recipe| {
                let mut parts = Vec::new();
                let mut missing = Vec::new();
                for (key, need) in recipe.inputs() {
                    let have = inventory_count(&self.state, key);
                    let name = if let Some(id) = key.strip_prefix("crop:") {
                        self.content
                            .crop(id)
                            .map(|crop| crop.name.clone())
                            .unwrap_or_else(|| id.to_string())
                    } else {
                        crate::ranch::item_def(key)
                            .map(|def| def.name.to_string())
                            .unwrap_or_else(|| (*key).to_string())
                    };
                    parts.push(format!("{name}×{need}"));
                    if have < *need {
                        missing.push(format!("缺{name}×{}", need - have));
                    }
                }
                BaitRecipeView {
                    recipe: *recipe,
                    label: recipe.label().into(),
                    input_text: parts.join(" + "),
                    output_text: format!(
                        "{}×{}（普通）",
                        recipe.bait_type().label(),
                        recipe.output_qty()
                    ),
                    duration_ms: recipe.duration_ms(),
                    can_start: self.state.workshop.built
                        && self.state.workshop.job(0).is_none()
                        && missing.is_empty(),
                    missing: missing.join("、"),
                }
            })
            .collect();
        let fish_recipes: Vec<FishRecipeView> = crate::workshop::FishRecipe::ALL
            .iter()
            .map(|recipe| {
                let mut parts = vec![format!("任意鱼×{}", recipe.fish_count())];
                let mut missing = Vec::new();
                let available_fish: u32 = self
                    .state
                    .inventory
                    .iter()
                    .filter_map(|(key, qty)| {
                        let id = key.strip_prefix("fish:")?;
                        if *qty == 0 || self.state.fishing.locked_fish.contains(id) {
                            return None;
                        }
                        Some(*qty)
                    })
                    .sum();
                if available_fish < recipe.fish_count() {
                    missing.push(format!(
                        "缺任意鱼×{}",
                        recipe.fish_count().saturating_sub(available_fish)
                    ));
                }
                for (key, need) in recipe.other_inputs() {
                    let have = inventory_count(&self.state, key);
                    let name = if let Some(id) = key.strip_prefix("crop:") {
                        self.content
                            .crop(id)
                            .map(|crop| crop.name.clone())
                            .unwrap_or_else(|| id.to_string())
                    } else {
                        crate::ranch::item_def(key)
                            .map(|def| def.name.to_string())
                            .unwrap_or_else(|| (*key).to_string())
                    };
                    parts.push(format!("{name}×{need}"));
                    if have < *need {
                        missing.push(format!("缺{name}×{}", need - have));
                    }
                }
                FishRecipeView {
                    recipe: *recipe,
                    label: recipe.label().into(),
                    input_text: parts.join(" + "),
                    output_text: format!("{}×{}", recipe.output_name(), recipe.output_qty()),
                    duration_ms: recipe.duration_ms(),
                    can_start: self.state.workshop.built
                        && self.state.workshop.job(0).is_none()
                        && missing.is_empty(),
                    missing: missing.join("、"),
                }
            })
            .collect();
        let crop_recipes: Vec<CropRecipeView> = self
            .content
            .crops
            .iter()
            .filter_map(|def| {
                let have = *self
                    .state
                    .inventory
                    .get(&format!("crop:{}", def.id))
                    .unwrap_or(&0);
                if have == 0 {
                    return None;
                }
                let missing = if have < CROP_BOX_INPUT {
                    format!("缺{}×{}", def.name, CROP_BOX_INPUT - have)
                } else {
                    String::new()
                };
                Some(CropRecipeView {
                    crop_id: def.id.clone(),
                    name: def.name.clone(),
                    qty: have,
                    need: CROP_BOX_INPUT,
                    output_name: format!("{}礼盒", def.name),
                    output_price: crop_box_price(def.sell_price),
                    duration_ms: CROP_BOX_INTERVAL_MS,
                    can_start: self.state.workshop.built
                        && self.state.workshop.job(0).is_none()
                        && have >= CROP_BOX_INPUT,
                    missing,
                })
            })
            .collect();
        let workshop_state = &self.state.workshop;
        let slot_views: Vec<WorkshopSlotView> = (0..workshop_state.slot_count())
            .map(|index| {
                let equipment = workshop_state.equipment(index);
                let job = workshop_state
                    .job(index)
                    .map(|job| workshop_job_view(job, &self.content, now));
                let level = workshop_state.equipment_level(index);
                let upgrade_cost = workshop_state.equipment_upgrade_cost(index).unwrap_or(0);
                let can_upgrade =
                    upgrade_cost > 0 && job.is_none() && self.state.coins >= upgrade_cost;
                let upgrade_missing = if index == 0 {
                    "手工台不可升级".to_string()
                } else if equipment.is_none() {
                    "空槽".to_string()
                } else if level >= WORKSHOP_EQUIPMENT_MAX_LEVEL {
                    "已满级".to_string()
                } else if job.is_some() {
                    "加工中".to_string()
                } else if self.state.coins < upgrade_cost {
                    format!("金币差 {}", upgrade_cost - self.state.coins)
                } else {
                    String::new()
                };
                WorkshopSlotView {
                    index,
                    equipment,
                    equipment_label: equipment
                        .map(|kind| kind.label().into())
                        .unwrap_or_else(|| "空槽".into()),
                    equipment_emoji: equipment
                        .map(|kind| kind.emoji().into())
                        .unwrap_or_default(),
                    equipment_level: level,
                    equipment_max_level: WORKSHOP_EQUIPMENT_MAX_LEVEL,
                    equipment_speed_pct: workshop_state.equipment_speed_pct(index),
                    equipment_yield_pct: workshop_state.equipment_yield_pct(index),
                    upgrade_cost,
                    can_upgrade,
                    upgrade_missing,
                    job,
                }
            })
            .collect();
        let has_empty_slot =
            (1..workshop_state.slot_count()).any(|slot| workshop_state.equipment(slot).is_none());
        let equipment_options: Vec<WorkshopEquipmentView> = EquipmentKind::INSTALLABLE
            .iter()
            .map(|kind| {
                let installed = workshop_state
                    .equipment
                    .iter()
                    .flatten()
                    .any(|equipment| equipment == kind);
                let can_install = workshop_state.built
                    && !installed
                    && has_empty_slot
                    && self.state.level >= kind.level()
                    && self.state.coins >= kind.cost();
                let missing = if installed {
                    "已装".into()
                } else if !workshop_state.built {
                    "先建制造屋".into()
                } else if !has_empty_slot {
                    "没有空槽".into()
                } else if self.state.level < kind.level() {
                    format!("需要农场 {} 级", kind.level())
                } else if self.state.coins < kind.cost() {
                    "金币不够".into()
                } else {
                    String::new()
                };
                WorkshopEquipmentView {
                    kind: *kind,
                    label: kind.label().into(),
                    emoji: kind.emoji().into(),
                    cost: kind.cost(),
                    level: kind.level(),
                    installed,
                    can_install,
                    missing,
                }
            })
            .collect();
        let equipment_recipes: Vec<WorkshopRecipeView> = EquipmentRecipe::ALL
            .iter()
            .map(|recipe| {
                let equipment = recipe.equipment();
                let installed = workshop_state
                    .equipment
                    .iter()
                    .flatten()
                    .any(|kind| *kind == equipment);
                let idle = workshop_state.first_idle_slot(equipment).is_some();
                let mut parts = Vec::new();
                let mut missing = Vec::new();
                for (key, need) in recipe.inputs() {
                    let have = inventory_count(&self.state, key);
                    let name = workshop_item_name(&self.content, key);
                    parts.push(format!("{name}×{need}"));
                    if have < *need {
                        missing.push(format!("缺{name}×{}", need - have));
                    }
                }
                if !installed {
                    missing.insert(0, format!("还没装{}", equipment.label()));
                } else if !idle {
                    missing.insert(0, format!("{}正忙", equipment.label()));
                }
                WorkshopRecipeView {
                    recipe: *recipe,
                    equipment,
                    equipment_label: equipment.label().into(),
                    label: recipe.label().into(),
                    input_text: parts.join(" + "),
                    duration_ms: recipe.duration_ms(),
                    output_name: recipe.label().into(),
                    output_price: craft_item(recipe.output_key())
                        .map(|item| item.price)
                        .unwrap_or(0),
                    can_start: workshop_state.built && installed && idle && missing.is_empty(),
                    missing: missing.join("、"),
                }
            })
            .collect();
        let workshop = WorkshopView {
            built: workshop_state.built,
            can_build: !workshop_state.built
                && self.state.level >= WORKSHOP_LEVEL
                && self.state.coins >= WORKSHOP_COST,
            cost: WORKSHOP_COST,
            level: WORKSHOP_LEVEL,
            slots: workshop_state.slot_count(),
            max_slots: WORKSHOP_MAX_SLOTS,
            can_expand: workshop_state.built && workshop_state.can_expand(),
            expand_cost: workshop_state.next_expand_cost(),
            expand_level: workshop_state.next_expand_level(),
            slot_views,
            equipment_options,
            manual_recipes,
            bait_recipes,
            fish_recipes,
            crop_recipes,
            equipment_recipes,
        };
        let fishing_state = &self.state.fishing;
        let player_fishing = &self.state.skills.fishing;
        let disciplines: Vec<DisciplineView> = crate::skills::DisciplineKind::ALL
            .iter()
            .map(|kind| {
                let d = self.state.skills.get(*kind);
                let skills = crate::skills::for_discipline(*kind)
                    .iter()
                    .map(|def| {
                        let level = d.skill_level(def.id);
                        let cost = crate::skills::skill_cost(level);
                        SkillView {
                            id: def.id.into(),
                            name: def.name.into(),
                            desc: def.desc.into(),
                            level,
                            max_level: def.max_level,
                            cost,
                            can_upgrade: level < def.max_level && d.points >= cost,
                        }
                    })
                    .collect();
                DisciplineView {
                    kind: *kind,
                    label: kind.label().into(),
                    level: d.level,
                    exp: d.exp,
                    exp_need: crate::skills::Discipline::need(d.level),
                    points: d.points,
                    skills,
                }
            })
            .collect();
        let buff_views: Vec<BuffView> = self
            .state
            .buffs
            .iter()
            .filter(|b| b.expires_at > now)
            .map(|b| BuffView {
                id: b.id.clone(),
                name: b.name.clone(),
                desc: b.desc.clone(),
                remaining_ms: (b.expires_at - now).max(0),
            })
            .collect();
        let buff_items: Vec<BuffItemView> = crate::buffs::BUFF_ITEMS
            .iter()
            .map(|def| BuffItemView {
                key: def.item_key.into(),
                name: def.name.into(),
                desc: def.desc.into(),
            })
            .collect();
        let skills_view = SkillsView {
            disciplines,
            buffs: buff_views,
            buff_items,
        };
        let spots = FishingSpot::ALL
            .iter()
            .map(|spot| FishingSpotView {
                spot: *spot,
                label: spot.label().into(),
                emoji: spot.emoji().into(),
                level: spot.unlock_level(),
                unlocked: fishing_state.spot_unlocked(player_fishing.level, *spot),
                fish_count: fishing::spot_fish_count(*spot),
            })
            .collect();
        let rods = RodKind::ALL
            .iter()
            .map(|rod| RodView {
                kind: *rod,
                label: rod.label().into(),
                cost: rod.cost(),
                level: rod.unlock_level(),
                owned: fishing_state.owns_rod(*rod),
                current: *rod == fishing_state.rod,
                can_buy: !fishing_state.owns_rod(*rod)
                    && fishing_state.rod_unlocked(player_fishing.level, *rod)
                    && self.state.coins >= rod.cost(),
                frame_pct: rod.frame_pct(),
                auto_minutes: rod.auto_interval_ms() / 60_000,
                wear: fishing_state.rod_wear(*rod),
                max_wear: rod.durability(),
                profit_label: rod.profit_label(),
                maintenance_cost: rod.maintenance_cost(),
                needs_maintenance: fishing_state.needs_maintenance(*rod),
            })
            .collect();
        let session = fishing_state.session.as_ref().map(|session| {
            let def = fishing::fish(&session.fish_id);
            let golden = fishing::golden_session(session.id, session.spot, fishing_state.rod);
            let mut params = fishing::params_with_reel(
                fishing_state.rod,
                def.unwrap_or(&fishing::FISH[0]),
                fishing_state.needs_maintenance(fishing_state.rod),
                session.personality,
                player_fishing.skill_level("reel"),
            );
            if golden {
                params.catch_ms = fishing::FISHING_GOLDEN_CATCH_MS;
            }
            FishingSessionView {
                id: session.id,
                spot: session.spot,
                spot_label: session.spot.label().into(),
                fish_id: session.fish_id.clone(),
                fish_name: def.map(|d| d.name).unwrap_or("???").into(),
                faces_right: fishing::faces_right(&session.fish_id),
                price: def.map(|d| d.price).unwrap_or(0),
                difficulty: def.map(|d| d.difficulty).unwrap_or(1),
                is_king: def.map(|d| d.rarity == FishRarity::King).unwrap_or(false),
                is_golden: golden,
                expires_at: session.expires_at,
                remaining_ms: (session.expires_at - now).max(0),
                expired: now > session.expires_at,
                params,
            }
        });
        let auto_speed_level = player_fishing.skill_level("auto_speed");
        let auto_interval =
            fishing::auto_interval_ms_with_skill(fishing_state.auto.rod, auto_speed_level);
        let auto_interval_label = format!("{} 分钟", auto_interval / 60_000);
        let auto = FishingAutoView {
            enabled: fishing_state.auto.enabled,
            spot: fishing_state.auto.spot,
            spot_label: fishing_state.auto.spot.label().into(),
            rod: fishing_state.auto.rod,
            rod_label: fishing_state.auto.rod.label().into(),
            bait_quality: fishing_state.auto.bait_quality,
            bait_quality_label: fishing_state.auto.bait_quality.label().into(),
            bait_type: fishing_state.auto.bait_type,
            bait_type_label: fishing_state.auto.bait_type.label().into(),
            interval_ms: auto_interval,
            interval_label: auto_interval_label.clone(),
            next_at: if fishing_state.auto.enabled {
                fishing_state.auto.last_at + auto_interval
            } else {
                0
            },
            can_toggle: fishing_state.spot_unlocked(player_fishing.level, fishing_state.auto.spot)
                && (fishing_state.bait_count_of(
                    fishing_state.auto.bait_type,
                    fishing_state.auto.bait_quality,
                ) > 0
                    || self.state.coins
                        >= fishing_state.auto.bait_quality.price().saturating_mul(10)
                    || fishing_state.auto.enabled),
        };
        let mut codex_caught = 0usize;
        let fish_codex: Vec<FishCodexView> = fishing::fish_list()
            .iter()
            .map(|def| {
                let entry = fishing_state.codex.get(def.id);
                let caught = entry.map(|e| e.count > 0).unwrap_or(false);
                if caught {
                    codex_caught += 1;
                }
                FishCodexView {
                    id: def.id.to_string(),
                    name: def.name.to_string(),
                    spot: def.spot,
                    spot_label: def.spot.label().into(),
                    rarity: def.rarity,
                    rarity_label: def.rarity.label().into(),
                    price: def.price,
                    difficulty: def.difficulty,
                    caught,
                    count: entry.map(|e| e.count).unwrap_or(0),
                    best_weight_tenths: entry.map(|e| e.best_weight_tenths).unwrap_or(0),
                    best_quality: entry.map(|e| e.best_quality).unwrap_or(0),
                    best_quality_label: fishing::quality_label(
                        entry.map(|e| e.best_quality).unwrap_or(0),
                    )
                    .into(),
                    best_price: entry.map(|e| e.best_price).unwrap_or(0),
                    golden: fishing_state
                        .golden_codex
                        .get(def.id)
                        .is_some_and(|count| *count > 0),
                    golden_count: fishing_state.golden_codex.get(def.id).copied().unwrap_or(0),
                    manual_count: entry.map(|e| e.manual_count).unwrap_or(0),
                    manual_best_weight_tenths: entry
                        .map(|e| e.manual_best_weight_tenths)
                        .unwrap_or(0),
                    manual_best_quality: entry.map(|e| e.manual_best_quality).unwrap_or(0),
                    manual_best_quality_label: fishing::quality_label(
                        entry.map(|e| e.manual_best_quality).unwrap_or(0),
                    )
                    .into(),
                    manual_best_price: entry.map(|e| e.manual_best_price).unwrap_or(0),
                    auto_count: entry.map(|e| e.auto_count).unwrap_or(0),
                    auto_best_weight_tenths: entry.map(|e| e.auto_best_weight_tenths).unwrap_or(0),
                    auto_best_quality: entry.map(|e| e.auto_best_quality).unwrap_or(0),
                    auto_best_quality_label: fishing::quality_label(
                        entry.map(|e| e.auto_best_quality).unwrap_or(0),
                    )
                    .into(),
                    auto_best_price: entry.map(|e| e.auto_best_price).unwrap_or(0),
                    condition_hint: fishing::condition_hint(def.id).into(),
                }
            })
            .collect();
        let last_catch = fishing_state.last_catch.as_ref().and_then(|catch| {
            let def = fishing::fish(&catch.fish_id)?;
            Some(FishCatchView {
                fish_id: catch.fish_id.clone(),
                name: def.name.to_string(),
                weight_tenths: catch.weight_tenths,
                quality: catch.quality,
                quality_label: fishing::quality_label(catch.quality).into(),
                price: catch.price,
                market_pct: catch.market_pct,
                is_king: def.rarity == FishRarity::King,
                golden: catch.golden,
                is_record: catch.is_record,
                at: catch.at,
            })
        });
        let spot_progress: Vec<SpotProgressView> = fishing::FishingSpot::ALL
            .iter()
            .map(|spot| {
                let (reward_coins, bait_type, quality, qty) = fishing::spot_reward(*spot);
                SpotProgressView {
                    spot: *spot,
                    label: spot.label().into(),
                    caught: fishing_state.spot_caught_count(*spot),
                    total: fishing::spot_fish_count(*spot),
                    reward_coins,
                    reward_bait_label: format!("{}{}×{qty}", bait_type.label(), quality.label()),
                    claimed: fishing_state.spot_rewards.contains(spot.key()),
                    complete: fishing_state.spot_complete(*spot),
                }
            })
            .collect();
        let king_trophies: Vec<KingTrophyView> = fish_codex
            .iter()
            .filter(|view| view.rarity == FishRarity::King)
            .map(|view| KingTrophyView {
                fish_id: view.id.clone(),
                name: view.name.clone(),
                spot_label: view.spot_label.clone(),
                caught: view.caught,
                count: view.count,
                best_weight_tenths: view.best_weight_tenths,
            })
            .collect();
        let golden_caught = fishing_state
            .golden_codex
            .values()
            .filter(|count| **count > 0)
            .count();
        let bait_counts = fishing::BaitQuality::ALL
            .iter()
            .map(|quality| BaitView {
                bait_type: fishing_state.active_bait_type,
                quality: *quality,
                label: quality.label().into(),
                price: quality.price(),
                count: fishing_state.bait_count_of(fishing_state.active_bait_type, *quality),
                quality_bonus: quality.quality_bonus(),
                weight_bonus: quality.weight_bonus(),
            })
            .collect();
        let bait_types = fishing::BaitType::ALL
            .iter()
            .map(|bait_type| BaitTypeView {
                bait_type: *bait_type,
                label: bait_type.label().into(),
            })
            .collect();
        let last_auto_report = fishing_state.last_auto_report.as_ref().map(|report| {
            let catches: Vec<AutoCatchView> = report
                .catches
                .iter()
                .filter_map(|(id, count)| {
                    let def = fishing::fish(id)?;
                    Some(AutoCatchView {
                        fish_id: id.clone(),
                        name: def.name.to_string(),
                        count: *count,
                    })
                })
                .collect();
            let total_caught = catches.iter().map(|catch| catch.count).sum();
            AutoReportView {
                at: report.at,
                bait_bought: report.bait_bought,
                bait_used: report.bait_used,
                coins_spent: report.coins_spent,
                catches,
                total_caught,
            }
        });
        let fishing_skills = skills_view
            .disciplines
            .iter()
            .find(|d| d.kind == crate::skills::DisciplineKind::Fishing)
            .map(|d| d.skills.clone())
            .unwrap_or_default();
        let condition = fishing::FishingCondition::at(&self.state.daily.day, now);
        let condition_view = FishingConditionView {
            season_label: condition.season.label().into(),
            time_label: condition.time_label().into(),
            weather_label: condition.weather_label().into(),
            summary: condition.summary(),
        };
        let quest_state = &fishing_state.quests;
        let to_commission = |c: &crate::fishing::FishingCommission| FishingCommissionView {
            id: c.id.clone(),
            npc_name: c.npc_name.clone(),
            title: c.title.clone(),
            fish_name: c.fish_name.clone(),
            spot_label: c
                .spot
                .map(|spot| spot.label().to_string())
                .unwrap_or_default(),
            min_weight_tenths: c.min_weight_tenths,
            qty: c.qty,
            progress: c.progress,
            complete: c.complete(),
            reward_coins: c.reward_coins,
            reward_bait: c.reward_bait,
            reward_exp: c.reward_exp,
        };
        let quests = FishingQuestView {
            offers: quest_state.offers.iter().map(&to_commission).collect(),
            active: quest_state.active.iter().map(&to_commission).collect(),
            tournament: quest_state
                .tournament
                .as_ref()
                .map(|tournament| FishingTournamentView {
                    day: tournament.day.clone(),
                    npc_name: tournament.npc_name.clone(),
                    npc_best_tenths: tournament.npc_best_tenths,
                    best_tenths: tournament.best_tenths,
                    best_fish_name: tournament.best_fish_name.clone(),
                }),
            results: quest_state
                .results
                .iter()
                .rev()
                .map(|result| FishingTournamentResultView {
                    day: result.day.clone(),
                    message: result.message.clone(),
                    rank: result.rank,
                    reward_coins: result.reward_coins,
                    reward_bait: result.reward_bait,
                    reward_exp: result.reward_exp,
                    claimed: result.claimed,
                    best_tenths: result.best_tenths,
                    npc_best_tenths: result.npc_best_tenths,
                })
                .collect(),
            can_refresh: quest_state.refreshes == 0,
            refresh_used: quest_state.refreshes > 0,
        };
        let fishing = FishingView {
            rod: fishing_state.rod,
            rod_label: fishing_state.rod.label().into(),
            frame_pct: fishing_state.rod.frame_pct(),
            level: player_fishing.level,
            exp: player_fishing.exp,
            exp_need: fishing::fishing_exp_need(player_fishing.level),
            skill_points: player_fishing.points,
            skills: fishing_skills,
            auto_interval_ms: auto_interval,
            auto_interval_label: auto_interval_label.clone(),
            bait: fishing_state.bait,
            bait_price: fishing::FISHING_BAIT_PRICE,
            active_bait: fishing_state.active_bait,
            active_bait_label: fishing_state.active_bait.label().into(),
            active_bait_type: fishing_state.active_bait_type,
            active_bait_type_label: fishing_state.active_bait_type.label().into(),
            bait_types,
            bait_counts,
            rod_wear: fishing_state.rod_wear(fishing_state.rod),
            rod_max_wear: fishing_state.rod.durability(),
            maintenance_cost: fishing_state.rod.maintenance_cost(),
            needs_maintenance: fishing_state.needs_maintenance(fishing_state.rod),
            locked_fish: fishing_state.locked_fish.iter().cloned().collect(),
            spots,
            rods,
            session,
            auto,
            codex: fish_codex,
            codex_caught,
            codex_total: fishing::FISH.len(),
            spot_progress,
            golden_caught,
            golden_total: fishing::FISH.len(),
            king_trophies,
            full_codex_claimed: fishing_state.full_codex_claimed,
            last_catch,
            last_auto_report,
            quests,
            condition: condition_view,
        };
        let order_view = |order: &crate::ranch_orders::RanchOrder| {
            let key = order.item_key();
            let have = self.state.inventory.get(&key).copied().unwrap_or(0);
            RanchOrderView {
                id: order.id.clone(),
                npc_name: order.npc_name.clone(),
                base_key: order.base_key.clone(),
                item_name: order.item_name.clone(),
                quality: order.quality,
                quality_label: order.quality_label().into(),
                qty: order.qty,
                expires_day: order.expires_day.clone(),
                reward_coins: order.reward_coins,
                reward_exp: order.reward_exp,
                reward_feed: order.reward_feed,
                have,
                can_deliver: have >= order.qty,
            }
        };
        let ranch_orders = RanchOrdersView {
            day: self.state.ranch_orders.day.clone(),
            offers: self
                .state
                .ranch_orders
                .offers
                .iter()
                .map(&order_view)
                .collect(),
            active: self
                .state
                .ranch_orders
                .active
                .iter()
                .map(&order_view)
                .collect(),
            max_active: crate::ranch_orders::RANCH_ORDER_MAX_ACTIVE,
        };

        let mut current_combos = std::collections::BTreeSet::new();
        for pen in &self.state.ranch.pens {
            if let Some(animal) = pen.animal.as_ref() {
                current_combos.insert(format!("{}:{}", animal.kind.key(), animal.temper.key()));
            }
        }
        let codex_kinds: Vec<RanchCodexKindView> = RanchAnimalKind::ALL
            .iter()
            .map(|kind| {
                let mut done = 0;
                let traits: Vec<RanchCodexTraitView> = RanchTrait::ALL
                    .iter()
                    .map(|ranch_trait| {
                        let key = format!("{}:{}", kind.key(), ranch_trait.key());
                        let have = self.state.ranch.codex_animals.contains(&key)
                            || current_combos.contains(&key);
                        if have {
                            done += 1;
                        }
                        RanchCodexTraitView {
                            key,
                            label: ranch_trait.label().into(),
                            have,
                        }
                    })
                    .collect();
                RanchCodexKindView {
                    kind: *kind,
                    label: kind.label().into(),
                    emoji: kind.emoji().into(),
                    traits,
                    done,
                    total: RanchTrait::ALL.len() as u32,
                }
            })
            .collect();
        let codex_specials: Vec<RanchCodexSpecialView> = RanchAnimalKind::ALL
            .iter()
            .map(|kind| {
                let key = kind.special_key();
                let have = self.state.ranch.codex_specials.contains(key)
                    || self.state.inventory.get(key).copied().unwrap_or(0) > 0;
                RanchCodexSpecialView {
                    key: key.into(),
                    name: kind.special_name().into(),
                    have,
                }
            })
            .collect();
        let animal_done = codex_kinds.iter().map(|kind| kind.done).sum();
        let special_done = codex_specials.iter().filter(|item| item.have).count() as u32;
        let ranch_codex = RanchCodexView {
            kinds: codex_kinds,
            specials: codex_specials,
            animal_done,
            animal_total: (RanchAnimalKind::ALL.len() * RanchTrait::ALL.len()) as u32,
            special_done,
            special_total: RanchAnimalKind::ALL.len() as u32,
            medals: self.state.ranch_contest.medals,
        };

        let contest = RanchContestView {
            week: self.state.ranch_contest.week.clone(),
            entered: self.state.ranch_contest.entered,
            result: self
                .state
                .ranch_contest
                .result
                .as_ref()
                .map(|result| RanchContestResultView {
                    week: result.week.clone(),
                    animal_name: result.animal_name.clone(),
                    kind_label: result.kind_label.clone(),
                    score: result.score,
                    rank: result.rank,
                    reward_coins: result.reward_coins,
                    medal: result.medal,
                }),
            medals: self.state.ranch_contest.medals,
            best_rank: self.state.ranch_contest.best_rank,
        };

        let ranch = RanchView {
            can_expand: self.state.ranch.can_expand(),
            expand_cost: self.state.ranch.next_expand_cost(),
            max_pens: RANCH_MAX_PENS,
            feed_count: premium_feed,
            orders: ranch_orders,
            codex: ranch_codex,
            contest,
            pens: self
                .state
                .ranch
                .pens
                .iter()
                .map(|pen| RanchPenView {
                    index: pen.index,
                    cleanliness: pen.cleanliness,
                    can_clean: pen.can_clean(&self.state.daily.day),
                    animal: pen.animal.as_ref().map(|animal| RanchAnimalView {
                        kind: animal.kind,
                        label: animal.kind.label().into(),
                        emoji: animal.kind.emoji().into(),
                        name: animal.name.clone(),
                        satiety: animal.satiety,
                        health: animal.health,
                        sick: animal.sick,
                        product: animal.kind.product_name().into(),
                        product_ready_at: animal.product_ready_at,
                        remaining_ms: (animal.product_ready_at - now).max(0),
                        can_collect: now >= animal.product_ready_at,
                        can_feed: has_feed && !animal.sick,
                        can_heal: animal.sick,
                        quality: animal.quality,
                        quality_label: animal.quality_label().into(),
                        generation: animal.generation,
                        trait_label: animal.temper.label().into(),
                        trait_hint: animal.temper.hint().into(),
                        age_days: ((now - animal.adopted_at).max(0) / 86_400_000) as u32,
                        special_name: animal.kind.special_name().into(),
                        special_hint: format!(
                            "品质优良 {}%、精品 {}%",
                            crate::ranch::special_chance(2),
                            crate::ranch::special_chance(3)
                        ),
                        line_id: animal.line_id,
                        line_label: format!("血统 #{}", animal.line_id),
                        young: animal.is_young(now),
                        young_remaining_ms: animal.young_remaining(now),
                        inbred: animal.inbred,
                        parents: animal.parents.clone(),
                        breed_ready: animal.breed_ready(now),
                        breed_remaining_ms: animal.breed_remaining(now),
                    }),
                })
                .collect(),
        };
        let pond_view = if let Some(pond) = self.state.pond.pond() {
            let today = &self.state.daily.day;
            let feed_today = if pond.last_feed_day == *today {
                pond.feed_today
            } else {
                0
            };
            let fish_views: Vec<PondFishView> = pond
                .fish
                .iter()
                .map(|fish| {
                    let def = fishing::fish(&fish.fish_id);
                    let market = crate::pond::market_weight_tenths(&fish.fish_id);
                    PondFishView {
                        fish_id: fish.fish_id.clone(),
                        name: def
                            .map(|def| def.name)
                            .unwrap_or(&fish.fish_id)
                            .to_string(),
                        count: fish.count,
                        weight_tenths: fish.weight_tenths,
                        weight_label: format!("{:.1} kg", f64::from(fish.weight_tenths) / 10.0),
                        quality: fish.quality,
                        quality_label: crate::ranch::quality_name(fish.quality).into(),
                        market_weight_tenths: market,
                        market_weight_label: format!("{:.1} kg", f64::from(market) / 10.0),
                        ready: fish.weight_tenths >= market,
                        progress_pct: if market == 0 {
                            100
                        } else {
                            fish.weight_tenths.saturating_mul(100).min(market * 100) / market
                        },
                    }
                })
                .collect();
            let stock: Vec<PondStockView> = self
                .state
                .inventory
                .iter()
                .filter(|(key, qty)| key.starts_with("fish:") && **qty > 0)
                .filter_map(|(key, qty)| {
                    let id = key.strip_prefix("fish:")?;
                    let def = fishing::fish(id)?;
                    if def.rarity == fishing::FishRarity::King {
                        return None;
                    }
                    Some(PondStockView {
                        fish_id: id.to_string(),
                        name: def.name.to_string(),
                        have: *qty,
                    })
                })
                .collect();
            let score = pond.water_score();
            let status_label = if score >= 80 {
                "清澈"
            } else if score >= 60 {
                "一般"
            } else if score >= 40 {
                "发浑"
            } else {
                "很差"
            };
            let ready = pond
                .fish
                .iter()
                .find(|fish| fish.weight_tenths >= crate::pond::market_weight_tenths(&fish.fish_id));
            let hint: String = if pond.fish.is_empty() {
                if self
                    .state
                    .inventory
                    .keys()
                    .any(|key| key.starts_with("fish:"))
                {
                    "背包里有鱼，放几条进来".to_string()
                } else {
                    "先去钓鱼，再把鱼放进来".to_string()
                }
            } else if let Some(fish) = ready {
                let name = fishing::fish(&fish.fish_id)
                    .map(|def| def.name)
                    .unwrap_or(&fish.fish_id);
                format!("{name} 已经够大了，可以捕")
            } else if pond.oxygen < 30 {
                "鱼浮头了：装增氧机或清理鱼塘".to_string()
            } else if pond.cleanliness < 40 || pond.ammonia > 70 {
                "水浑了，清理一下鱼塘".to_string()
            } else if feed_today == 0 {
                "今天还没投喂，喂一次长得更快".to_string()
            } else {
                "水质稳定，等鱼长大".to_string()
            };
            let order = self.state.pond.order.as_ref().map(|order| PondOrderView {
                fish_name: order.fish_name.clone(),
                qty: order.qty,
                min_weight_label: format!("{:.1} kg", f64::from(order.min_weight_tenths) / 10.0),
                reward_coins: order.reward_coins,
                have: order.have(pond).0,
                can_deliver: order.can_deliver(pond),
            });
            let milestones: Vec<PondMilestoneView> = [
                (
                    crate::pond::POND_MILESTONE_FIRST_STOCK,
                    "第一次放养",
                ),
                (
                    crate::pond::POND_MILESTONE_FIRST_HARVEST,
                    "第一次捕捞",
                ),
                (crate::pond::POND_MILESTONE_FIRST_FRY, "第一窝鱼苗"),
                (crate::pond::POND_MILESTONE_HUNDRED, "累计捕 100 条"),
            ]
            .iter()
            .map(|(key, label)| PondMilestoneView {
                key: (*key).to_string(),
                label: (*label).to_string(),
                done: self.state.pond.milestone(key),
            })
            .collect();
            PondView {
                built: true,
                build_level: crate::pond::POND_BUILD_LEVEL,
                build_cost: crate::pond::POND_BUILD_COST,
                can_build: false,
                build_missing: String::new(),
                water: Some(PondWaterView {
                    temperature_tenths: pond.temperature_tenths,
                    temperature_label: format!("{:.1}°C", f64::from(pond.temperature_tenths) / 10.0),
                    oxygen: pond.oxygen,
                    cleanliness: pond.cleanliness,
                    ammonia: pond.ammonia,
                    score,
                }),
                status_label: status_label.to_string(),
                hint,
                fish: fish_views,
                stock,
                capacity: crate::pond::POND_CAPACITY,
                stock_count: pond.total_count(),
                species_cap: crate::pond::POND_SPECIES_CAP,
                species_count: pond.species_count(),
                aerator: pond.aerator,
                plants: pond.plants,
                aerator_cost: crate::pond::POND_AERATOR_COST,
                plants_cost: crate::pond::POND_PLANTS_COST,
                feed_today,
                feed_max: crate::pond::POND_FEED_PER_DAY,
                can_feed: feed_today < crate::pond::POND_FEED_PER_DAY,
                can_clean: today.is_empty() || pond.last_clean_day != *today,
                auto_fry: pond.auto_fry,
                order,
                order_done: self.state.pond.order_done,
                milestones,
            }
        } else {
            let level_ok = self.state.level >= crate::pond::POND_BUILD_LEVEL;
            let coins_ok = self.state.coins >= crate::pond::POND_BUILD_COST;
            PondView {
                built: false,
                build_level: crate::pond::POND_BUILD_LEVEL,
                build_cost: crate::pond::POND_BUILD_COST,
                can_build: level_ok && coins_ok,
                build_missing: if !level_ok {
                    format!("需要农场 {} 级", crate::pond::POND_BUILD_LEVEL)
                } else if !coins_ok {
                    format!("金币不够（{}）", crate::pond::POND_BUILD_COST)
                } else {
                    String::new()
                },
                water: None,
                status_label: String::new(),
                hint: "建好鱼塘后，把钓到的鱼放进来".to_string(),
                fish: Vec::new(),
                stock: Vec::new(),
                capacity: crate::pond::POND_CAPACITY,
                stock_count: 0,
                species_cap: crate::pond::POND_SPECIES_CAP,
                species_count: 0,
                aerator: false,
                plants: false,
                aerator_cost: crate::pond::POND_AERATOR_COST,
                plants_cost: crate::pond::POND_PLANTS_COST,
                feed_today: 0,
                feed_max: crate::pond::POND_FEED_PER_DAY,
                can_feed: false,
                can_clean: false,
                auto_fry: false,
                order: None,
                order_done: false,
                milestones: Vec::new(),
            }
        };
        let pet_care = PetCareView {
            dog_bed: self.state.pet_care.dog_bed,
            cat_bed: self.state.pet_care.cat_bed,
            dog_bed_broken: self.state.pet_care.dog_bed_broken,
            cat_bed_broken: self.state.pet_care.cat_bed_broken,
            bowls: self.state.pet_care.bowls,
            dog_toy: self.state.pet_care.dog_toy,
            cat_toy: self.state.pet_care.cat_toy,
            dog_toy_broken: self.state.pet_care.dog_toy_broken,
            cat_toy_broken: self.state.pet_care.cat_toy_broken,
            medicine: self.state.pet_care.medicine,
            cat_food: self.state.pet_care.cat_food,
            dog_food: self.state.pet_care.dog_food,
            dog_bone: self.state.pet_care.dog_bone,
            cat_can: self.state.pet_care.cat_can,
            cat_food_cost: PetFoodKind::CatFood.cost(),
            dog_food_cost: PetFoodKind::DogFood.cost(),
            dog_bone_cost: PetFoodKind::DogBone.cost(),
            cat_can_cost: PetFoodKind::CatCan.cost(),
            dog_bed_cost: PetSupplyKind::DogBed.cost(),
            cat_bed_cost: PetSupplyKind::CatBed.cost(),
            bowl_cost: PetSupplyKind::Bowl.cost(),
            dog_toy_cost: PetSupplyKind::DogToy.cost(),
            cat_toy_cost: PetSupplyKind::CatToy.cost(),
            medicine_cost: PetSupplyKind::Medicine.cost(),
            dog_bed_repair: PetSupplyKind::DogBed.repair_cost(),
            cat_bed_repair: PetSupplyKind::CatBed.repair_cost(),
            dog_toy_repair: PetSupplyKind::DogToy.repair_cost(),
            cat_toy_repair: PetSupplyKind::CatToy.repair_cost(),
        };

        let to_order_view = |order: &crate::orders::FarmOrder| {
            let key = order.delivery_key();
            let have = self.state.inventory.get(&key).copied().unwrap_or(0);
            FarmOrderView {
                id: order.id.clone(),
                npc_name: order.npc_name.clone(),
                crop_id: order.crop_id.clone(),
                crop_name: order.crop_name.clone(),
                item_key: key,
                item_name: order.item_name_or_crop().to_string(),
                qty: order.qty,
                expires_day: order.expires_day.clone(),
                reward_coins: order.reward_coins,
                reward_exp: order.reward_exp,
                have,
                can_deliver: have >= order.qty,
                is_craft: order.is_craft(),
                craft: order
                    .recipe
                    .map(|_| craft_order_track(self, order, now)),
            }
        };
        let orders = FarmOrdersView {
            day: self.state.orders.day.clone(),
            offers: self
                .state
                .orders
                .offers
                .iter()
                .map(&to_order_view)
                .collect(),
            active: self
                .state
                .orders
                .active
                .iter()
                .map(&to_order_view)
                .collect(),
            max_active: crate::orders::FARM_ORDER_MAX_ACTIVE,
        };
        let mystery = MysteryShopView {
            day: self.state.mystery.day.clone(),
            offers: self
                .state
                .mystery
                .offers
                .iter()
                .map(|offer| MysteryOfferView {
                    id: offer.id.clone(),
                    kind: offer.kind.label().to_string(),
                    name: offer.name.clone(),
                    desc: offer.desc.clone(),
                    tag: offer.tag.clone(),
                    price: offer.price,
                    rarity: match offer.rarity {
                        crate::mystery::MysteryRarity::Rare => "rare".into(),
                        crate::mystery::MysteryRarity::Legendary => "legendary".into(),
                    },
                    rarity_label: offer.rarity.label().to_string(),
                    featured: offer.featured,
                    sold: offer.sold,
                    affordable: self.state.coins >= offer.price,
                })
                .collect(),
            legendary: self
                .state
                .mystery
                .offers
                .iter()
                .any(|offer| offer.rarity == crate::mystery::MysteryRarity::Legendary),
        };

        Snapshot {
            rev: self.state.rev,
            now,
            season: self.state.season,
            season_label: self.state.season.label().into(),
            season_emoji: self.state.season.emoji().into(),
            region: self.state.settings.region,
            region_label: self.state.settings.region.label().into(),
            farm_name: self.state.farm_name.clone(),
            owner_name: self.state.owner_name.clone(),
            created_at: self.state.created_at,
            level: self.state.level,
            exp: self.state.exp,
            exp_need: level_need(self.state.level),
            coins: self.state.coins,
            plots,
            inventory,
            facilities,
            onboarded: self.state.settings.onboarded,
            daily: daily::view(&self.state.daily, self.state.ranch.has_animal()),
            orders,
            mystery,
            level_rewards: level_reward::views(&self.state, &self.content),
            level_rewards_claimable: level_reward::claimable_count(&self.state),
            codex,
            codex_discovered,
            achievements: achievement_views,
            achievements_earned,
            specimens,
            specimen_unique,
            specimen_featured: featured_specimen,
            specimen_milestones: specimen::MILESTONES
                .iter()
                .map(|(unique, reward_coins)| SpecimenMilestoneView {
                    unique: *unique,
                    reward_coins: *reward_coins,
                    claimed: self.state.specimen_milestones_claimed.contains(unique),
                    claimable: !self.state.specimen_milestones_claimed.contains(unique)
                        && specimen_unique >= *unique as usize,
                })
                .collect(),
            pet,
            pets,
            pet_care,
            ranch,
            pond: pond_view,
            workshop,
            fishing,
            room: self.room_view(crate::room::LIVING_ROOM, now).expect("living room"),
            extra_room: self.room_view(crate::room::EXTRA_ROOM, now),
            decor_shop: DecorShopView {
                day: self.state.room.decor_shop.day.clone(),
                offers: self
                    .state
                    .room
                    .decor_shop
                    .offers
                    .iter()
                    .map(|offer| DecorOfferView {
                        id: offer.id.clone(),
                        kind: offer.kind.label().into(),
                        name: offer.name.clone(),
                        price: offer.price,
                        sold: offer.sold,
                        owned: match offer.kind {
                            crate::decor::DecorKind::Floor => {
                                self.state.room.owned_floors.contains(&offer.item_id)
                            }
                            crate::decor::DecorKind::Wall => {
                                self.state.room.owned_walls.contains(&offer.item_id)
                            }
                        },
                    })
                    .collect(),
            },
            npc_rooms: self.state.npcs.iter().map(|entry| {
                let threshold = npc_threshold(&entry.id);
                NpcRoomView {
                    npc_id: entry.id.clone(),
                    name: entry.name.clone(),
                    relation: npc::relation(entry.friendship).into(),
                    friendship: entry.friendship,
                    threshold,
                    unlocked: entry.friendship >= threshold,
                    souvenirs: entry.souvenirs.clone(),
                    souvenir_items: entry
                        .souvenirs
                        .iter()
                        .map(|name| crate::npc::NpcSouvenirView {
                            name: name.clone(),
                            icon: crate::npc::souvenir_icon(name).to_string(),
                        })
                        .collect(),
                    room: room_view_data(
                        &format!("npc_{}", entry.id),
                        &format!("{}的小屋", entry.name),
                        12,
                        9,
                        npc_room_layout(&entry.id),
                    ),
                }
            }).collect(),
            skills: skills_view,
            boarding_debt: self.state.boarding_debt,
            exp_rate: self.state.settings.exp_rate,
            npcs,
            npc_messages,
            village_events,
            festival,
            steals_left: npc::steal_left(&self.state.daily),
            helps_left: npc::help_left(&self.state.daily),
            ranch_steals_left: npc::ranch_steal_left(&self.state.daily),
            ranch_helps_left: npc::ranch_help_left(&self.state.daily),
            log: self.state.log.clone(),
            content_crops: self.content.crops.len(),
        }
    }
}

fn inventory_count(state: &GameState, key: &str) -> u32 {
    if key.starts_with("ranch:") {
        (1..=3)
            .map(|tier| {
                state
                    .inventory
                    .get(&crate::ranch::with_quality(key, tier))
                    .copied()
                    .unwrap_or(0)
            })
            .sum()
    } else {
        state.inventory.get(key).copied().unwrap_or(0)
    }
}

fn workshop_item_name(content: &ContentPack, key: &str) -> String {
    if let Some(id) = key.strip_prefix("crop:") {
        content
            .crop(id)
            .map(|crop| crop.name.clone())
            .unwrap_or_else(|| id.to_string())
    } else if key.starts_with("craft:") {
        craft_item(key)
            .map(|item| item.name.to_string())
            .unwrap_or_else(|| key.to_string())
    } else if key.starts_with("ranch:") {
        crate::ranch::item_def(key)
            .map(|def| def.name.to_string())
            .unwrap_or_else(|| key.to_string())
    } else {
        key.to_string()
    }
}

fn workshop_job_view(job: &WorkshopJob, content: &ContentPack, now: i64) -> WorkshopJobView {
    match job {
        WorkshopJob::Ranch {
            recipe,
            quality,
            ready_at,
            ..
        } => {
            let suffix = if *quality > 1 {
                crate::ranch::quality_name(*quality)
            } else {
                ""
            };
            WorkshopJobView {
                kind: "ranch".into(),
                label: recipe.label().into(),
                is_crop: false,
                output_name: format!("{suffix}{}", recipe.output_name()),
                output_price: recipe.output_price(),
                quality: *quality,
                quality_label: crate::ranch::quality_name(*quality).into(),
                ready_at: *ready_at,
                remaining_ms: (*ready_at - now).max(0),
                can_collect: now >= *ready_at,
            }
        }
        WorkshopJob::CropBox {
            crop_id, ready_at, ..
        } => {
            let def = content.crop(crop_id);
            let name = def
                .map(|crop| crop.name.clone())
                .unwrap_or_else(|| crop_id.clone());
            WorkshopJobView {
                kind: "cropBox".into(),
                label: format!("{name}礼盒"),
                is_crop: true,
                output_name: format!("{name}礼盒"),
                output_price: def.map(|crop| crop_box_price(crop.sell_price)).unwrap_or(0),
                quality: 1,
                quality_label: "普通".into(),
                ready_at: *ready_at,
                remaining_ms: (*ready_at - now).max(0),
                can_collect: now >= *ready_at,
            }
        }
        WorkshopJob::Equipment {
            recipe, ready_at, ..
        } => WorkshopJobView {
            kind: "equipment".into(),
            label: recipe.label().into(),
            is_crop: false,
            output_name: recipe.label().into(),
            output_price: craft_item(recipe.output_key())
                .map(|item| item.price)
                .unwrap_or(0),
            quality: 1,
            quality_label: "普通".into(),
            ready_at: *ready_at,
            remaining_ms: (*ready_at - now).max(0),
            can_collect: now >= *ready_at,
        },
        WorkshopJob::Bait {
            recipe, ready_at, ..
        } => WorkshopJobView {
            kind: "bait".into(),
            label: recipe.label().into(),
            is_crop: false,
            output_name: format!("{}×{}", recipe.bait_type().label(), recipe.output_qty()),
            output_price: 0,
            quality: 1,
            quality_label: "普通".into(),
            ready_at: *ready_at,
            remaining_ms: (*ready_at - now).max(0),
            can_collect: now >= *ready_at,
        },
        WorkshopJob::Fish {
            recipe, ready_at, ..
        } => WorkshopJobView {
            kind: "fish".into(),
            label: recipe.label().into(),
            is_crop: false,
            output_name: format!("{}×{}", recipe.output_name(), recipe.output_qty()),
            output_price: craft_item(recipe.output_key())
                .map(|item| item.price)
                .unwrap_or(0),
            quality: 1,
            quality_label: "普通".into(),
            ready_at: *ready_at,
            remaining_ms: (*ready_at - now).max(0),
            can_collect: now >= *ready_at,
        },
    }
}

fn pet_view(pet: &PetState, now: i64) -> PetView {
    let touch_remaining = (PET_TOUCH_COOLDOWN_MS - (now - pet.last_pet_at)).max(0);
    let play_remaining = (PET_PLAY_COOLDOWN_MS - (now - pet.last_play_at)).max(0);
    let dig_remaining = (pet.dig_cooldown_ms() - (now - pet.last_dig_at)).max(0);
    let remaining = pet.skill_remaining_ms(now);
    let activity = pet.activity(now);
    let talent_tiers: Vec<PetTalentTierView> = (1..=3u8)
        .map(|tier| {
            let threshold = PetTalent::TIER_LEVELS[(tier - 1) as usize];
            let options = PetTalent::ALL
                .iter()
                .copied()
                .filter(|talent| talent.tier() == tier && talent.species() == pet.species)
                .map(|talent| PetTalentView {
                    id: talent,
                    label: talent.label().into(),
                    description: talent.description().into(),
                    selected: pet.talents.has(talent),
                })
                .collect();
            PetTalentTierView {
                tier,
                threshold,
                unlocked: pet.level >= threshold,
                options,
            }
        })
        .collect();
    PetView {
        id: pet.id.clone(),
        name: pet.name.clone(),
        species: pet.species,
        species_label: pet.species.label().into(),
        species_emoji: pet.species.emoji().into(),
        breed: pet.breed,
        breed_label: pet.breed.label().into(),
        image: pet.breed.image().into(),
        temperament: format!("{:?}", pet.temperament()).to_lowercase(),
        temperament_label: pet.temperament().label().into(),
        personality: pet.personality.label(),
        quirk: pet.personality.quirk.label().into(),
        quirk_line: pet.quirk_line().into(),
        skill: pet.breed.signature().name.into(),
        skill_description: pet.breed.signature().description.into(),
        active_skill: pet.active_skill().name.into(),
        active_description: pet.active_skill().description.into(),
        skill_remaining_ms: remaining,
        can_use_skill: !pet.boarded && remaining == 0,
        activity: format!("{:?}", activity).to_lowercase(),
        activity_label: activity.label().into(),
        activity_line: activity.line().into(),
        activity_streak: pet.activity_streak,
        mischief_chance: pet.mischief_chance(),
        behavior: pet.behavior_line(),
        dig_skill: pet.dig_skill,
        dig_exp: pet.dig_exp,
        dig_need: PetState::skill_need(pet.dig_skill),
        guard_skill: pet.guard_skill,
        guard_exp: pet.guard_exp,
        guard_need: PetState::skill_need(pet.guard_skill),
        guard_train: pet.training.guard,
        dig_train: pet.training.dig,
        mischief_train: pet.training.mischief,
        companion_train: pet.training.companion,
        training_spent: pet.training_spent(),
        training_cap: pet.training_cap(),
        training_center: false,
        talent_tiers,
        boarded: pet.boarded,
        boarding_fee: PET_BOARD_FEE_PER_MONTH,
        stage: pet.stage(now),
        stage_label: pet.stage(now).label().into(),
        age_days: pet.age_days(now),
        level: pet.level,
        exp: pet.exp,
        exp_need: pet.exp_need(),
        bond: pet.bond,
        satiety: pet.satiety,
        energy: pet.energy,
        mood: pet.mood,
        mood_label: pet.mood_label().into(),
        health: pet.health,
        sickness: pet.sickness.as_ref().map(|s| s.kind.label().into()),
        location: pet.location,
        location_label: pet.location.label().into(),
        job_level: pet.job.level,
        job_exp: pet.job.exp,
        job_need: PetJob::need(pet.job.level),
        fishing_job_level: pet.fishing_job.level,
        fishing_job_exp: pet.fishing_job.exp,
        fishing_job_need: PetJob::need(pet.fishing_job.level),
        can_feed: !pet.boarded,
        can_touch: !pet.boarded && touch_remaining == 0,
        can_play: !pet.boarded && play_remaining == 0 && pet.sickness.is_none() && pet.energy >= 15,
        can_heal: !pet.boarded && pet.sickness.is_some(),
        can_dig: !pet.boarded
            && dig_remaining == 0
            && pet.sickness.is_none()
            && pet.location == PetLocation::Farm
            && pet.energy >= 20,
        touch_remaining_ms: touch_remaining,
        play_remaining_ms: play_remaining,
        dig_remaining_ms: dig_remaining,
    }
}

impl Game {
    /// 两只动物的后代表现（只读）。与真繁殖共用 `breedSeq`，预览即下一发。
    pub fn preview_ranch_breeding(
        &self,
        a: u8,
        b: u8,
        now: i64,
    ) -> Result<RanchBreedPreview, GameError> {
        let animal_a = self
            .state
            .ranch
            .pen(a)
            .and_then(|pen| pen.animal.as_ref())
            .ok_or(GameError::RanchPenEmpty)?;
        let animal_b = self
            .state
            .ranch
            .pen(b)
            .and_then(|pen| pen.animal.as_ref())
            .ok_or(GameError::RanchPenEmpty)?;
        let blockers = self.ranch_breed_blockers(a, b, now)?;
        let plan = crate::ranch::breed_child(animal_a, animal_b, self.state.ranch.breed_seq);
        let line_id = if plan.line_id == 0 {
            self.state.ranch.line_seq + 1
        } else {
            plan.line_id
        };
        let inbred = a != b && plan.inbred;
        Ok(RanchBreedPreview {
            ok: blockers.is_empty(),
            kind: plan.kind,
            kind_label: plan.kind.label().into(),
            quality: plan.quality,
            quality_label: crate::ranch::quality_name(plan.quality).into(),
            generation: plan.generation,
            line_id,
            line_label: format!("血统 #{line_id}"),
            trait_label: plan.temper.label().into(),
            trait_hint: plan.temper.hint().into(),
            inbred,
            parents: plan.parents,
            warnings: if inbred {
                vec!["近亲繁殖：后代品质上限 −1，生病率 +8%".into()]
            } else {
                Vec::new()
            },
            missing: blockers.iter().map(|blocker| blocker.message()).collect(),
        })
    }
}
