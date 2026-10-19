use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("存档版本比程序新：{0}")]
    TooNew(u32),
    #[error("存档版本不认识：{0}")]
    Unknown(u32),
    #[error("内容包不合法：{0}")]
    InvalidContent(String),
}

#[derive(Debug, Error)]
pub enum GameError {
    #[error("没有 {0} 号地")]
    NoSuchPlot(u8),
    #[error("这块地还没开垦")]
    PlotLocked,
    #[error("这块地已经种着东西")]
    PlotOccupied,
    #[error("这块地是空的")]
    PlotEmpty,
    #[error("作物已经枯萎了")]
    CropWithered,
    #[error("需要 {need} 级")]
    LevelTooLow { need: u32 },
    #[error("金币不够：需要 {need}")]
    NotEnoughCoins { need: u64 },
    #[error("种子不够")]
    NotEnoughSeeds,
    #[error("仓库里没有这么多")]
    NotEnoughItems,
    #[error("还没成熟")]
    NotReady,
    #[error("先清理草虫")]
    PestBlocking,
    #[error("这块地没有草虫")]
    NoPest,
    #[error("这块地没有灾害")]
    NoDisaster,
    #[error("冰雹已经造成损伤，无法处理")]
    HailNoAction,
    #[error("今天已经签到过了")]
    AlreadySignedIn,
    #[error("这个日常奖励已经领过了")]
    AlreadyClaimed,
    #[error("日常任务还没完成")]
    DailyNotDone,
    #[error("这块地已经升到最高级")]
    PlotMaxTier,
    #[error("没有这个日常任务")]
    BadDailyTask,
    #[error("没有这个等级的奖励")]
    BadLevelReward,
    #[error("经验倍率只能是 1、2、5")]
    InvalidExpRate,
    #[error("没有这个邻居")]
    UnknownNpc,
    #[error("邻居这块地是空的")]
    NpcPlotEmpty,
    #[error("邻居的作物还没熟")]
    NpcNotReady,
    #[error("这块地已经帮过忙了")]
    NpcAlreadyHelped,
    #[error("邻居牧场没有这只动物")]
    NpcRanchEmpty,
    #[error("这只动物已经帮过忙了")]
    NpcRanchAlreadyHelped,
    #[error("邻居牧场的产物还没好")]
    NpcRanchNotReady,
    #[error("今天牧场偷产物次数用完了")]
    NpcRanchStealLimit,
    #[error("今天牧场帮忙次数用完了")]
    NpcRanchHelpLimit,
    #[error("今天偷菜次数用完了")]
    NpcStealLimit,
    #[error("今天帮忙次数用完了")]
    NpcHelpLimit,
    #[error("给邻居留一点吧")]
    NpcLeaveSome,
    #[error("已经成熟了")]
    AlreadyReady,
    #[error("这季已经浇过水")]
    AlreadyWatered,
    #[error("这块地已经开垦过了")]
    AlreadyUnlocked,
    #[error("没有这种作物")]
    UnknownCrop,
    #[error("没有这个物品")]
    UnknownItem,
    #[error("只有果实能卖")]
    OnlyProduceSellable,
    #[error("这个钓点还没解锁，需要钓鱼等级 {need} 级")]
    FishingSpotLocked { need: u32 },
    #[error("鱼饵不够")]
    FishingBaitEmpty,
    #[error("已经有一竿在水里了")]
    FishingSessionActive,
    #[error("这一竿已经过期了")]
    FishingSessionExpired,
    #[error("没有进行中的钓鱼")]
    FishingNoSession,
    #[error("这根鱼竿还没解锁，需要钓鱼等级 {need} 级")]
    FishingRodLocked { need: u32 },
    #[error("已经有更好的鱼竿了")]
    FishingRodOwned,
    #[error("没有这种鱼")]
    UnknownFish,
    #[error("这条鱼已锁定，不会出售")]
    FishLocked,
    #[error("没有这个钓术")]
    BadFishingSkill,
    #[error("这个钓术已经满级")]
    FishingSkillMax,
    #[error("没有这个专精")]
    BadSkill,
    #[error("这个专精已经满级")]
    SkillMax,
    #[error("这个物品不能吃喝加成")]
    BadBuffItem,
    #[error("技能点不够：需要 {need}")]
    NotEnoughSkillPoints { need: u32 },
    #[error("没有可卖的东西")]
    NothingToSell,
    #[error("数量必须大于 0")]
    InvalidQty,
    #[error("没有可收的东西")]
    NothingToHarvest,
    #[error("需要先建收割机")]
    NeedHarvester,
    #[error("这个设施已经建过了")]
    FacilityOwned,
    #[error("设施已达到上限")]
    FacilityMax,
    #[error("没有损坏的风车")]
    NoDamagedWindmill,
    #[error("已经有宠物了")]
    PetExists,
    #[error("还没有宠物")]
    NoPet,
    #[error("宠物还在冷却中")]
    PetCooldown,
    #[error("宠物太累了")]
    PetTooTired,
    #[error("宠物生病了，先治疗它")]
    PetSick,
    #[error("这个地点还没开放")]
    PetLocationLocked,
    #[error("宠物没生病")]
    PetHealthy,
    #[error("宠物名字需要 1–12 个字")]
    InvalidPetName,
    #[error("最多同时养 3 只宠物，先寄养或放生一只")]
    PetLimit,
    #[error("找不到这只宠物")]
    PetNotFound,
    #[error("这只宠物在寄养中，先接回来")]
    PetBoarded,
    #[error("这个用品已经买了")]
    PetSupplyOwned,
    #[error("这个用品没有损坏")]
    PetSupplyNotBroken,
    #[error("当前没有寄养欠费")]
    NoBoardingDebt,
    #[error("这个畜栏没有动物")]
    RanchPenEmpty,
    #[error("这个畜栏已经有动物了")]
    RanchPenOccupied,
    #[error("产物还没准备好")]
    RanchProductNotReady,
    #[error("动物没有生病")]
    RanchAnimalHealthy,
    #[error("动物生病了，先治疗")]
    RanchAnimalSick,
    #[error("畜栏已经扩到上限")]
    RanchMaxPens,
    #[error("没有专用饲料")]
    RanchNoFeed,
    #[error("没有可繁殖的动物组合")]
    RanchNoBreedPair,
    #[error("没有空畜栏放幼崽")]
    RanchNoEmptyPen,
    #[error("已经建过制造屋了")]
    RanchWorkshopOwned,
    #[error("需要先建制造屋")]
    RanchWorkshopMissing,
    #[error("制造屋正忙，先取走这一份")]
    RanchCraftBusy,
    #[error("加工还没完成")]
    RanchCraftNotReady,
    #[error("制造屋没有正在加工的东西")]
    RanchCraftMissing,
    #[error("原料不够")]
    RanchIngredientsShort,
    #[error("加工槽已经扩到上限")]
    WorkshopMaxSlots,
    #[error("没有这个加工槽")]
    WorkshopSlotInvalid,
    #[error("这个加工槽已经装了设备")]
    WorkshopSlotOccupied,
    #[error("还没有装这台设备")]
    WorkshopEquipmentMissing,
    #[error("这个配方不属于这台设备")]
    WorkshopEquipmentMismatch,
    #[error("药品用完了，去宠物用品里买")]
    NoPetMedicine,
    #[error("主动技能还在冷却")]
    PetSkillCooldown,
    #[error("还没有训练中心")]
    TrainingLocked,
    #[error("训练点不够或这项已满级")]
    TrainingMax,
    #[error("设施需要 {need} 级")]
    FacilityLocked { need: u32 },
    #[error("内容包不合法：{0}")]
    InvalidContent(String),
    #[error("内容包要删除正在使用的作物：{0}")]
    ContentRemovesUsedCrop(String),
    #[error("广播最多 40 个字")]
    BroadcastTooLong,
    #[error("广播不能为空")]
    EmptyText,
    #[error("数值溢出了")]
    Overflow,
}

