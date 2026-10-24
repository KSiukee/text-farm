//! 文字农场领域内核：零 Tauri、零文件系统、零系统时间。
//!
//! 时间由调用方以毫秒时间戳传入；存档序列化在这里，落盘在壳里。

mod achievements;
mod action;
mod buffs;
mod content;
mod daily;
mod engine;
mod error;
mod fishing;
mod level_reward;
mod model;
mod mystery;
mod npc;
mod orders;
mod pet;
mod planting;
mod processing;
mod ranch;
mod ranching;
mod room;
mod save;
mod seedgen;
mod skills;
mod specimen;
mod view;
mod workshop;

pub use achievements::{AchievementView, ACHIEVEMENTS};
pub use action::{Action, FacilityKind};
pub use buffs::{
    buff_item, craft_speed_pct as buff_craft_speed_pct, exp_pct as buff_exp_pct,
    plant_yield_pct as buff_plant_yield_pct, ranch_quality_pct as buff_ranch_quality_pct, Buff,
    BuffDef, BuffStat, BUFF_ITEMS,
};
pub use content::{
    economy_preview, ContentPack, CropDef, EconomyPreview, Nouns, RegionAffinity, Resistances,
    CONTENT_SCHEMA,
};
pub use daily::{DailyTaskView, DailyView, DAILY_TASKS};
pub use engine::{Effect, Outcome};
pub use error::{GameError, SaveError};
pub use room::{
    default_extra_layout, default_layout, floor_styles, furniture_def, room_size, validate_layout,
    wall_styles, window_style, window_styles, FurnitureDef, RoomDoor, RoomFurniture, RoomLayout,
    RoomState, RoomSurfaceItem, RoomWindow, DOOR_HEIGHT, DOOR_WIDTH, EXTRA_HEIGHT, EXTRA_ROOM, EXTRA_ROOM_COST,
    EXTRA_WIDTH, FURNITURE, LIVING_HEIGHT, LIVING_ROOM, LIVING_WIDTH, MAX_WINDOWS, ROOM_GRID,
    WALL_HEIGHT_TILES, WALL_HEIGHT_UNITS,
};
pub use fishing::{
    apply_manual_bonus, auto_interval_ms_with_skill, bait_saver_pct, bait_weight,
    big_fish_bonus_tenths, catch_exp, condition_hint, condition_weight, faces_right, fish,
    fish_behavior, fish_list, fishing_exp_need, golden_floor_price,
    golden_session, king_base_price, market_pct, params, pet_fish_pool, pick_auto_fish,
    pick_auto_fish_with_bait, pick_auto_fish_with_bait_condition, pick_fish, pick_fish_with_bait,
    pick_fish_with_bait_condition, pick_from_pool, pick_personality,
    price_for, quality_for, quality_label,
    reel_bonus, rod_exp, rod_unlocked,
    sell_price, skill_cost, skill_def, skill_level, spot_fish, spot_fish_count, spot_unlocked,
    weight_tenths_for,
    AutoFishing, BaitQuality, BaitRecipe, BaitType, FishBatch, FishBehavior, FishCatchResult,
    FishCodexEntry, FishConditionTag, FishDef, FishPersonality, FishRarity, FishingCommission,
    FishingCondition, FishingParams, FishingQuestState, FishingSession, FishingSkillDef,
    FishingTournament, FishingTournamentResult, FishingSpot, FishingState, FishingStats, RodKind,
    FISH, FISH_CONDITION_WEIGHTS, FISHING_AUTO_CAP_MS,
    FISHING_AUTO_MAX_STEPS, FISHING_BAIT_PRICE, FISHING_GOLDEN_CATCH_MS, FISHING_KING_CATCH_MS,
    FISHING_MAX_LEVEL, FISHING_SESSION_TTL_MS,
    FISHING_SKILLS, FISHING_SKILL_MAX, FISHING_START_BAIT, MANUAL_QUALITY_BONUS,
    MANUAL_WEIGHT_BONUS_TENTHS,
};
pub use level_reward::{LevelRewardView, LEVEL_REWARD_MIN};
pub use model::{
    facility_cost, facility_name, facility_unlock_level, level_need, plot_tier, plot_unlock_cost,
    plot_unlock_level, CodexEntry, CropInstance, DailyState, Disaster, DisasterKind, Facilities,
    Game, GameState, LogEntry, Pest, PestKind, Plot, PlotKind, PlotTier, Region, Season, Settings,
    Stats, MAX_LEVEL, PLOT_COUNT, PLOT_MAX_TIER, SAVE_SCHEMA,
};
pub use mystery::{
    MysteryKind, MysteryOffer, MysteryRarity, MysteryShopState, MYSTERY_LEGENDARY_PCT,
    MYSTERY_OFFERS,
};
pub use npc::{
    NpcMessage, NpcMessageView, NpcPlot, NpcPlotView, NpcState, NpcView, HELP_DAILY_LIMIT,
    NPC_VISIT_CAP, STEAL_DAILY_LIMIT,
};
pub use pet::{
    guard_result, guard_result_full, PetActivity, PetBreed, PetCare, PetLocation, PetPersonality,
    PetQuirk, PetSickness, PetSicknessKind, PetSpecies, PetStage, PetState, PetSupplyKind,
    PetTalent, PetTalents, PetTraining, PetTrainingKind, PetTrait, PET_ACTIVE_LIMIT,
    PET_BOARD_FEE_PER_MONTH, PET_DIG_COOLDOWN_MS, PET_MAX_LEVEL, PET_PLAY_COOLDOWN_MS,
    PET_TOUCH_COOLDOWN_MS,
};
pub use planting::{
    growth_pct as planting_growth_pct, harvest_exp, seed_return_pct, water_extra_pct, yield_bonus,
    yield_pct as planting_yield_pct,
};
pub use processing::{
    collect_exp as processing_collect_exp, quality_chance as processing_quality_chance,
    save_chance as processing_save_chance, speed_pct as processing_speed_pct,
    yield_chance as processing_yield_chance,
};
pub use ranch::{
    RanchAnimal, RanchAnimalKind, RanchPen, RanchRecipe, RanchState, RANCH_EXPAND_COSTS,
    RANCH_PEN_COUNT, RANCH_QUALITY_PCT,
};
pub use ranching::{
    care_pct, collect_exp as ranching_collect_exp, feed_save_chance,
    quality_chance as ranching_quality_chance, yield_chance as ranching_yield_chance,
};
pub use save::{migrate_save, SaveFile};
pub use orders::{
    add_days, FarmOrder, FarmOrderState, FARM_ORDER_MAX_ACTIVE, FARM_ORDER_OFFERS,
};
pub use seedgen::{
    generate as generate_seeds, review_batch, review_candidates, ReviewStatus, SeedMetrics,
    SeedReview,
};
pub use skills::{
    for_discipline, Discipline, DisciplineKind, PlayerSkills, SkillDef, DISCIPLINE_MAX_LEVEL,
    PROFESSION_EXP_BASE, PROFESSION_EXP_SLOPE, SKILL_MAX_LEVEL,
};
pub use specimen::{recipe as specimen_recipe, SpecimenRecipe};
pub use view::{
    BaitRecipeView, BaitTypeView, BuffItemView, BuffView, CodexView, CropView, DisasterView,
    DisciplineView, FacilityView, FishRecipeView, ItemKind, ItemStack, KingTrophyView, PestView,
    NpcRoomView, PetTalentTierView, PetTalentView, PetView, PlotEffectsView, PlotStatus, PlotView,
    RoomView, RoomWindowStyleView,
    SkillView, SkillsView, Snapshot, SpecimenView, SpotProgressView,
};
pub use workshop::{
    craft_item, crop_box_key, crop_box_price, workshop_margin, workshop_price, CraftItemDef,
    EquipmentKind, EquipmentRecipe, FishRecipe, WorkshopJob, WorkshopState, CRAFT_ITEMS,
    CROP_BOX_INPUT, CROP_BOX_INTERVAL_MS, WORKSHOP_BASE_SLOTS, WORKSHOP_COST,
    WORKSHOP_EQUIPMENT_MAX_LEVEL, WORKSHOP_LEVEL, WORKSHOP_LEVEL_SPEED_PCT,
    WORKSHOP_LEVEL_YIELD_PCT, WORKSHOP_MAX_SLOTS, WORKSHOP_PROFIT_PER_HOUR, WORKSHOP_SLOT_COSTS,
    WORKSHOP_SLOT_LEVELS,
};

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
