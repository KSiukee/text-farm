use serde::{Deserialize, Serialize};

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
        #[serde(rename = "cropId")]
        crop_id: String,
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
    BreedRanch,
    BuildWorkshop,
    ExpandWorkshop,
    InstallEquipment {
        slot: u8,
        kind: crate::workshop::EquipmentKind,
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
    ClaimSignIn,
    ClaimDaily {
        id: u8,
    },
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
}
