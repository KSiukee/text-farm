use serde::{Deserialize, Serialize};

use crate::room::RoomLayout;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FacilityKind {
    Sprinkler,
    Harvester,
    Windmill,
    Greenhouse,
    TrainingCenter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    Plant {
        plot: u8,
        #[serde(rename = "cropId")]
        crop_id: String,
    },
    PlantRareSeed {
        plot: u8,
        #[serde(rename = "cropId")]
        crop_id: String,
    },
    Water {
        plot: u8,
    },
    Harvest {
        plot: u8,
    },
    HarvestAll,
    Clear {
        plot: u8,
    },
    ClearPest {
        plot: u8,
    },
    UnlockPlot {
        plot: u8,
    },
    UpgradePlot {
        plot: u8,
    },
    MitigateDisaster {
        plot: u8,
    },
    SetRegion {
        region: crate::model::Region,
    },
    CraftSpecimen {
        #[serde(rename = "cropId")]
        crop_id: String,
    },
    ClaimSpecimenMilestone {
        unique: u32,
    },
    RepairWindmills,
    AdoptPet {
        name: String,
        breed: crate::pet::PetBreed,
        #[serde(default)]
        board: bool,
    },
    FeedPet {
        #[serde(rename = "petId")]
        pet_id: String,
        /// 喂一条鱼（`fish:<id>` 的 id）；只有猫吃生鱼。
        #[serde(rename = "fishId")]
        fish_id: String,
    },
    /// 买一份宠物食品（猫粮/狗粮/狗骨头/猫罐头）。
    BuyPetFood {
        kind: crate::pet::PetFoodKind,
    },    /// 喂宠物吃买来的食品。
    FeedPetFood {
        #[serde(rename = "petId")]
        pet_id: String,
        kind: crate::pet::PetFoodKind,
    },
    PetCompanion {
        #[serde(rename = "petId")]
        pet_id: String,
    },
    PlayWithPet {
        #[serde(rename = "petId")]
        pet_id: String,
    },
    HealPet {
        #[serde(rename = "petId")]
        pet_id: String,
    },
    DigWithPet {
        #[serde(rename = "petId")]
        pet_id: String,
    },
    SetPetLocation {
        #[serde(rename = "petId")]
        pet_id: String,
        location: crate::pet::PetLocation,
    },
    RenamePet {
        #[serde(rename = "petId")]
        pet_id: String,
        name: String,
    },
    SetPetBoarded {
        #[serde(rename = "petId")]
        pet_id: String,
        boarded: bool,
    },
    ReleasePet {
        #[serde(rename = "petId")]
        pet_id: String,
    },
    SettleBoardingDebt,
    BuyRanchAnimal {
        pen: u8,
        kind: crate::ranch::RanchAnimalKind,
    },
    FeedRanchAnimal {
        pen: u8,
    },
    /// 清理畜栏（每天一次，卫生 +50）。
    CleanRanchPen {
        pen: u8,
    },
    CollectRanchProduct {
        pen: u8,
    },
    HealRanchAnimal {
        pen: u8,
    },
    ExpandRanch,
    FeedRanchPremium {
        pen: u8,
    },
    BuyRanchFeed,
    /// 手动选两只动物繁殖；预览与真繁殖用同一个序号，结果一致。
    BreedRanchPair {
        a: u8,
        b: u8,
    },
    /// 给牧场动物改名（1–12 个字）。
    RenameRanchAnimal {
        pen: u8,
        name: String,
    },
    /// 接一笔牧场订单。
    AcceptRanchOrder {
        id: String,
    },
    /// 交付一笔牧场订单。
    DeliverRanchOrder {
        id: String,
    },
    /// 报名每周牧场评比（每周一只）。
    EnterRanchContest {
        pen: u8,
    },
    /// 建鱼塘（8 级 / 8,000）。
    BuildPond,
    /// 把背包里的鱼放进鱼塘（每次最多 5 条）。
    StockPond {
        #[serde(rename = "fishId")]
        fish_id: String,
        qty: u32,
    },
    /// 给鱼塘投喂一份作物。
    FeedPond,
    /// 清理鱼塘。
    CleanPond,
    /// 装增氧机。
    InstallPondAerator,
    /// 种水草。
    PlantPondWeeds,
    /// 捕捞一种鱼（达到商品规格才可捕）。
    HarvestPond {
        #[serde(rename = "fishId")]
        fish_id: String,
    },
    /// 开关留种（每天自动来一窝鱼苗）。
    TogglePondFry,
    /// 交付今天的鱼塘订单。
    DeliverPondOrder,
    BuildWorkshop,
    ExpandWorkshop,
    InstallEquipment {
        slot: u8,
        kind: crate::workshop::EquipmentKind,
    },
    UpgradeEquipment {
        slot: u8,
    },
    StartRanchCraft {
        recipe: crate::ranch::RanchRecipe,
    },
    StartCropCraft {
        #[serde(rename = "cropId")]
        crop_id: String,
    },
    StartEquipmentCraft {
        recipe: crate::workshop::EquipmentRecipe,
    },
    /// 制造屋自动做：自动拆半成品、自动收取，直到做完一份。
    AutoCraftWorkshop {
        recipe: crate::workshop::EquipmentRecipe,
    },
    /// 取消当前制造屋自动做。
    CancelWorkshopAuto,
    StartBaitCraft {
        recipe: crate::fishing::BaitRecipe,
    },
    StartFishCraft {
        recipe: crate::workshop::FishRecipe,
    },
    CollectCraft {
        slot: u8,
    },
    BuyBait {
        qty: u32,
        #[serde(default)]
        quality: crate::fishing::BaitQuality,
        #[serde(default)]
        bait_type: crate::fishing::BaitType,
    },
    SetBaitType {
        #[serde(rename = "baitType")]
        bait_type: crate::fishing::BaitType,
    },
    BuyRod {
        rod: crate::fishing::RodKind,
    },
    SwitchRod {
        rod: crate::fishing::RodKind,
    },
    CastLine {
        spot: crate::fishing::FishingSpot,
        #[serde(default)]
        quality: Option<crate::fishing::BaitQuality>,
    },
    ToggleFishLock {
        #[serde(rename = "fishId")]
        fish_id: String,
    },
    SellAllFish,
    MaintainRod {
        rod: crate::fishing::RodKind,
    },
    ResolveFishing {
        success: bool,
    },
    AcceptFishingCommission {
        id: String,
    },
    ClaimFishingCommission {
        id: String,
    },
    RefreshFishingCommissions,
    ClaimFishingTournament {
        day: String,
    },
    SetAutoFishing {
        spot: crate::fishing::FishingSpot,
        enabled: bool,
        #[serde(default)]
        rod: Option<crate::fishing::RodKind>,
        #[serde(default, rename = "baitQuality")]
        bait_quality: Option<crate::fishing::BaitQuality>,
        #[serde(default, rename = "baitType")]
        bait_type: Option<crate::fishing::BaitType>,
    },
    UpgradeFishingSkill {
        skill: String,
    },
    UpgradeSkill {
        discipline: crate::skills::DisciplineKind,
        skill: String,
    },
    UseBuffItem {
        key: String,
    },
    BuyPetSupply {
        kind: crate::pet::PetSupplyKind,
    },
    RepairPetSupply {
        kind: crate::pet::PetSupplyKind,
    },
    UsePetMedicine {
        #[serde(rename = "petId")]
        pet_id: String,
    },
    UsePetSkill {
        #[serde(rename = "petId")]
        pet_id: String,
    },
    TrainPet {
        #[serde(rename = "petId")]
        pet_id: String,
        kind: crate::pet::PetTrainingKind,
    },
    ChoosePetTalent {
        #[serde(rename = "petId")]
        pet_id: String,
        talent: crate::pet::PetTalent,
    },
    BuySeed {
        #[serde(rename = "cropId")]
        crop_id: String,
        qty: u32,
    },
    Sell {
        key: String,
        qty: u32,
    },
    BuyFacility {
        kind: FacilityKind,
    },
    HelpNpc {
        #[serde(rename = "npcId")]
        npc_id: String,
        plot: u8,
    },
    StealNpc {
        #[serde(rename = "npcId")]
        npc_id: String,
        plot: u8,
    },
    HelpNpcRanch {
        #[serde(rename = "npcId")]
        npc_id: String,
        index: u8,
    },
    StealNpcRanch {
        #[serde(rename = "npcId")]
        npc_id: String,
        index: u8,
    },
    NpcGreeting {
        #[serde(rename = "npcId")]
        npc_id: String,
    },
    /// 和村民聊几句：每天一次好感，好感够了触发一段小故事。
    ChatWithNpc {
        #[serde(rename = "npcId")]
        npc_id: String,
    },
    /// 帮村民完成本周的小请求。
    HelpNpcRequest {
        #[serde(rename = "npcId")]
        npc_id: String,
    },
    /// 请邻居搭把手，每天一次。
    AskNpcHelp {
        #[serde(rename = "npcId")]
        npc_id: String,
    },
    /// 给已触发的故事做二选一。
    NpcStoryChoice {
        #[serde(rename = "npcId")]
        npc_id: String,
        #[serde(rename = "eventId")]
        event_id: String,
        #[serde(rename = "choiceId")]
        choice_id: String,
    },
    /// 生日当天送一份小礼物。
    GiveNpcBirthdayGift {
        #[serde(rename = "npcId")]
        npc_id: String,
    },
    /// 平时送一件 NPC 喜欢的小东西，每天一次。
    GiveNpcGift {
        #[serde(rename = "npcId")]
        npc_id: String,
    },
    /// 领取第一周手册目标奖励。
    ClaimHandbookGoal {
        id: String,
    },
    /// 参加今天的节日，领一份小回礼（每年一次）。
    CelebrateFestival,
    /// 选一段节日小剧情的选项。
    ChooseFestivalStory {
        #[serde(rename = "choiceId")]
        choice_id: String,
    },
    /// 和集市上的摊主/村人聊两句。
    TalkFestivalMarket {
        id: String,
    },
    /// 给节日集市交一份备货。
    SupplyFestivalMarket {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    /// 在节日集市买一件限定品。
    BuyFestivalOffer {
        #[serde(rename = "offerId")]
        offer_id: String,
    },
    /// 节日集市寄卖一件库存（+30%，每天 3 次）。
    SellFestivalGoods {
        key: String,
    },
    /// 领节日集市贡献回礼。
    ClaimFestivalGift {
        threshold: u32,
    },
    ClaimSignIn,
    UpdateProfile {
        #[serde(rename = "ownerName")]
        owner_name: String,
    },
    ClaimDaily {
        id: u8,
    },
    AcceptFarmOrder {
        id: String,
    },
    DeliverFarmOrder {
        id: String,
    },
    /// 把一笔加工订单直接送进对应的制造屋设备。
    StartOrderCraft {
        id: String,
    },
    /// 自动排程：缺什么半成品就先逐级做，取货后继续。
    AutoCraftOrder {
        id: String,
    },
    BuyMysteryOffer {
        id: String,
    },
    BuyFurniture {
        def: String,
    },
    BuyDecor {
        id: String,
    },
    UseFarmCard,
    ClaimLevelReward {
        level: u32,
    },
    UpdateSettings {
        #[serde(rename = "expRate")]
        exp_rate: u32,
    },
    ShowGuide,
    DismissGuide,
    Broadcast {
        text: String,
    },
    SaveRoomLayout {
        room: String,
        layout: RoomLayout,
    },
    MoveFishToAquarium {
        room: String,
        #[serde(rename = "furnitureId")]
        furniture_id: String,
        #[serde(rename = "fishId")]
        fish_id: String,
    },
    RemoveFishFromAquarium {
        room: String,
        #[serde(rename = "furnitureId")]
        furniture_id: String,
        #[serde(rename = "fishId")]
        fish_id: String,
    },
    UnlockExtraRoom,
}
