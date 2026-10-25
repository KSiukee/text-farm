use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PET_TOUCH_COOLDOWN_MS: i64 = 60 * 60 * 1000;
pub const PET_PLAY_COOLDOWN_MS: i64 = 30 * 60 * 1000;
pub const PET_DIG_COOLDOWN_MS: i64 = 8 * 60 * 60 * 1000;
pub const PET_MAX_LEVEL: u32 = 30;
pub const PET_ACTIVE_LIMIT: usize = 3;
pub const PET_BOARD_FEE_PER_MONTH: u64 = 3_000;
pub const PET_SKILL_MAX: u32 = 5;
pub const PET_DIG_SKILL_STEP_MS: i64 = 30 * 60 * 1000;

pub const PET_MILESTONE_FIRST_RANCH: &str = "first_ranch";
pub const PET_MILESTONE_FIRST_FISHING: &str = "first_fishing";
pub const PET_MILESTONE_FIRST_POND: &str = "first_pond";
pub const PET_MILESTONE_JOB_MASTER: &str = "job_master";
pub const PET_MILESTONE_OLD_FRIEND: &str = "old_friend";

pub fn milestone_label(key: &str) -> &'static str {
    match key {
        PET_MILESTONE_FIRST_RANCH => "第一次看牧场",
        PET_MILESTONE_FIRST_FISHING => "第一次去钓鱼地",
        PET_MILESTONE_FIRST_POND => "第一次看鱼塘",
        PET_MILESTONE_JOB_MASTER => "岗位满级",
        PET_MILESTONE_OLD_FRIEND => "相伴 100 天",
        _ => "小小的纪念",
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PetCare {
    pub dog_bed: bool,
    pub cat_bed: bool,
    pub dog_bed_broken: bool,
    pub cat_bed_broken: bool,
    pub bowls: u8,
    pub dog_toy: bool,
    pub cat_toy: bool,
    pub dog_toy_broken: bool,
    pub cat_toy_broken: bool,
    /// 共用软垫：每天让当值宠物心情 +1。
    pub soft_mat: bool,
    /// 自动饮水器：每天让当值宠物健康 +1。
    pub water_fountain: bool,
    pub medicine: u32,
    /// 猫粮/狗粮/狗骨头/猫罐头，买来存在家里。
    pub cat_food: u32,
    pub dog_food: u32,
    pub dog_bone: u32,
    pub cat_can: u32,
}

impl PetCare {
    pub fn food_count(&self, kind: PetFoodKind) -> u32 {
        match kind {
            PetFoodKind::CatFood => self.cat_food,
            PetFoodKind::DogFood => self.dog_food,
            PetFoodKind::DogBone => self.dog_bone,
            PetFoodKind::CatCan => self.cat_can,
        }
    }

    pub fn add_food(&mut self, kind: PetFoodKind, amount: u32) {
        let slot = match kind {
            PetFoodKind::CatFood => &mut self.cat_food,
            PetFoodKind::DogFood => &mut self.dog_food,
            PetFoodKind::DogBone => &mut self.dog_bone,
            PetFoodKind::CatCan => &mut self.cat_can,
        };
        *slot = slot.saturating_add(amount);
    }

    pub fn take_food(&mut self, kind: PetFoodKind) -> bool {
        let slot = match kind {
            PetFoodKind::CatFood => &mut self.cat_food,
            PetFoodKind::DogFood => &mut self.dog_food,
            PetFoodKind::DogBone => &mut self.dog_bone,
            PetFoodKind::CatCan => &mut self.cat_can,
        };
        if *slot == 0 {
            return false;
        }
        *slot -= 1;
        true
    }
}

/// 宠物食品：猫粮/狗粮是主食，狗骨头/猫罐头是零食。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PetFoodKind {
    CatFood,
    DogFood,
    DogBone,
    CatCan,
}