impl GameError {
    pub fn code(&self) -> &'static str {
        match self {
            GameError::NoSuchPlot(_) => "no_such_plot",
            GameError::PlotLocked => "plot_locked",
            GameError::PlotOccupied => "plot_occupied",
            GameError::PlotEmpty => "plot_empty",
            GameError::CropWithered => "crop_withered",
            GameError::LevelTooLow { .. } => "level_too_low",
            GameError::NotEnoughCoins { .. } => "not_enough_coins",
            GameError::NotEnoughSeeds => "not_enough_seeds",
            GameError::NotEnoughItems => "not_enough_items",
            GameError::NotReady => "not_ready",
            GameError::PestBlocking => "pest_blocking",
            GameError::NoPest => "no_pest",
            GameError::NoDisaster => "no_disaster",
            GameError::HailNoAction => "hail_no_action",
            GameError::AlreadySignedIn => "already_signed_in",
            GameError::AlreadyClaimed => "already_claimed",
            GameError::DailyNotDone => "daily_not_done",
            GameError::PlotMaxTier => "plot_max_tier",
            GameError::BadDailyTask => "bad_daily_task",
            GameError::BadLevelReward => "bad_level_reward",
            GameError::InvalidExpRate => "invalid_exp_rate",
            GameError::UnknownNpc => "unknown_npc",
            GameError::NpcPlotEmpty => "npc_plot_empty",
            GameError::NpcNotReady => "npc_not_ready",
            GameError::NpcAlreadyHelped => "npc_already_helped",
            GameError::NpcRanchEmpty => "npc_ranch_empty",
            GameError::NpcRanchAlreadyHelped => "npc_ranch_already_helped",
            GameError::NpcRanchNotReady => "npc_ranch_not_ready",
            GameError::NpcRanchStealLimit => "npc_ranch_steal_limit",
            GameError::NpcRanchHelpLimit => "npc_ranch_help_limit",
            GameError::NpcStealLimit => "npc_steal_limit",
            GameError::NpcHelpLimit => "npc_help_limit",
            GameError::NpcLeaveSome => "npc_leave_some",
            GameError::AlreadyReady => "already_ready",
            GameError::AlreadyWatered => "already_watered",
            GameError::AlreadyUnlocked => "already_unlocked",
            GameError::UnknownCrop => "unknown_crop",
            GameError::UnknownItem => "unknown_item",
            GameError::FishingSpotLocked { .. } => "fishing_spot_locked",
            GameError::FishingBaitEmpty => "fishing_bait_empty",
            GameError::FishingSessionActive => "fishing_session_active",
            GameError::FishingSessionExpired => "fishing_session_expired",
            GameError::FishingNoSession => "fishing_no_session",
            GameError::FishingRodLocked { .. } => "fishing_rod_locked",
            GameError::FishingRodOwned => "fishing_rod_owned",
            GameError::UnknownFish => "unknown_fish",
            GameError::FishLocked => "fish_locked",
            GameError::BadFishingSkill => "bad_fishing_skill",
            GameError::FishingSkillMax => "fishing_skill_max",
            GameError::BadSkill => "bad_skill",
            GameError::SkillMax => "skill_max",
            GameError::BadBuffItem => "bad_buff_item",
            GameError::NotEnoughSkillPoints { .. } => "not_enough_skill_points",
            GameError::NothingToSell => "nothing_to_sell",
            GameError::OnlyProduceSellable => "only_produce_sellable",
            GameError::InvalidQty => "invalid_qty",
            GameError::NothingToHarvest => "nothing_to_harvest",
            GameError::NeedHarvester => "need_harvester",
            GameError::FacilityOwned => "facility_owned",
            GameError::FacilityMax => "facility_max",
            GameError::NoDamagedWindmill => "no_damaged_windmill",
            GameError::PetExists => "pet_exists",
            GameError::NoPet => "no_pet",
            GameError::PetCooldown => "pet_cooldown",
            GameError::PetTooTired => "pet_too_tired",
            GameError::PetSick => "pet_sick",
            GameError::PetLocationLocked => "pet_location_locked",
            GameError::PetHealthy => "pet_healthy",
            GameError::InvalidPetName => "invalid_pet_name",
            GameError::PetLimit => "pet_limit",
            GameError::PetNotFound => "pet_not_found",
            GameError::PetBoarded => "pet_boarded",
            GameError::PetSupplyOwned => "pet_supply_owned",
            GameError::PetSupplyNotBroken => "pet_supply_not_broken",
            GameError::NoBoardingDebt => "no_boarding_debt",
            GameError::RanchPenEmpty => "ranch_pen_empty",
            GameError::RanchPenOccupied => "ranch_pen_occupied",
            GameError::RanchProductNotReady => "ranch_product_not_ready",
            GameError::RanchAnimalHealthy => "ranch_animal_healthy",
            GameError::RanchAnimalSick => "ranch_animal_sick",
            GameError::RanchMaxPens => "ranch_max_pens",
            GameError::RanchNoFeed => "ranch_no_feed",
            GameError::RanchNoBreedPair => "ranch_no_breed_pair",
            GameError::RanchNoEmptyPen => "ranch_no_empty_pen",
            GameError::RanchWorkshopOwned => "ranch_workshop_owned",
            GameError::RanchWorkshopMissing => "ranch_workshop_missing",
            GameError::RanchCraftBusy => "ranch_craft_busy",
            GameError::RanchCraftNotReady => "ranch_craft_not_ready",
            GameError::RanchCraftMissing => "ranch_craft_missing",
            GameError::RanchIngredientsShort => "ranch_ingredients_short",
            GameError::WorkshopMaxSlots => "workshop_max_slots",
            GameError::WorkshopSlotInvalid => "workshop_slot_invalid",
            GameError::WorkshopSlotOccupied => "workshop_slot_occupied",
            GameError::WorkshopEquipmentMissing => "workshop_equipment_missing",
            GameError::WorkshopEquipmentMismatch => "workshop_equipment_mismatch",
            GameError::NoPetMedicine => "no_pet_medicine",
            GameError::PetSkillCooldown => "pet_skill_cooldown",
            GameError::TrainingLocked => "training_locked",
            GameError::TrainingMax => "training_max",
            GameError::FacilityLocked { .. } => "facility_locked",
            GameError::InvalidContent(_) => "invalid_content",
            GameError::ContentRemovesUsedCrop(_) => "content_removes_used_crop",
            GameError::BroadcastTooLong => "broadcast_too_long",
            GameError::EmptyText => "empty_text",
            GameError::Overflow => "overflow",
        }
    }
}