impl PetFoodKind {
    pub const ALL: [PetFoodKind; 4] = [
        PetFoodKind::CatFood,
        PetFoodKind::DogFood,
        PetFoodKind::DogBone,
        PetFoodKind::CatCan,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PetFoodKind::CatFood => "猫粮",
            PetFoodKind::DogFood => "狗粮",
            PetFoodKind::DogBone => "狗骨头",
            PetFoodKind::CatCan => "猫罐头",
        }
    }

    pub fn cost(self) -> u64 {
        match self {
            PetFoodKind::CatFood => 30,
            PetFoodKind::DogFood => 30,
            PetFoodKind::DogBone => 15,
            PetFoodKind::CatCan => 20,
        }
    }

    pub fn species(self) -> PetSpecies {
        match self {
            PetFoodKind::CatFood | PetFoodKind::CatCan => PetSpecies::Cat,
            PetFoodKind::DogFood | PetFoodKind::DogBone => PetSpecies::Dog,
        }
    }

    pub fn is_staple(self) -> bool {
        matches!(self, PetFoodKind::CatFood | PetFoodKind::DogFood)
    }

    pub fn snack_gain(self) -> (u32, u32, u32) {
        match self {
            PetFoodKind::DogBone => (6, 14, 3),
            PetFoodKind::CatCan => (8, 16, 3),
            _ => (0, 0, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PetTraining {
    pub guard: u32,
    pub dig: u32,
    pub mischief: u32,
    pub companion: u32,
}

impl PetTraining {
    pub fn spent(self) -> u32 {
        self.guard + self.dig + self.mischief + self.companion
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PetTrainingKind {
    Guard,
    Dig,
    Mischief,
    Companion,
}

impl PetTrainingKind {
    pub fn label(self) -> &'static str {
        match self {
            PetTrainingKind::Guard => "守护",
            PetTrainingKind::Dig => "挖掘嗅探",
            PetTrainingKind::Mischief => "闯祸拆家",
            PetTrainingKind::Companion => "陪伴",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PetSupplyKind {
    DogBed,
    CatBed,
    Bowl,
    DogToy,
    CatToy,
    SoftMat,
    WaterFountain,
    Medicine,
}

impl PetSupplyKind {
    pub fn label(self) -> &'static str {
        match self {
            PetSupplyKind::DogBed => "狗窝",
            PetSupplyKind::CatBed => "猫窝",
            PetSupplyKind::Bowl => "食盆",
            PetSupplyKind::DogToy => "狗狗玩具",
            PetSupplyKind::CatToy => "猫猫玩具",
            PetSupplyKind::SoftMat => "共用软垫",
            PetSupplyKind::WaterFountain => "自动饮水器",
            PetSupplyKind::Medicine => "药品",
        }
    }

    pub fn cost(self) -> u64 {
        match self {
            PetSupplyKind::DogBed => 3_000,
            PetSupplyKind::CatBed => 2_200,
            PetSupplyKind::Bowl => 600,
            PetSupplyKind::DogToy => 800,
            PetSupplyKind::CatToy => 600,
            PetSupplyKind::SoftMat => 1_200,
            PetSupplyKind::WaterFountain => 1_800,
            PetSupplyKind::Medicine => 300,
        }
    }

    pub fn repair_cost(self) -> u64 {
        match self {
            PetSupplyKind::DogBed => 1_200,
            PetSupplyKind::CatBed => 900,
            PetSupplyKind::Bowl => 0,
            PetSupplyKind::DogToy => 320,
            PetSupplyKind::CatToy => 240,
            PetSupplyKind::SoftMat | PetSupplyKind::WaterFountain => 0,
            PetSupplyKind::Medicine => 0,
        }
    }
}

/// 宠物天赋：狗、猫各三层，每层二选一；按宠物等级解锁，可随时改选。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PetTalent {
    // 狗
    Gatekeeper,
    Fetch,
    NightWatch,
    Herding,
    IronWall,
    Leader,
    OldPartner,
    // 猫
    SharpClaw,
    Clingy,
    Sly,
    FishingEye,
    TreasureNose,
    KeenEye,
    Moonlight,
}

impl PetTalent {
    pub const ALL: [PetTalent; 14] = [
        PetTalent::Gatekeeper,
        PetTalent::Fetch,
        PetTalent::NightWatch,
        PetTalent::Herding,
        PetTalent::IronWall,
        PetTalent::Leader,
        PetTalent::OldPartner,
        PetTalent::SharpClaw,
        PetTalent::Clingy,
        PetTalent::Sly,
        PetTalent::FishingEye,
        PetTalent::TreasureNose,
        PetTalent::KeenEye,
        PetTalent::Moonlight,
    ];
    /// 四层的解锁等级。
    pub const TIER_LEVELS: [u32; 4] = [5, 12, 20, 26];

    pub fn species(self) -> PetSpecies {
        match self {
            PetTalent::Gatekeeper
            | PetTalent::Fetch
            | PetTalent::NightWatch
            | PetTalent::Herding
            | PetTalent::IronWall
            | PetTalent::Leader
            | PetTalent::OldPartner => PetSpecies::Dog,
            PetTalent::SharpClaw
            | PetTalent::Clingy
            | PetTalent::Sly
            | PetTalent::FishingEye
            | PetTalent::TreasureNose
            | PetTalent::KeenEye
            | PetTalent::Moonlight => PetSpecies::Cat,
        }
    }

    pub fn tier(self) -> u8 {
        match self {
            PetTalent::Gatekeeper
            | PetTalent::Fetch
            | PetTalent::SharpClaw
            | PetTalent::Clingy => 1,
            PetTalent::NightWatch
            | PetTalent::Herding
            | PetTalent::Sly
            | PetTalent::FishingEye => 2,
            PetTalent::IronWall
            | PetTalent::Leader
            | PetTalent::TreasureNose
            | PetTalent::KeenEye => 3,
            PetTalent::OldPartner | PetTalent::Moonlight => 4,
        }
    }

    pub fn level_need(self) -> u32 {
        Self::TIER_LEVELS[(self.tier() - 1) as usize]
    }

    pub fn label(self) -> &'static str {
        match self {
            PetTalent::Gatekeeper => "守门",
            PetTalent::Fetch => "寻回",
            PetTalent::NightWatch => "守夜",
            PetTalent::Herding => "牧群",
            PetTalent::IronWall => "铁壁",
            PetTalent::Leader => "领头",
            PetTalent::OldPartner => "老伙计",
            PetTalent::SharpClaw => "利爪",
            PetTalent::Clingy => "黏人",
            PetTalent::Sly => "潜行",
            PetTalent::FishingEye => "鱼鹰",
            PetTalent::TreasureNose => "寻宝",
            PetTalent::KeenEye => "慧眼",
            PetTalent::Moonlight => "月影",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            PetTalent::Gatekeeper => "守卫吓跑概率 +3%",
            PetTalent::Fetch => "挖种子冷却 -30 分钟",
            PetTalent::NightWatch => "守卫吓跑概率 +5%",
            PetTalent::Herding => "牧场岗位产出概率 +8%",
            PetTalent::IronWall => "守卫失败时再多减 1 点损失",
            PetTalent::Leader => "牧场/钓鱼地岗位经验 +50%",
            PetTalent::OldPartner => "所有岗位经验 +75%",
            PetTalent::SharpClaw => "挖种子双倍概率 +5%",
            PetTalent::Clingy => "摸摸心情 +15%",
            PetTalent::Sly => "挖种子冷却 -45 分钟",
            PetTalent::FishingEye => "钓鱼地叼鱼概率 +10%",
            PetTalent::TreasureNose => "挖种子双倍概率再 +10%",
            PetTalent::KeenEye => "钓鱼地可叼单价 ≤120 的鱼",
            PetTalent::Moonlight => "鱼塘/钓鱼地岗位成功率 +10%",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct PetTalents {
    pub tier1: Option<PetTalent>,
    pub tier2: Option<PetTalent>,
    pub tier3: Option<PetTalent>,
    pub tier4: Option<PetTalent>,
}

impl PetTalents {
    pub fn get(self, tier: u8) -> Option<PetTalent> {
        match tier {
            1 => self.tier1,
            2 => self.tier2,
            3 => self.tier3,
            4 => self.tier4,
            _ => None,
        }
    }

    pub fn has(self, talent: PetTalent) -> bool {
        self.get(talent.tier()) == Some(talent)
    }

    /// 记下这一层的选择；返回是否真的变了。
    pub fn pick(&mut self, talent: PetTalent) -> bool {
        let slot = match talent.tier() {
            1 => &mut self.tier1,
            2 => &mut self.tier2,
            3 => &mut self.tier3,
            _ => &mut self.tier4,
        };
        if *slot == Some(talent) {
            return false;
        }
        *slot = Some(talent);
        true
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PetBreed {
    #[default]
    Corgi,
    GermanShepherd,
    GoldenRetriever,
    BorderCollie,
    Shiba,
    RuralDog,
    Bichon,
    DomesticCat,
    Ragdoll,
    GoldenShaded,
    SilverShaded,
    BlueCat,
    CoffeeCat,
}

impl PetBreed {
    pub fn label(self) -> &'static str {
        match self {
            PetBreed::Corgi => "柯基",
            PetBreed::GermanShepherd => "德国牧羊犬",
            PetBreed::GoldenRetriever => "金毛",
            PetBreed::BorderCollie => "边牧",
            PetBreed::Shiba => "柴犬",
            PetBreed::RuralDog => "田园犬",
            PetBreed::Bichon => "比熊",
            PetBreed::DomesticCat => "田园猫",
            PetBreed::Ragdoll => "布偶猫",
            PetBreed::GoldenShaded => "金渐层",
            PetBreed::SilverShaded => "银渐层",
            PetBreed::BlueCat => "蓝猫",
            PetBreed::CoffeeCat => "咖啡猫",
        }
    }

    pub fn species(self) -> PetSpecies {
        match self {
            PetBreed::DomesticCat
            | PetBreed::Ragdoll
            | PetBreed::GoldenShaded
            | PetBreed::SilverShaded
            | PetBreed::BlueCat
            | PetBreed::CoffeeCat => PetSpecies::Cat,
            _ => PetSpecies::Dog,
        }
    }

    pub fn image(self) -> &'static str {
        match self {
            PetBreed::Corgi => "/pets/dog.png",
            PetBreed::GermanShepherd => "/pets/german_shepherd.png",
            PetBreed::GoldenRetriever => "/pets/golden_retriever.png",
            PetBreed::BorderCollie => "/pets/border_collie.png",
            PetBreed::Shiba => "/pets/shiba.png",
            PetBreed::RuralDog => "/pets/rural_dog.png",
            PetBreed::Bichon => "/pets/bichon.png",
            PetBreed::DomesticCat => "/pets/cat.png",
            PetBreed::Ragdoll => "/pets/ragdoll.png",
            PetBreed::GoldenShaded => "/pets/golden_shaded.png",
            PetBreed::SilverShaded => "/pets/silver_shaded.png",
            PetBreed::BlueCat => "/pets/blue_cat.png",
            PetBreed::CoffeeCat => "/pets/coffee_cat.png",
        }
    }

    pub fn default_for(species: PetSpecies) -> Self {
        match species {
            PetSpecies::Dog => PetBreed::Corgi,
            PetSpecies::Cat => PetBreed::DomesticCat,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Temperament {
    Lively,
    Affectionate,
    Independent,
    Gluttonous,
    Alert,
    Calm,
    Gentle,
    Stubborn,
}

impl Temperament {
    pub fn label(self) -> &'static str {
        match self {
            Temperament::Lively => "活泼",
            Temperament::Affectionate => "黏人",
            Temperament::Independent => "独立",
            Temperament::Gluttonous => "贪吃",
            Temperament::Alert => "警觉",
            Temperament::Calm => "安静",
            Temperament::Gentle => "温顺",
            Temperament::Stubborn => "倔强",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BreedProfile {
    pub temperament: Temperament,
    pub feed_pct: u32,
    pub touch_pct: u32,
    pub play_pct: u32,
    pub guard_bonus: u32,
    pub dig_bonus: u32,
    pub note: &'static str,
}

impl PetBreed {
    pub fn profile(self) -> BreedProfile {
        match self {
            PetBreed::Corgi => BreedProfile {
                temperament: Temperament::Lively,
                feed_pct: 100,
                touch_pct: 100,
                play_pct: 110,
                guard_bonus: 4,
                dig_bonus: 4,
                note: "短腿跑得飞快，爱把玩具叼到你脚边。",
            },
            PetBreed::GermanShepherd => BreedProfile {
                temperament: Temperament::Alert,
                feed_pct: 105,
                touch_pct: 95,
                play_pct: 100,
                guard_bonus: 10,
                dig_bonus: 2,
                note: "耳朵一直竖着，农场有动静它先知道。",
            },
            PetBreed::GoldenRetriever => BreedProfile {
                temperament: Temperament::Affectionate,
                feed_pct: 105,
                touch_pct: 115,
                play_pct: 105,
                guard_bonus: 5,
                dig_bonus: 2,
                note: "见谁都摇尾巴，最喜欢把头搁在你腿上。",
            },
            PetBreed::BorderCollie => BreedProfile {
                temperament: Temperament::Lively,
                feed_pct: 100,
                touch_pct: 100,
                play_pct: 120,
                guard_bonus: 6,
                dig_bonus: 6,
                note: "聪明又闲不住，总能自己找到事做。",
            },
            PetBreed::Shiba => BreedProfile {
                temperament: Temperament::Stubborn,
                feed_pct: 95,
                touch_pct: 85,
                play_pct: 100,
                guard_bonus: 4,
                dig_bonus: 5,
                note: "有自己的主意，心情好才肯给你摸。",
            },
            PetBreed::RuralDog => BreedProfile {
                temperament: Temperament::Alert,
                feed_pct: 110,
                touch_pct: 100,
                play_pct: 95,
                guard_bonus: 7,
                dig_bonus: 7,
                note: "皮实又警觉，田边有风吹草动就去看。",
            },
            PetBreed::Bichon => BreedProfile {
                temperament: Temperament::Affectionate,
                feed_pct: 105,
                touch_pct: 120,
                play_pct: 110,
                guard_bonus: 2,
                dig_bonus: 2,
                note: "像一团会走路的棉花，特别粘人。",
            },
            PetBreed::DomesticCat => BreedProfile {
                temperament: Temperament::Independent,
                feed_pct: 100,
                touch_pct: 90,
                play_pct: 105,
                guard_bonus: 2,
                dig_bonus: 7,
                note: "来去自由，但饭点一定准时出现。",
            },
            PetBreed::Ragdoll => BreedProfile {
                temperament: Temperament::Gentle,
                feed_pct: 105,
                touch_pct: 120,
                play_pct: 85,
                guard_bonus: 1,
                dig_bonus: 3,
                note: "抱起来软绵绵的，不爱闹。",
            },
            PetBreed::GoldenShaded => BreedProfile {
                temperament: Temperament::Gluttonous,
                feed_pct: 115,
                touch_pct: 95,
                play_pct: 90,
                guard_bonus: 2,
                dig_bonus: 4,
                note: "对吃的特别上心，碗一响就跑来。",
            },
            PetBreed::SilverShaded => BreedProfile {
                temperament: Temperament::Calm,
                feed_pct: 95,
                touch_pct: 80,
                play_pct: 85,
                guard_bonus: 2,
                dig_bonus: 5,
                note: "安静地蹲在高处，偶尔才下来巡视。",
            },
            PetBreed::BlueCat => BreedProfile {
                temperament: Temperament::Calm,
                feed_pct: 100,
                touch_pct: 85,
                play_pct: 90,
                guard_bonus: 1,
                dig_bonus: 6,
                note: "有点敏感，熟悉之后会悄悄跟着你。",
            },
            PetBreed::CoffeeCat => BreedProfile {
                temperament: Temperament::Lively,
                feed_pct: 110,
                touch_pct: 95,
                play_pct: 115,
                guard_bonus: 2,
                dig_bonus: 6,
                note: "精力旺盛，喜欢追着落叶跑。",
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BreedSignature {
    pub name: &'static str,
    pub description: &'static str,
    pub touch_pct: i32,
    pub play_pct: i32,
    pub guard_bonus: i32,
    pub dig_bonus: i32,
    pub mischief_delta: i32,
    pub mood_floor: u32,
    pub raid_loss_reduction: u32,
    pub play_energy_save: u32,
}

impl PetBreed {
    pub fn signature(self) -> BreedSignature {
        match self {
            PetBreed::Corgi => BreedSignature {
                name: "绕圈牧牛",
                description: "祖传的牧牛本能，守卫时能少损失一点作物。",
                touch_pct: 0,
                play_pct: 0,
                guard_bonus: 1,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 1,
                play_energy_save: 0,
            },
            PetBreed::GermanShepherd => BreedSignature {
                name: "警戒吠叫",
                description: "耳朵一动就开始示警，偷菜贼更难得手。",
                touch_pct: 0,
                play_pct: 0,
                guard_bonus: 3,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::GoldenRetriever => BreedSignature {
                name: "暖心陪伴",
                description: "被它靠一会儿，心情会明显好起来。",
                touch_pct: 10,
                play_pct: 0,
                guard_bonus: 0,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::BorderCollie => BreedSignature {
                name: "接飞盘",
                description: "跑跳是它的拿手戏，玩耍更省力也更开心。",
                touch_pct: 0,
                play_pct: 10,
                guard_bonus: 0,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 5,
            },
            PetBreed::Shiba => BreedSignature {
                name: "我行我素",
                description: "不太爱被摸，但心情自然衰减不会低于 35。",
                touch_pct: -5,
                play_pct: 0,
                guard_bonus: 0,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 35,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::RuralDog => BreedSignature {
                name: "看家护院",
                description: "土生土长的看家好手，守卫损失更小。",
                touch_pct: 0,
                play_pct: 0,
                guard_bonus: 2,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 1,
                play_energy_save: 0,
            },
            PetBreed::Bichon => BreedSignature {
                name: "撒娇",
                description: "往你脚边一躺，摸摸的收益特别高。",
                touch_pct: 10,
                play_pct: 0,
                guard_bonus: 0,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::DomesticCat => BreedSignature {
                name: "捕鼠",
                description: "对土里的动静特别敏感，挖种更拿手。",
                touch_pct: 0,
                play_pct: 0,
                guard_bonus: 0,
                dig_bonus: 2,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::Ragdoll => BreedSignature {
                name: "软绵绵",
                description: "被抱起来也不挣扎，心情自然衰减不会低于 40。",
                touch_pct: 0,
                play_pct: -5,
                guard_bonus: 0,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 40,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::GoldenShaded => BreedSignature {
                name: "招财",
                description: "对亮晶晶的东西敏感，挖种时更容易有收获。",
                touch_pct: 0,
                play_pct: 0,
                guard_bonus: 0,
                dig_bonus: 2,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::SilverShaded => BreedSignature {
                name: "夜巡",
                description: "夜里也保持警觉，但不太爱被长时间摸。",
                touch_pct: -5,
                play_pct: 0,
                guard_bonus: 2,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::BlueCat => BreedSignature {
                name: "静悄悄",
                description: "动作轻，拆家概率更低，心情自然衰减不会低于 30。",
                touch_pct: 0,
                play_pct: 0,
                guard_bonus: 0,
                dig_bonus: 0,
                mischief_delta: -2,
                mood_floor: 30,
                raid_loss_reduction: 0,
                play_energy_save: 0,
            },
            PetBreed::CoffeeCat => BreedSignature {
                name: "精力旺盛",
                description: "像喝了咖啡一样，玩耍更省力、更开心。",
                touch_pct: 0,
                play_pct: 8,
                guard_bonus: 0,
                dig_bonus: 0,
                mischief_delta: 0,
                mood_floor: 0,
                raid_loss_reduction: 0,
                play_energy_save: 5,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PetSpecies {
    #[default]
    Dog,
    Cat,
}

impl PetSpecies {
    pub fn label(self) -> &'static str {
        match self {
            PetSpecies::Dog => "狗狗",
            PetSpecies::Cat => "猫猫",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            PetSpecies::Dog => "🐶",
            PetSpecies::Cat => "🐱",
        }
    }

    pub fn image(self) -> &'static str {
        match self {
            PetSpecies::Dog => "/pets/dog.png",
            PetSpecies::Cat => "/pets/cat.png",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PetStage {
    Baby,
    Adult,
    Senior,
}

impl PetStage {
    pub fn label(self) -> &'static str {
        match self {
            PetStage::Baby => "幼年",
            PetStage::Adult => "成年",
            PetStage::Senior => "老年",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PetSicknessKind {
    Cold,
    Stomach,
    Fatigue,
}

impl PetSicknessKind {
    pub fn label(self) -> &'static str {
        match self {
            PetSicknessKind::Cold => "着凉",
            PetSicknessKind::Stomach => "吃坏肚子",
            PetSicknessKind::Fatigue => "累坏了",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetSickness {
    pub kind: PetSicknessKind,
    pub since: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PetLocation {
    #[default]
    Farm,
    Ranch,
    Pond,
    Fishing,
}

impl PetLocation {
    pub fn label(self) -> &'static str {
        match self {
            PetLocation::Farm => "农场",
            PetLocation::Ranch => "牧场",
            PetLocation::Pond => "鱼塘",
            PetLocation::Fishing => "钓鱼地",
        }
    }

    pub fn available(self) -> bool {
        matches!(
            self,
            PetLocation::Farm | PetLocation::Ranch | PetLocation::Pond | PetLocation::Fishing
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActiveSkillKind {
    Seed,
    Mood,
    Energy,
    Coins,
    GuardExp,
    ClearPest,
}

#[derive(Debug, Clone, Copy)]
pub struct BreedActive {
    pub name: &'static str,
    pub description: &'static str,
    pub cooldown_ms: i64,
    pub kind: ActiveSkillKind,
    pub amount: u64,
}

impl PetBreed {
    pub fn active(self) -> BreedActive {
        match self {
            PetBreed::Corgi => BreedActive {
                name: "叼回种子",
                description: "短腿跑一圈，从田边叼回一颗种子。",
                cooldown_ms: 24 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Seed,
                amount: 1,
            },
            PetBreed::GermanShepherd => BreedActive {
                name: "巡逻示警",
                description: "沿田埂走一圈，守卫经验涨得很快。",
                cooldown_ms: 24 * 60 * 60 * 1000,
                kind: ActiveSkillKind::GuardExp,
                amount: 15,
            },
            PetBreed::GoldenRetriever => BreedActive {
                name: "抱抱",
                description: "把脑袋搁在你肩上，心情立刻回暖。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Mood,
                amount: 15,
            },
            PetBreed::BorderCollie => BreedActive {
                name: "接飞盘",
                description: "冲刺接住飞盘，精力回来一些，心情也变好。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Energy,
                amount: 20,
            },
            PetBreed::Shiba => BreedActive {
                name: "装死",
                description: "往地上一躺，谁叫都不理，心情自己缓过来。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Mood,
                amount: 15,
            },
            PetBreed::RuralDog => BreedActive {
                name: "巡田",
                description: "把田边转一遍，顺便叼回一颗种子。",
                cooldown_ms: 24 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Seed,
                amount: 1,
            },
            PetBreed::Bichon => BreedActive {
                name: "打滚撒娇",
                description: "翻个肚皮，心情和亲密一起涨。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Mood,
                amount: 15,
            },
            PetBreed::DomesticCat => BreedActive {
                name: "捕鼠",
                description: "循着动静扑过去，清掉一块地里的草虫。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::ClearPest,
                amount: 1,
            },
            PetBreed::Ragdoll => BreedActive {
                name: "翻肚皮",
                description: "软绵绵地摊开，心情恢复得很快。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Mood,
                amount: 18,
            },
            PetBreed::GoldenShaded => BreedActive {
                name: "招财",
                description: "在角落里扒拉出几枚金币。",
                cooldown_ms: 24 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Coins,
                amount: 60,
            },
            PetBreed::SilverShaded => BreedActive {
                name: "夜巡",
                description: "夜里走一圈，叼回一颗种子。",
                cooldown_ms: 24 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Seed,
                amount: 1,
            },
            PetBreed::BlueCat => BreedActive {
                name: "静心",
                description: "安静地趴一会儿，把烦躁压下去。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Mood,
                amount: 15,
            },
            PetBreed::CoffeeCat => BreedActive {
                name: "跑酷",
                description: "沿着屋脊跑一圈，精力明显恢复。",
                cooldown_ms: 12 * 60 * 60 * 1000,
                kind: ActiveSkillKind::Energy,
                amount: 25,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PetTrait {
    Lively,
    Clingy,
    Independent,
    Foodie,
    Alert,
    Calm,
    Gentle,
    Stubborn,
    Curious,
    Timid,
    Proud,
    Sleepy,
}

impl PetTrait {
    pub fn label(self) -> &'static str {
        match self {
            PetTrait::Lively => "活泼",
            PetTrait::Clingy => "黏人",
            PetTrait::Independent => "独立",
            PetTrait::Foodie => "贪吃",
            PetTrait::Alert => "警觉",
            PetTrait::Calm => "安静",
            PetTrait::Gentle => "温顺",
            PetTrait::Stubborn => "倔强",
            PetTrait::Curious => "好奇",
            PetTrait::Timid => "胆小",
            PetTrait::Proud => "傲娇",
            PetTrait::Sleepy => "贪睡",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            PetTrait::Lively => "⚡",
            PetTrait::Clingy => "🫶",
            PetTrait::Independent => "🐾",
            PetTrait::Foodie => "🍖",
            PetTrait::Alert => "👂",
            PetTrait::Calm => "🌿",
            PetTrait::Gentle => "🕊️",
            PetTrait::Stubborn => "🪨",
            PetTrait::Curious => "🔍",
            PetTrait::Timid => "🫣",
            PetTrait::Proud => "😼",
            PetTrait::Sleepy => "😴",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            PetTrait::Lively => "一听到动静就冲过来，尾巴停不下来。",
            PetTrait::Clingy => "你走到哪它跟到哪，坐下就把头搁你腿上。",
            PetTrait::Independent => "自己找地方待着，但饭点一定出现。",
            PetTrait::Foodie => "碗一响就跑来，吃完还盯着你的手。",
            PetTrait::Alert => "耳朵竖着，田边有风吹草动就去看。",
            PetTrait::Calm => "安安静静趴着，很少大惊小怪。",
            PetTrait::Gentle => "动作轻轻的，被摸时会眯起眼睛。",
            PetTrait::Stubborn => "不想动的时候，怎么哄都不挪窝。",
            PetTrait::Curious => "什么都要闻一闻，连新扫帚也不放过。",
            PetTrait::Timid => "先躲远一点观察，确认安全才靠近。",
            PetTrait::Proud => "心情好才给你摸，摸久了还会扭头。",
            PetTrait::Sleepy => "一天能睡很多觉，醒了先伸个懒腰。",
        }
    }

    fn feed_mod(self) -> i32 {
        match self {
            PetTrait::Foodie => 10,
            PetTrait::Lively => 3,
            PetTrait::Sleepy => -3,
            _ => 0,
        }
    }

    fn touch_mod(self) -> i32 {
        match self {
            PetTrait::Clingy => 10,
            PetTrait::Gentle => 5,
            PetTrait::Timid => 5,
            PetTrait::Independent => -5,
            PetTrait::Proud => -8,
            PetTrait::Stubborn => -6,
            _ => 0,
        }
    }

    fn play_mod(self) -> i32 {
        match self {
            PetTrait::Lively => 10,
            PetTrait::Curious => 5,
            PetTrait::Sleepy => -8,
            PetTrait::Calm => -5,
            _ => 0,
        }
    }

    fn guard_mod(self) -> i32 {
        match self {
            PetTrait::Alert => 3,
            PetTrait::Timid => -2,
            PetTrait::Sleepy => -2,
            PetTrait::Proud => -1,
            _ => 0,
        }
    }

    fn dig_mod(self) -> i32 {
        match self {
            PetTrait::Curious => 3,
            PetTrait::Independent => 2,
            PetTrait::Sleepy => -2,
            _ => 0,
        }
    }

    fn mischief_mod(self) -> i32 {
        match self {
            PetTrait::Lively => 1,
            PetTrait::Curious => 1,
            PetTrait::Stubborn => 1,
            PetTrait::Calm => -1,
            PetTrait::Timid => -1,
            PetTrait::Sleepy => -1,
            _ => 0,
        }
    }
}

const PET_TRAITS: [PetTrait; 12] = [
    PetTrait::Lively,
    PetTrait::Clingy,
    PetTrait::Independent,
    PetTrait::Foodie,
    PetTrait::Alert,
    PetTrait::Calm,
    PetTrait::Gentle,
    PetTrait::Stubborn,
    PetTrait::Curious,
    PetTrait::Timid,
    PetTrait::Proud,
    PetTrait::Sleepy,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PetQuirk {
    Digger,
    Barker,
    Cuddler,
    Beggar,
    Napper,
    Shadow,
    ToyHoarder,
    WindowWatcher,
}

impl PetQuirk {
    pub fn label(self) -> &'static str {
        match self {
            PetQuirk::Digger => "爱刨坑",
            PetQuirk::Barker => "爱叫",
            PetQuirk::Cuddler => "爱蹭腿",
            PetQuirk::Beggar => "爱讨食",
            PetQuirk::Napper => "爱睡",
            PetQuirk::Shadow => "跟脚",
            PetQuirk::ToyHoarder => "藏玩具",
            PetQuirk::WindowWatcher => "趴窗",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            PetQuirk::Digger => "没事就在院子里刨两下，像在藏什么宝贝。",
            PetQuirk::Barker => "听见门外有动静就汪汪叫，给自己壮胆。",
            PetQuirk::Cuddler => "走着走着就贴过来，用脑袋蹭你的小腿。",
            PetQuirk::Beggar => "你吃东西时它一定坐在旁边，眼神特别真诚。",
            PetQuirk::Napper => "晒着太阳就打盹，叫它名字才抬一下眼皮。",
            PetQuirk::Shadow => "你去哪它跟到哪，连倒水都要在门口等着。",
            PetQuirk::ToyHoarder => "把玩具一个个叼到窝边，谁碰跟谁急。",
            PetQuirk::WindowWatcher => "喜欢趴在窗边看外面，一看就是半天。",
        }
    }
}

const PET_QUIRKS: [PetQuirk; 8] = [
    PetQuirk::Digger,
    PetQuirk::Barker,
    PetQuirk::Cuddler,
    PetQuirk::Beggar,
    PetQuirk::Napper,
    PetQuirk::Shadow,
    PetQuirk::ToyHoarder,
    PetQuirk::WindowWatcher,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetPersonality {
    pub primary: PetTrait,
    pub secondary: PetTrait,
    pub quirk: PetQuirk,
}

impl Default for PetPersonality {
    fn default() -> Self {
        Self {
            primary: PetTrait::Foodie,
            secondary: PetTrait::Curious,
            quirk: PetQuirk::Beggar,
        }
    }
}

impl PetPersonality {
    pub fn from_seed(mut seed: u64) -> Self {
        seed = seed.max(1);
        let primary = PET_TRAITS[(seed as usize) % PET_TRAITS.len()];
        let secondary = PET_TRAITS[((seed >> 9) as usize + 1) % PET_TRAITS.len()];
        let quirk = PET_QUIRKS[((seed >> 17) as usize) % PET_QUIRKS.len()];
        Self {
            primary,
            secondary: if secondary == primary {
                PET_TRAITS[(PET_TRAITS.iter().position(|t| *t == primary).unwrap_or(0) + 1)
                    % PET_TRAITS.len()]
            } else {
                secondary
            },
            quirk,
        }
    }

    pub fn for_pet(name: &str, breed: PetBreed, now: i64) -> Self {
        let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ (now as u64);
        hash ^= breed as u64;
        for byte in name.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        Self::from_seed(hash)
    }

    pub fn label(self) -> String {
        format!("{} · {}", self.primary.label(), self.secondary.label())
    }

    pub fn summary(self) -> String {
        format!("{} · 小癖好：{}", self.label(), self.quirk.label())
    }

    pub fn feed_pct(self) -> u32 {
        (100 + self.primary.feed_mod() + self.secondary.feed_mod()).max(50) as u32
    }

    pub fn touch_pct(self) -> u32 {
        (100 + self.primary.touch_mod() + self.secondary.touch_mod()).max(50) as u32
    }

    pub fn play_pct(self) -> u32 {
        (100 + self.primary.play_mod() + self.secondary.play_mod()).max(50) as u32
    }

    pub fn guard_bonus(self) -> u32 {
        (self.primary.guard_mod() + self.secondary.guard_mod()).max(0) as u32
    }

    pub fn dig_bonus(self) -> u32 {
        (self.primary.dig_mod() + self.secondary.dig_mod()).max(0) as u32
    }

    pub fn mischief_delta(self) -> i32 {
        self.primary.mischief_mod() + self.secondary.mischief_mod()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PetActivity {
    Boarded,
    Resting,
    Napping,
    Begging,
    Following,
    Exploring,
    Patrolling,
    Sunbathing,
    Grooming,
    Playing,
    WindowWatching,
    Moping,
    Guarding,
}

impl PetActivity {
    pub fn label(self) -> &'static str {
        match self {
            PetActivity::Boarded => "寄养中",
            PetActivity::Resting => "养病",
            PetActivity::Napping => "打盹",
            PetActivity::Begging => "讨食",
            PetActivity::Following => "跟脚",
            PetActivity::Exploring => "探险",
            PetActivity::Patrolling => "巡田",
            PetActivity::Sunbathing => "晒太阳",
            PetActivity::Grooming => "舔毛",
            PetActivity::Playing => "自娱自乐",
            PetActivity::WindowWatching => "趴窗",
            PetActivity::Moping => "发呆",
            PetActivity::Guarding => "守门",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            PetActivity::Boarded => "在宠物院子里，正等着你去接它。",
            PetActivity::Resting => "蜷在窝里，没什么精神。",
            PetActivity::Napping => "缩成一团，睡得正香。",
            PetActivity::Begging => "坐在食盆旁边，眼神很真诚。",
            PetActivity::Following => "你走到哪它跟到哪，一步不落。",
            PetActivity::Exploring => "在院子角落闻来闻去，像在找什么。",
            PetActivity::Patrolling => "沿着田埂走了一圈，耳朵一直竖着。",
            PetActivity::Sunbathing => "找了块暖和地方，摊成一张饼。",
            PetActivity::Grooming => "慢条斯理地舔毛，谁叫都不理。",
            PetActivity::Playing => "自己追着尾巴/落叶玩，玩得很投入。",
            PetActivity::WindowWatching => "趴在窗边看外面，一看就是半天。",
            PetActivity::Moping => "有点蔫，趴在你脚边发呆。",
            PetActivity::Guarding => "守在门口，听到动静就抬头。",
        }
    }
}

fn default_skill() -> u32 {
    1
}

/// 牧场岗位熟练度：只记录在牧场干活的天数经验，1–5 级。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct PetJob {
    pub level: u32,
    pub exp: u64,
}

impl Default for PetJob {
    fn default() -> Self {
        Self { level: 1, exp: 0 }
    }
}

impl PetJob {
    pub const MAX_LEVEL: u32 = 5;

    pub fn need(level: u32) -> u64 {
        20 + u64::from(level) * 20
    }

    /// 加岗位经验；返回是否升级。
    pub fn gain(&mut self, exp: u64) -> bool {
        if self.level >= Self::MAX_LEVEL {
            return false;
        }
        self.exp = self.exp.saturating_add(exp);
        let mut leveled = false;
        while self.level < Self::MAX_LEVEL && self.exp >= Self::need(self.level) {
            self.exp -= Self::need(self.level);
            self.level += 1;
            leveled = true;
        }
        if self.level >= Self::MAX_LEVEL {
            self.exp = 0;
        }
        leveled
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PetState {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub species: PetSpecies,
    #[serde(default)]
    pub breed: PetBreed,
    #[serde(default)]
    pub boarded: bool,
    #[serde(default = "default_skill")]
    pub dig_skill: u32,
    #[serde(default)]
    pub dig_exp: u64,
    #[serde(default = "default_skill")]
    pub guard_skill: u32,
    #[serde(default)]
    pub guard_exp: u64,
    #[serde(default)]
    pub training: PetTraining,
    #[serde(default)]
    pub talents: PetTalents,
    #[serde(default)]
    pub personality: PetPersonality,
    #[serde(default)]
    pub last_mischief_day: String,
    #[serde(default)]
    pub last_activity_day: String,
    #[serde(default)]
    pub last_activity: Option<PetActivity>,
    #[serde(default)]
    pub activity_streak: u32,
    pub adopted_at: i64,
    pub level: u32,
    pub exp: u64,
    pub bond: u32,
    pub satiety: u32,
    pub energy: u32,
    pub mood: u32,
    pub health: u32,
    pub sickness: Option<PetSickness>,
    pub location: PetLocation,
    pub last_tick_at: i64,
    pub last_sick_day: String,
    pub last_pet_at: i64,
    pub last_play_at: i64,
    pub last_dig_at: i64,
    #[serde(default)]
    pub last_skill_at: i64,
    #[serde(default)]
    pub job: PetJob,
    /// 钓鱼地岗位熟练度；与牧场岗位各记各的。
    #[serde(default)]
    pub fishing_job: PetJob,
    /// 鱼塘看护岗位；与牧场/钓鱼地各记各的。
    #[serde(default)]
    pub pond_job: PetJob,
    #[serde(default)]
    pub last_job_day: String,
    /// 第一次看塘/巡塘/钓鱼这类小纪念。
    #[serde(default)]
    pub milestones: BTreeSet<String>,
    #[serde(default)]
    pub last_event_day: String,
    #[serde(default)]
    pub last_event: String,
    #[serde(default)]
    pub last_companion_day: String,
    #[serde(default)]
    pub last_companion: String,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "旺财".into(),
            species: PetSpecies::Dog,
            breed: PetBreed::Corgi,
            boarded: false,
            dig_skill: 1,
            dig_exp: 0,
            guard_skill: 1,
            guard_exp: 0,
            training: PetTraining::default(),
            talents: PetTalents::default(),
            personality: PetPersonality::default(),
            last_mischief_day: String::new(),
            last_activity_day: String::new(),
            last_activity: None,
            activity_streak: 0,
            adopted_at: 0,
            level: 1,
            exp: 0,
            bond: 10,
            satiety: 80,
            energy: 80,
            mood: 80,
            health: 100,
            sickness: None,
            location: PetLocation::Farm,
            last_tick_at: 0,
            last_sick_day: String::new(),
            last_pet_at: 0,
            last_play_at: 0,
            last_dig_at: 0,
            last_skill_at: 0,
            job: PetJob::default(),
            fishing_job: PetJob::default(),
            pond_job: PetJob::default(),
            last_job_day: String::new(),
            milestones: BTreeSet::new(),
            last_event_day: String::new(),
            last_event: String::new(),
            last_companion_day: String::new(),
            last_companion: String::new(),
        }
    }
}

impl PetState {
    pub fn new(name: String, species: PetSpecies, now: i64) -> Self {
        let breed = PetBreed::default_for(species);
        Self {
            personality: PetPersonality::for_pet(&name, breed, now),
            breed,
            name,
            species,
            adopted_at: now,
            last_tick_at: now,
            last_pet_at: now - PET_TOUCH_COOLDOWN_MS,
            last_play_at: now - PET_PLAY_COOLDOWN_MS,
            last_dig_at: now - PET_DIG_COOLDOWN_MS,
            last_skill_at: now - breed.active().cooldown_ms,
            ..Self::default()
        }
    }

    pub fn new_breed(name: String, breed: PetBreed, now: i64) -> Self {
        let species = breed.species();
        let profile = breed.profile();
        let personality = PetPersonality::for_pet(&name, breed, now);
        Self {
            breed,
            personality,
            last_skill_at: now - breed.active().cooldown_ms,
            dig_skill: if profile.dig_bonus >= 6 { 2 } else { 1 },
            guard_skill: if profile.guard_bonus >= 7 { 2 } else { 1 },
            ..Self::new(name, species, now)
        }
    }

    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    pub fn exp_need(&self) -> u64 {
        50 + u64::from(self.level) * 30
    }

    pub fn skill_need(level: u32) -> u64 {
        20 + u64::from(level) * 20
    }

    pub fn training_cap(&self) -> u32 {
        (self.level / 5).min(6)
    }

    pub fn training_spent(&self) -> u32 {
        self.training.spent()
    }

    pub fn can_train(&self, kind: PetTrainingKind) -> bool {
        let track = match kind {
            PetTrainingKind::Guard => self.training.guard,
            PetTrainingKind::Dig => self.training.dig,
            PetTrainingKind::Mischief => self.training.mischief,
            PetTrainingKind::Companion => self.training.companion,
        };
        track < 5 && self.training_spent() < self.training_cap()
    }

    pub fn train(&mut self, kind: PetTrainingKind) -> bool {
        if !self.can_train(kind) {
            return false;
        }
        match kind {
            PetTrainingKind::Guard => self.training.guard += 1,
            PetTrainingKind::Dig => self.training.dig += 1,
            PetTrainingKind::Mischief => self.training.mischief += 1,
            PetTrainingKind::Companion => self.training.companion += 1,
        }
        true
    }

    pub fn mischief_chance(&self) -> u32 {
        (4i32 - self.training.mischief as i32
            + self.personality.mischief_delta()
            + self.breed.signature().mischief_delta)
            .max(0) as u32
    }

    pub fn dig_bonus(&self) -> u32 {
        self.breed.profile().dig_bonus
            + self.dig_skill.saturating_sub(1) * 3
            + self.training.dig * 3
            + self.personality.dig_bonus()
            + self.breed.signature().dig_bonus.max(0) as u32
    }

    pub fn guard_bonus(&self) -> u32 {
        self.breed.profile().guard_bonus
            + self.guard_skill.saturating_sub(1) * 2
            + self.training.guard * 3
            + self.personality.guard_bonus()
            + self.breed.signature().guard_bonus.max(0) as u32
            + self.talent_guard_bonus()
    }

    pub fn talent_guard_bonus(&self) -> u32 {
        let mut bonus = 0;
        if self.talents.has(PetTalent::Gatekeeper) {
            bonus += 3;
        }
        if self.talents.has(PetTalent::NightWatch) {
            bonus += 5;
        }
        bonus
    }

    pub fn talent_dig_double(&self) -> u32 {
        let mut bonus = 0;
        if self.talents.has(PetTalent::SharpClaw) {
            bonus += 5;
        }
        if self.talents.has(PetTalent::TreasureNose) {
            bonus += 10;
        }
        bonus
    }

    pub fn talent_dig_cut_ms(&self) -> i64 {
        let mut cut = 0;
        if self.talents.has(PetTalent::Fetch) {
            cut += 30 * 60 * 1000;
        }
        if self.talents.has(PetTalent::Sly) {
            cut += 45 * 60 * 1000;
        }
        cut
    }

    pub fn talent_loss_cut(&self) -> u32 {
        u32::from(self.talents.has(PetTalent::IronWall))
    }

    pub fn talent_job_exp(&self) -> u64 {
        if self.talents.has(PetTalent::OldPartner) {
            14
        } else if self.talents.has(PetTalent::Leader) {
            12
        } else {
            8
        }
    }

    pub fn talent_ranch_chance(&self) -> u32 {
        if self.talents.has(PetTalent::Herding) {
            8
        } else {
            0
        }
    }

    pub fn talent_fishing_chance(&self) -> u32 {
        let mut chance = 0;
        if self.talents.has(PetTalent::FishingEye) {
            chance += 10;
        }
        if self.talents.has(PetTalent::Moonlight) {
            chance += 10;
        }
        chance
    }

    pub fn talent_pond_chance(&self) -> u32 {
        if self.talents.has(PetTalent::Moonlight) {
            10
        } else {
            0
        }
    }

    pub fn talent_fish_price_cap(&self) -> u64 {
        if self.talents.has(PetTalent::KeenEye) {
            120
        } else {
            70
        }
    }

    pub fn dig_cooldown_ms(&self) -> i64 {
        (PET_DIG_COOLDOWN_MS
            - i64::from(self.dig_skill.saturating_sub(1)) * PET_DIG_SKILL_STEP_MS
            - self.talent_dig_cut_ms())
        .max(4 * 60 * 60 * 1000)
    }

    pub fn active_skill(&self) -> BreedActive {
        self.breed.active()
    }

    pub fn skill_remaining_ms(&self, now: i64) -> i64 {
        (self.active_skill().cooldown_ms - (now - self.last_skill_at)).max(0)
    }

    /// 返回是否升级。
    pub fn add_skill_exp(&mut self, digging: bool, amount: u64) -> bool {
        let (level, exp) = if digging {
            (&mut self.dig_skill, &mut self.dig_exp)
        } else {
            (&mut self.guard_skill, &mut self.guard_exp)
        };
        if *level >= PET_SKILL_MAX {
            return false;
        }
        *exp = exp.saturating_add(amount);
        let mut leveled = false;
        while *level < PET_SKILL_MAX && *exp >= Self::skill_need(*level) {
            *exp -= Self::skill_need(*level);
            *level += 1;
            leveled = true;
        }
        leveled
    }

    /// 记录一次小纪念；返回是否是第一次。
    pub fn note_milestone(&mut self, key: &str) -> bool {
        self.milestones.insert(key.to_string())
    }

    /// 每天一条随地点变化的小事件，roll 由日期和宠物名确定。
    pub fn daily_event(&self, roll: u64) -> &'static str {
        let index = (roll % 3) as usize;
        match (self.location, self.species) {
            (PetLocation::Farm, PetSpecies::Dog) => [
                "在田埂上追了一只蝴蝶，鼻尖沾着草籽",
                "把一根树枝叼回来，放在你脚边",
                "在院子里挖了个小坑，自己躺进去",
            ][index],
            (PetLocation::Farm, PetSpecies::Cat) => [
                "在窗台上晒了一下午太阳",
                "追着一只蛾子撞到了门框",
                "在草垛上睡醒，胡须上沾着草屑",
            ][index],
            (PetLocation::Ranch, PetSpecies::Dog) => [
                "绕着畜栏跑了一圈，把麻雀赶走了",
                "趴在饲料桶边守了半天",
                "在草堆里打了个滚，沾了一身干草",
            ][index],
            (PetLocation::Ranch, PetSpecies::Cat) => [
                "蹲在牛栏顶上，谁叫都不下来",
                "在干草堆里睡了一觉，只露出尾巴",
                "盯着挤奶桶看了很久，最后走开了",
            ][index],
            (PetLocation::Pond, PetSpecies::Dog) => [
                "绕着鱼塘跑了一圈，把野猫赶走了",
                "在塘边刨了个浅坑，趴进去看水",
                "对着水面上的倒影叫了两声",
            ][index],
            (PetLocation::Pond, PetSpecies::Cat) => [
                "蹲在鱼塘边看水纹，尾巴一下一下拍着地",
                "伸爪碰了碰水面，又很快收回来",
                "在塘边芦苇里蹲了半天，没让人发现",
            ][index],
            (PetLocation::Fishing, PetSpecies::Dog) => [
                "在岸边叼回一根别人落下的鱼线",
                "对着水里的鱼影叫了两声",
                "把鼻子凑近鱼护，被腥味呛得打了个喷嚏",
            ][index],
            (PetLocation::Fishing, PetSpecies::Cat) => [
                "在钓鱼佬脚边蹲了一下午，等一条小鱼",
                "盯着浮漂看，比钓鱼的人还认真",
                "在岸边石头上磨爪子，磨完就睡",
            ][index],
        }
    }

    pub fn age_days(&self, now: i64) -> u64 {        (now.saturating_sub(self.adopted_at) / 86_400_000).max(0) as u64
    }

    pub fn stage(&self, now: i64) -> PetStage {
        let days = self.age_days(now);
        if days < 2 {
            PetStage::Baby
        } else if days < 30 {
            PetStage::Adult
        } else {
            PetStage::Senior
        }
    }

    pub fn add_exp(&mut self, amount: u64) -> bool {
        if self.level >= PET_MAX_LEVEL {
            return false;
        }
        self.exp = self.exp.saturating_add(amount);
        let mut leveled = false;
        while self.level < PET_MAX_LEVEL && self.exp >= self.exp_need() {
            self.exp -= self.exp_need();
            self.level += 1;
            self.bond = (self.bond + 2).min(100);
            leveled = true;
        }
        leveled
    }

    pub fn mood_label(&self) -> &'static str {
        if self.sickness.is_some() {
            "不舒服"
        } else if self.satiety < 20 {
            "饿了"
        } else if self.energy < 20 {
            "困了"
        } else if self.mood >= 80 {
            "开心"
        } else if self.mood >= 50 {
            "平静"
        } else {
            "低落"
        }
    }

    /// 离线/在线按小时结算；不做死亡，最低保留 0 和病情。
    pub fn tick(&mut self, now: i64, today: &str, bed: bool) -> bool {
        let elapsed = now.saturating_sub(self.last_tick_at);
        if elapsed < 3_600_000 {
            return false;
        }
        let hours = (elapsed / 3_600_000).min(24 * 30) as u32;
        self.last_tick_at += i64::from(hours) * 3_600_000;
        if hours == 0 {
            return false;
        }
        self.satiety = self.satiety.saturating_sub(hours * 2);
        if self.sickness.is_some() {
            self.energy = self.energy.saturating_sub(hours);
            self.mood = self.mood.saturating_sub(hours);
            self.health = self.health.saturating_sub(hours / 4);
        } else {
            self.energy = (self.energy + hours * 2).min(100);
            if bed {
                self.energy = (self.energy + hours).min(100);
            }
            self.mood = self.mood.saturating_sub(hours / 3);
            self.mood = self.mood.max(self.breed.signature().mood_floor);
            match self.activity(now) {
                PetActivity::Napping => self.energy = (self.energy + hours).min(100),
                PetActivity::Sunbathing => self.mood = (self.mood + hours / 2).min(100),
                PetActivity::Playing => self.mood = (self.mood + hours / 3).min(100),
                _ => {}
            }
        }
        if self.satiety == 0 {
            self.health = self.health.saturating_sub(hours / 3);
        }

        if self.last_sick_day != today {
            self.last_sick_day = today.to_string();
            let weak = self.satiety < 25 || self.energy < 20;
            if weak && roll(day_seed(today) as i64, &self.name, "sick") % 100 < 18 {
                let kind = match roll(day_seed(today) as i64, &self.name, "sick-kind") % 3 {
                    0 => PetSicknessKind::Cold,
                    1 => PetSicknessKind::Stomach,
                    _ => PetSicknessKind::Fatigue,
                };
                self.sickness = Some(PetSickness { kind, since: now });
            }
        }
        true
    }

    pub fn temperament(&self) -> Temperament {
        self.breed.profile().temperament
    }

    pub fn behavior_line(&self) -> String {
        if self.boarded {
            "在宠物院子里，正等着你去接它。".to_string()
        } else if self.sickness.is_some() {
            "蜷在窝里，没什么精神。".to_string()
        } else if self.satiety < 20 {
            "围着你转，时不时看一眼食盆。".to_string()
        } else if self.energy < 20 {
            "眼皮打架，趴在地上不想动。".to_string()
        } else if self.mood < 40 {
            "有点蔫，趴在你脚边。".to_string()
        } else if self.mood >= 75 {
            self.personality.primary.line().to_string()
        } else {
            self.personality.secondary.line().to_string()
        }
    }

    pub fn quirk_line(&self) -> &'static str {
        self.personality.quirk.line()
    }

    pub fn daily_activity_bonus(&mut self, activity: PetActivity, streak: u32) -> Option<String> {
        let streak_bonus = (streak / 3).min(2);
        let (text, extra) = match activity {
            PetActivity::Patrolling | PetActivity::Guarding => {
                self.add_skill_exp(false, 2 + u64::from(streak_bonus));
                ("巡田时把守卫经验练上去了一点。", None)
            }
            PetActivity::Napping => {
                self.energy = (self.energy + 2 + streak_bonus).min(100);
                ("睡了个好觉，精力恢复了一些。", Some("睡眠"))
            }
            PetActivity::Sunbathing => {
                self.mood = (self.mood + 2 + streak_bonus).min(100);
                ("晒了太阳，心情不错。", Some("晒太阳"))
            }
            PetActivity::Grooming => {
                self.bond = (self.bond + 1 + streak_bonus).min(100);
                ("慢条斯理地舔毛，和你更亲近了一点。", Some("舔毛"))
            }
            PetActivity::Following => {
                self.bond = (self.bond + 1 + streak_bonus).min(100);
                ("跟着你跑了一天，亲密了一点。", Some("跟脚"))
            }
            PetActivity::Playing => {
                self.mood = (self.mood + 1 + streak_bonus).min(100);
                ("自己玩得很开心。", Some("自娱自乐"))
            }
            PetActivity::Exploring => {
                self.mood = (self.mood + 1 + streak_bonus).min(100);
                ("在院子里探险，心情不错。", Some("探险"))
            }
            PetActivity::Moping => {
                self.mood = (self.mood + 1 + streak_bonus).min(100);
                ("发了会儿呆，心情缓过来一点。", Some("发呆"))
            }
            PetActivity::WindowWatching => {
                self.mood = (self.mood + 1 + streak_bonus).min(100);
                ("趴在窗边看了很久，心情平静。", Some("趴窗"))
            }
            _ => return None,
        };
        let mut message = format!("今天{text}");
        if streak >= 2 {
            message.push_str(&format!("连续{streak}天{}。", extra.unwrap_or("这样")));
        }
        Some(message)
    }

    pub fn activity(&self, now: i64) -> PetActivity {
        if self.boarded {
            return PetActivity::Boarded;
        }
        if self.sickness.is_some() {
            return PetActivity::Resting;
        }
        if self.satiety < 20 {
            return PetActivity::Begging;
        }
        if self.energy < 20 {
            return PetActivity::Napping;
        }
        if self.mood < 40 {
            return PetActivity::Moping;
        }

        let p = self.personality.primary;
        let s = self.personality.secondary;
        let q = self.personality.quirk;
        let guard = self.breed.profile().guard_bonus;
        let mut choices = vec![
            (
                PetActivity::Napping,
                2 + if p == PetTrait::Sleepy { 5 } else { 0 }
                    + if s == PetTrait::Sleepy { 2 } else { 0 },
            ),
            (
                PetActivity::Following,
                2 + if p == PetTrait::Clingy { 4 } else { 0 }
                    + if q == PetQuirk::Shadow { 5 } else { 0 },
            ),
            (
                PetActivity::Exploring,
                2 + if p == PetTrait::Curious { 4 } else { 0 }
                    + if p == PetTrait::Independent { 2 } else { 0 }
                    + if q == PetQuirk::Digger { 4 } else { 0 },
            ),
            (
                PetActivity::Patrolling,
                2 + if p == PetTrait::Alert { 4 } else { 0 } + if guard >= 8 { 2 } else { 0 },
            ),
            (
                PetActivity::Sunbathing,
                2 + if p == PetTrait::Calm { 4 } else { 0 },
            ),
            (
                PetActivity::Grooming,
                2 + if p == PetTrait::Proud { 4 } else { 0 },
            ),
            (
                PetActivity::Playing,
                2 + if p == PetTrait::Lively { 4 } else { 0 }
                    + if s == PetTrait::Lively { 2 } else { 0 },
            ),
            (
                PetActivity::Begging,
                1 + if p == PetTrait::Foodie { 4 } else { 0 }
                    + if q == PetQuirk::Beggar { 5 } else { 0 },
            ),
            (
                PetActivity::WindowWatching,
                1 + if q == PetQuirk::WindowWatcher { 6 } else { 0 },
            ),
            (
                PetActivity::Moping,
                1 + if p == PetTrait::Timid { 3 } else { 0 }
                    + if p == PetTrait::Stubborn { 2 } else { 0 },
            ),
            (
                PetActivity::Guarding,
                if self.species == PetSpecies::Dog {
                    2
                } else {
                    0
                } + if guard >= 10 { 3 } else { 0 },
            ),
        ];
        choices.retain(|(_, weight)| *weight > 0);
        let total: u32 = choices.iter().map(|(_, weight)| *weight).sum();
        let mut pick = (roll(now / 3_600_000, &self.name, "activity") % u64::from(total)) as u32;
        for (activity, weight) in choices {
            if pick < weight {
                return activity;
            }
            pick -= weight;
        }
        PetActivity::Napping
    }

    pub fn feed(&mut self, _now: i64, preferred: bool, bowl: bool) -> u64 {
        let pct = self.breed.profile().feed_pct * self.personality.feed_pct() / 100;
        let gain = if preferred { 30 } else { 24 };
        let gain = gain * pct / 100 * if bowl { 12 } else { 10 } / 10;
        let gain = gain.max(1);
        self.satiety = (self.satiety + gain).min(100);
        self.mood = (self.mood + if preferred { 10 } else { 5 }).min(100);
        self.bond = (self.bond + if preferred { 3 } else { 2 }).min(100);
        8
    }

    /// 零食：不顶饱，主要涨心情。
    pub fn snack(&mut self, satiety: u32, mood: u32, bond: u32) -> u64 {
        self.satiety = (self.satiety + satiety).min(100);
        self.mood = (self.mood + mood).min(100);
        self.bond = (self.bond + bond).min(100);
        5
    }

    pub fn touch(&mut self, now: i64) -> u64 {
        let base = (self.breed.profile().touch_pct + self.training.companion * 5) as i32
            + self.breed.signature().touch_pct;
        let mut pct = (base.max(50) as u32) * self.personality.touch_pct() / 100;
        if self.talents.has(PetTalent::Clingy) {
            pct = pct * 115 / 100;
        }
        self.mood = (self.mood + (8 * pct / 100).max(1)).min(100);
        self.bond = (self.bond + 1).min(100);
        self.last_pet_at = now;
        5
    }

    pub fn play(&mut self, now: i64, toy: bool) -> u64 {
        let base = (self.breed.profile().play_pct + self.training.companion * 5) as i32
            + self.breed.signature().play_pct;
        let pct = (base.max(50) as u32) * self.personality.play_pct() / 100;
        self.energy = self.energy.saturating_sub(
            15u32
                .saturating_sub(self.breed.signature().play_energy_save)
                .max(5),
        );
        let mood = 12 * pct / 100 * if toy { 5 } else { 4 } / 4;
        self.mood = (self.mood + mood.max(1)).min(100);
        self.bond = (self.bond + 3).min(100);
        self.last_play_at = now;
        10
    }

    pub fn heal(&mut self, _now: i64) {
        self.sickness = None;
        self.health = (self.health + 25).min(100);
        self.mood = (self.mood + 10).min(100);
    }

    pub fn dig(&mut self, now: i64) -> u64 {
        self.energy = self.energy.saturating_sub(20);
        self.mood = (self.mood + 5).min(100);
        self.bond = (self.bond + 1).min(100);
        self.last_dig_at = now;
        15
    }
}

/// 牧场岗位日结的确定性掷骰；引擎按天结算时用。
pub(crate) fn job_roll(seed: i64, name: &str) -> u64 {
    roll(seed, name, "ranch-job")
}

/// 按日期生成稳定种子；同一天同一件事掷同一个结果，跨天才会变。
pub(crate) fn day_seed(today: &str) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64;
    for byte in today.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^ (hash >> 33)
}

fn roll(now: i64, name: &str, salt: &str) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ (now as u64);
    for byte in name.bytes().chain(salt.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^ (hash >> 33)
}

/// 守卫判定，供引擎和单测共用。返回 (吓跑, 减少后的损失)。
pub fn guard_result(
    species: PetSpecies,
    bond: u32,
    energy: u32,
    sick: bool,
    loss: u32,
    roll: u64,
) -> (bool, u32) {
    guard_result_with_bonus(species, bond, energy, sick, loss, roll, 0)
}

pub fn guard_result_with_bonus(
    species: PetSpecies,
    bond: u32,
    energy: u32,
    sick: bool,
    loss: u32,
    roll: u64,
    bonus: u32,
) -> (bool, u32) {
    guard_result_full(species, bond, energy, sick, loss, roll, bonus, 0)
}

/// 带天赋「铁壁」的版本：吓跑失败后还能再减 `loss_cut` 点损失。
#[allow(clippy::too_many_arguments)]
pub fn guard_result_full(
    species: PetSpecies,
    bond: u32,
    energy: u32,
    sick: bool,
    loss: u32,
    roll: u64,
    bonus: u32,
    loss_cut: u32,
) -> (bool, u32) {
    if sick || energy < 20 {
        return (false, loss);
    }
    let scare_chance =
        (8 + bond / 4 + if species == PetSpecies::Dog { 8 } else { 3 } + bonus).min(35);
    if roll % 100 < u64::from(scare_chance) {
        return (true, 0);
    }
    (false, loss.div_ceil(2).max(1).saturating_sub(loss_cut))
}
