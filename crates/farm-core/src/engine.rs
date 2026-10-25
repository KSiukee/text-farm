use std::collections::BTreeMap;

use serde::Serialize;

use crate::achievements::{progress, ACHIEVEMENTS};
use crate::action::{Action, FacilityKind};
use crate::buffs;
use crate::content::{ContentPack, CropDef, RegionAffinity};
use crate::daily::{daily_progress, mark_claimed, DAILY_TASKS};
use crate::decor;
use crate::error::GameError;
use crate::fishing::{
    self, FishCatchResult, FishingSession, FishingSpot, RodKind, FISHING_AUTO_CAP_MS,
    FISHING_AUTO_MAX_STEPS, FISHING_SESSION_TTL_MS,
};
use crate::level_reward::{self, LEVEL_REWARD_MIN};
use crate::model::{
    facility_cost, facility_max_count, facility_name, facility_next_cost, facility_unlock_level,
    greenhouse_growth_factor, greenhouse_off_multiplier, greenhouse_tier, greenhouse_wither_chance,
    level_need, plot_tier, plot_unlock_cost, plot_unlock_level, CodexEntry, CropInstance,
    DailyState, Disaster, DisasterKind, Game, LogEntry, Pest, PestKind, PlotKind, Region, Season,
    MAX_LEVEL, PLOT_MAX_TIER,
};
use crate::npc::{
    NpcCrop, NpcMessage, HELP_DAILY_LIMIT, NPC_VISIT_CAP, NPC_VISIT_GAP_MS, RANCH_HELP_DAILY_LIMIT,
    RANCH_STEAL_DAILY_LIMIT, STEAL_DAILY_LIMIT,
};
use crate::pet::{
    day_seed, guard_result_full, job_roll, ActiveSkillKind, PetBreed, PetLocation, PetSpecies,
    PetState, PetSupplyKind, PetTalent, PetTrainingKind, PET_ACTIVE_LIMIT,
    PET_BOARD_FEE_PER_MONTH, PET_PLAY_COOLDOWN_MS, PET_TOUCH_COOLDOWN_MS,
};
use crate::planting;
use crate::processing;
use crate::ranch::{
    BreedBlocker, RanchAnimal, RanchAnimalKind, RanchRecipe, RANCH_BREED_MIN_HEALTH,
    RANCH_DIRTY_LINE, RANCH_YOUNG_MS,
};
use crate::ranching;
use crate::room::{self, RoomLayout};
use crate::skills::{self, DisciplineKind};
use crate::workshop::{
    crop_box_key, EquipmentKind, EquipmentRecipe, WorkshopJob, CROP_BOX_INPUT,
    CROP_BOX_INTERVAL_MS, WORKSHOP_COST, WORKSHOP_LEVEL,
};

const NPC_HELP_MS: i64 = 4 * 60 * 60 * 1000;
const XIAOMAN_HELP_BUFF: buffs::BuffDef = buffs::BuffDef {
    item_key: "npc:xiaoman",
    id: "npc_help_xiaoman",
    name: "小满搭把手",
    desc: "加工速度 +15%，4 小时",
    stat: buffs::BuffStat::CraftSpeed,
    value: 15,
    duration_ms: NPC_HELP_MS,
};
const QINGXING_HELP_BUFF: buffs::BuffDef = buffs::BuffDef {
    item_key: "npc:qingxing",
    id: "npc_help_qingxing",
    name: "青杏搭把手",
    desc: "畜产品品质 +15%，4 小时",
    stat: buffs::BuffStat::RanchQuality,
    value: 15,
    duration_ms: NPC_HELP_MS,
};
const LAOTAO_HELP_BUFF: buffs::BuffDef = buffs::BuffDef {
    item_key: "npc:laotao",
    id: "npc_help_laotao",
    name: "老陶搭把手",
    desc: "加工速度 +10%，4 小时",
    stat: buffs::BuffStat::CraftSpeed,
    value: 10,
    duration_ms: NPC_HELP_MS,
};
const LIUYUE_HELP_BUFF: buffs::BuffDef = buffs::BuffDef {
    item_key: "npc:liuyue",
    id: "npc_help_liuyue",
    name: "六月搭把手",
    desc: "钓鱼经验 +20%，4 小时",
    stat: buffs::BuffStat::ExpFishing,
    value: 20,
    duration_ms: NPC_HELP_MS,
};

fn job_ready_at_mut(job: &mut WorkshopJob) -> &mut i64 {
    match job {
        WorkshopJob::Ranch { ready_at, .. }
        | WorkshopJob::CropBox { ready_at, .. }
        | WorkshopJob::Equipment { ready_at, .. }
        | WorkshopJob::Bait { ready_at, .. }
        | WorkshopJob::Fish { ready_at, .. } => ready_at,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Effect {
    Log { text: String },
    LevelUp { level: u32 },
    Achievement { id: String, name: String },
    #[serde(rename_all = "camelCase")]
    Specimen {
        crop_id: String,
        crop_name: String,
        seed_id: String,
        seed_name: String,
        seed_qty: u32,
        coins: u64,
        featured: bool,
    },
    #[serde(rename_all = "camelCase")]
    NpcTalk {
        npc_id: String,
        npc_name: String,
        title: String,
        text: String,
        reward: String,
        favor: i32,
        tag: String,
        event_id: String,
        choices: Vec<crate::npc::NpcChoiceView>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub rev: u64,
    pub changed: bool,
    pub effects: Vec<Effect>,
}

fn npc_choice_views(event_id: &str) -> Vec<crate::npc::NpcChoiceView> {
    crate::npc_story::choices(event_id)
        .iter()
        .map(|choice| crate::npc::NpcChoiceView {
            id: choice.id.to_string(),
            label: choice.label.to_string(),
        })
        .collect()
}

/// 宠物牧场岗位的日结奖励；先收集再统一入库，避免借用到一半改库存。
enum RanchJobReward {
    Coins(u64),
    Item(&'static str),
    Feed,
}

/// 宠物钓鱼地岗位的日结奖励：猫叼鱼 / 狗找饵，兜底金币或面团饵。
enum FishingJobReward {
    Coins(u64),
    Bait(fishing::BaitType, u32),
    Fish(&'static fishing::FishDef, u32, u64),
}

impl Game {
    pub fn dispatch(&mut self, action: Action, now: i64) -> Result<Outcome, GameError> {
        let effects = match action {
            Action::Plant { plot, crop_id } => self.plant(plot, &crop_id, now)?,
            Action::PlantRareSeed { plot, crop_id } => {
                self.plant_rare_seed(plot, &crop_id, now)?
            }
            Action::Water { plot } => self.water(plot, now)?,
            Action::Harvest { plot } => self.harvest(plot, now)?,
            Action::HarvestAll => self.harvest_all(now)?,
            Action::Clear { plot } => self.clear(plot, now)?,
            Action::ClearPest { plot } => self.clear_pest(plot, now)?,
            Action::UnlockPlot { plot } => self.unlock_plot(plot, now)?,
            Action::UpgradePlot { plot } => self.upgrade_plot(plot, now)?,
            Action::MitigateDisaster { plot } => self.mitigate_disaster(plot, now)?,
            Action::SetRegion { region } => self.set_region(region, now)?,
            Action::CraftSpecimen { crop_id } => self.craft_specimen(&crop_id, now)?,
            Action::ClaimSpecimenMilestone { unique } => {
                self.claim_specimen_milestone(unique, now)?
            }
            Action::RepairWindmills => self.repair_windmills(now)?,
            Action::AdoptPet { name, breed, board } => self.adopt_pet(name, breed, board, now)?,
            Action::FeedPet { pet_id, fish_id } => self.feed_pet(&pet_id, &fish_id, now)?,
            Action::BuyPetFood { kind } => self.buy_pet_food(kind, now)?,
            Action::FeedPetFood { pet_id, kind } => self.feed_pet_food(&pet_id, kind, now)?,
            Action::ChatWithNpc { npc_id } => self.chat_with_npc(&npc_id, now)?,
            Action::AskNpcHelp { npc_id } => self.ask_npc_help(&npc_id, now)?,
            Action::HelpNpcRequest { npc_id } => self.help_npc_request(&npc_id, now)?,
            Action::NpcStoryChoice {
                npc_id,
                event_id,
                choice_id,
            } => self.npc_story_choice(&npc_id, &event_id, &choice_id, now)?,
            Action::GiveNpcBirthdayGift { npc_id } => {
                self.give_npc_birthday_gift(&npc_id, now)?
            }
            Action::GiveNpcGift { npc_id } => self.give_npc_gift(&npc_id, now)?,
            Action::CelebrateFestival => self.celebrate_festival(now)?,
            Action::ClaimHandbookGoal { id } => self.claim_handbook_goal(&id, now)?,
            Action::PetCompanion { pet_id } => self.touch_pet(&pet_id, now)?,
            Action::PlayWithPet { pet_id } => self.play_pet(&pet_id, now)?,
            Action::HealPet { pet_id } => self.heal_pet(&pet_id, now)?,
            Action::DigWithPet { pet_id } => self.dig_pet(&pet_id, now)?,
            Action::SetPetLocation { pet_id, location } => {
                self.set_pet_location(&pet_id, location, now)?
            }
            Action::RenamePet { pet_id, name } => self.rename_pet(&pet_id, name, now)?,
            Action::SetPetBoarded { pet_id, boarded } => {
                self.set_pet_boarded(&pet_id, boarded, now)?
            }
            Action::ReleasePet { pet_id } => self.release_pet(&pet_id, now)?,
            Action::SettleBoardingDebt => self.settle_boarding_debt(now)?,
            Action::BuyRanchAnimal { pen, kind } => self.buy_ranch_animal(pen, kind, now)?,
            Action::FeedRanchAnimal { pen } => self.feed_ranch_animal(pen, now)?,
            Action::CleanRanchPen { pen } => self.clean_ranch_pen(pen, now)?,
            Action::CollectRanchProduct { pen } => self.collect_ranch_product(pen, now)?,
            Action::HealRanchAnimal { pen } => self.heal_ranch_animal(pen, now)?,
            Action::ExpandRanch => self.expand_ranch(now)?,
            Action::FeedRanchPremium { pen } => self.feed_ranch_premium(pen, now)?,
            Action::BuyRanchFeed => self.buy_ranch_feed(now)?,
            Action::BreedRanchPair { a, b } => self.breed_ranch_pair(a, b, now)?,
            Action::RenameRanchAnimal { pen, name } => {
                self.rename_ranch_animal(pen, &name, now)?
            }
            Action::AcceptRanchOrder { id } => self.accept_ranch_order(&id, now)?,
            Action::DeliverRanchOrder { id } => self.deliver_ranch_order(&id, now)?,
            Action::EnterRanchContest { pen } => self.enter_ranch_contest(pen, now)?,
            Action::BuildPond => self.build_pond(now)?,
            Action::StockPond { fish_id, qty } => self.stock_pond(&fish_id, qty, now)?,
            Action::FeedPond => self.feed_pond(now)?,
            Action::CleanPond => self.clean_pond(now)?,
            Action::InstallPondAerator => self.install_pond_aerator(now)?,
            Action::PlantPondWeeds => self.plant_pond_weeds(now)?,
            Action::HarvestPond { fish_id } => self.harvest_pond(&fish_id, now)?,
            Action::TogglePondFry => self.toggle_pond_fry(now)?,
            Action::DeliverPondOrder => self.deliver_pond_order(now)?,
            Action::BuildWorkshop => self.build_workshop(now)?,
            Action::ExpandWorkshop => self.expand_workshop(now)?,
            Action::InstallEquipment { slot, kind } => self.install_equipment(slot, kind, now)?,
            Action::UpgradeEquipment { slot } => self.upgrade_equipment(slot, now)?,
            Action::StartRanchCraft { recipe } => self.start_ranch_craft(recipe, now)?,
            Action::StartCropCraft { crop_id } => self.start_crop_craft(&crop_id, now)?,
            Action::StartEquipmentCraft { recipe } => self.start_equipment_craft(recipe, now)?,
            Action::AutoCraftWorkshop { recipe } => self.auto_craft_workshop(recipe, now)?,
            Action::CancelWorkshopAuto => self.cancel_workshop_auto(now)?,
            Action::StartBaitCraft { recipe } => self.start_bait_craft(recipe, now)?,
            Action::StartFishCraft { recipe } => self.start_fish_craft(recipe, now)?,
            Action::CollectCraft { slot } => self.collect_craft(slot, now)?,
            Action::BuyBait {
                qty,
                quality,
                bait_type,
            } => self.buy_bait(bait_type, quality, qty, now)?,
            Action::SetBaitType { bait_type } => self.set_bait_type(bait_type, now)?,
            Action::BuyRod { rod } => self.buy_rod(rod, now)?,
            Action::SwitchRod { rod } => self.switch_rod(rod, now)?,
            Action::CastLine { spot, quality } => self.cast_line(spot, quality, now)?,
            Action::ToggleFishLock { fish_id } => self.toggle_fish_lock(&fish_id, now)?,
            Action::SellAllFish => self.sell_all_fish(now)?,
            Action::MaintainRod { rod } => self.maintain_rod(rod, now)?,
            Action::ResolveFishing { success } => self.resolve_fishing(success, now)?,
            Action::AcceptFishingCommission { id } => {
                self.accept_fishing_commission(&id, now)?
            }
            Action::ClaimFishingCommission { id } => {
                self.claim_fishing_commission(&id, now)?
            }
            Action::RefreshFishingCommissions => self.refresh_fishing_commissions(now)?,
            Action::ClaimFishingTournament { day } => {
                self.claim_fishing_tournament(&day, now)?
            }
            Action::SetAutoFishing {
                spot,
                enabled,
                rod,
                bait_quality,
                bait_type,
            } => self.set_auto_fishing(spot, enabled, rod, bait_quality, bait_type, now)?,
            Action::UpgradeFishingSkill { skill } => self.upgrade_fishing_skill(&skill, now)?,
            Action::UpgradeSkill { discipline, skill } => {
                self.upgrade_skill(discipline, &skill, now)?
            }
            Action::UseBuffItem { key } => self.use_buff_item(&key, now)?,
            Action::BuyPetSupply { kind } => self.buy_pet_supply(kind, now)?,
            Action::RepairPetSupply { kind } => self.repair_pet_supply(kind, now)?,
            Action::UsePetMedicine { pet_id } => self.use_pet_medicine(&pet_id, now)?,
            Action::UsePetSkill { pet_id } => self.use_pet_skill(&pet_id, now)?,
            Action::TrainPet { pet_id, kind } => self.train_pet(&pet_id, kind, now)?,
            Action::ChoosePetTalent { pet_id, talent } => {
                self.choose_pet_talent(&pet_id, talent, now)?
            }
            Action::BuySeed { crop_id, qty } => self.buy_seed(&crop_id, qty, now)?,
            Action::Sell { key, qty } => self.sell(&key, qty, now)?,
            Action::BuyFacility { kind } => self.buy_facility(kind, now)?,
            Action::HelpNpc { npc_id, plot } => self.help_npc(&npc_id, plot, now)?,
            Action::StealNpc { npc_id, plot } => self.steal_npc(&npc_id, plot, now)?,
            Action::HelpNpcRanch { npc_id, index } => self.help_npc_ranch(&npc_id, index, now)?,
            Action::StealNpcRanch { npc_id, index } => self.steal_npc_ranch(&npc_id, index, now)?,
            Action::NpcGreeting { npc_id } => self.npc_greeting(&npc_id, now)?,
            Action::ClaimSignIn => self.claim_sign_in(now)?,
            Action::UpdateProfile { owner_name } => self.update_profile(owner_name, now)?,
            Action::ClaimDaily { id } => self.claim_daily(id, now)?,
            Action::AcceptFarmOrder { id } => self.accept_farm_order(&id, now)?,
            Action::DeliverFarmOrder { id } => self.deliver_farm_order(&id, now)?,
            Action::StartOrderCraft { id } => self.start_order_craft(&id, now)?,
            Action::AutoCraftOrder { id } => self.auto_craft_order(&id, now)?,
            Action::BuyMysteryOffer { id } => self.buy_mystery_offer(&id, now)?,
            Action::BuyFurniture { def } => self.buy_furniture(&def, now)?,
            Action::BuyDecor { id } => self.buy_decor(&id, now)?,
            Action::UseFarmCard => self.use_farm_card(now)?,
            Action::ClaimLevelReward { level } => self.claim_level_reward(level, now)?,
            Action::UpdateSettings { exp_rate } => self.update_settings(exp_rate, now)?,
            Action::ShowGuide => self.show_guide(now)?,
            Action::DismissGuide => self.dismiss_guide(now)?,
            Action::Broadcast { text } => self.broadcast(&text, now)?,
            Action::SaveRoomLayout { room, layout } => self.save_room_layout(&room, layout, now)?,
            Action::MoveFishToAquarium { room, furniture_id, fish_id } => {
                self.move_fish_to_aquarium(&room, &furniture_id, &fish_id, now)?
            }
            Action::RemoveFishFromAquarium { room, furniture_id, fish_id } => {
                self.remove_fish_from_aquarium(&room, &furniture_id, &fish_id, now)?
            }
            Action::UnlockExtraRoom => self.unlock_extra_room(now)?,
        };

        let mut effects = effects;
        effects.extend(self.check_achievements(now));

        self.state.rev += 1;
        self.state.last_seen_at = now;
        Ok(Outcome {
            rev: self.state.rev,
            changed: true,
            effects,
        })
    }

    fn plot_pos(&self, index: u8) -> Result<usize, GameError> {
        self.state
            .plots
            .iter()
            .position(|p| p.index == index)
            .ok_or(GameError::NoSuchPlot(index))
    }

    fn plant(&mut self, index: u8, crop_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        self.plant_inner(index, crop_id, false, now)
    }

    fn plant_rare_seed(
        &mut self,
        index: u8,
        crop_id: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        self.plant_inner(index, crop_id, true, now)
    }

    fn plant_inner(
        &mut self,
        index: u8,
        crop_id: &str,
        rare: bool,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        if !self.state.plots[pos].unlocked {
            return Err(GameError::PlotLocked);
        }
        if self.state.plots[pos].crop.is_some() {
            return Err(GameError::PlotOccupied);
        }

        let def = self
            .content
            .crop(crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        if !rare && self.state.level < def.unlock_level {
            return Err(GameError::LevelTooLow {
                need: def.unlock_level,
            });
        }

        let seed_key = if rare {
            format!("rare_seed:{}", def.id)
        } else {
            format!("seed:{}", def.id)
        };
        let have = *self.state.inventory.get(&seed_key).unwrap_or(&0);
        if have == 0 {
            return Err(GameError::NotEnoughSeeds);
        }
        if have == 1 {
            self.state.inventory.remove(&seed_key);
        } else {
            self.state.inventory.insert(seed_key, have - 1);
        }

        let base = i64::from(def.grow_minutes) * 60_000;
        let windmill_active = self.state.facilities.active_windmills().min(15);
        let grow = base * (100 - i64::from(windmill_active)) / 100;
        let growth_pct =
            planting::growth_pct(self.state.skills.planting.skill_level("plant_growth"));
        let grow = grow * (100 - i64::from(growth_pct)) / 100;
        let greenhouse = self.state.plots[pos].kind == PlotKind::Greenhouse;
        let greenhouse_level = self.state.plots[pos].tier.min(PLOT_MAX_TIER);
        let region_mismatch = matches!(
            (def.region, self.state.settings.region),
            (RegionAffinity::North, Region::South) | (RegionAffinity::South, Region::North)
        );
        let wither_chance = if greenhouse {
            greenhouse_wither_chance(greenhouse_level)
        } else {
            30
        };
        let withered =
            region_mismatch && roll_wither(index, now, &def.id) % 100 < u64::from(wither_chance);
        let off_season =
            !def.sow_seasons.is_empty() && !def.sow_seasons.contains(&self.state.season);
        let factor = growth_factor_for(
            &def,
            self.state.season,
            self.state.settings.region,
            greenhouse.then_some(greenhouse_level),
        );
        let (ready_at, watered) = if self.state.facilities.sprinkler {
            // 灌溉机自动浇的水也算今天的浇水任务，否则建了灌溉机以后
            // 每日「浇水 5 次」永远做不完。
            self.state.daily.waterings += 1;
            (now + grow * 9 / 10, true)
        } else {
            (now + grow, false)
        };
        let ready_at = now + (ready_at - now) * 1000 / i64::from(factor);

        self.state.plots[pos].crop = Some(CropInstance {
            crop_id: def.id.clone(),
            planted_at: now,
            ready_at,
            season: 1,
            seasons: def.seasons,
            remaining: def.yield_per_season,
            watered,
            pest: None,
            disaster: None,
            off_season,
            growth_factor: factor,
            greenhouse,
            withered,
        });
        self.state.stats.planted += 1;

        let suffix = if self.state.facilities.sprinkler {
            "（灌溉机已浇水）"
        } else {
            ""
        };
        let suffix = if greenhouse {
            format!("{suffix}（温室，减缓季节影响）")
        } else if off_season {
            format!("{suffix}（反季，生长慢）")
        } else {
            suffix.to_string()
        };
        let suffix = if withered {
            format!("{suffix}（水土不服，枯萎了）")
        } else {
            suffix
        };
        let text = if rare {
            format!("你种下了 稀有·{}×1{}", def.name, suffix)
        } else {
            format!("你种下了 {}×1{}", def.name, suffix)
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn water(&mut self, index: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        let crop = self.state.plots[pos]
            .crop
            .clone()
            .ok_or(GameError::PlotEmpty)?;
        if crop.watered {
            return Err(GameError::AlreadyWatered);
        }
        if crop.ready_at <= now {
            return Err(GameError::AlreadyReady);
        }

        let def = self
            .content
            .crop(&crop.crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        let remaining = crop.ready_at - now;
        let water_extra =
            planting::water_extra_pct(self.state.skills.planting.skill_level("plant_water"));
        let new_ready = now + remaining * (90 - i64::from(water_extra)) / 100;

        if let Some(c) = self.state.plots[pos].crop.as_mut() {
            c.ready_at = new_ready;
            c.watered = true;
        }
        self.state.daily.waterings += 1;

        let text = format!("你给 {} 浇了水", def.name);
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        effects.extend(self.add_exp(1, now));
        Ok(effects)
    }

    fn harvest(&mut self, index: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        let crop = self.state.plots[pos]
            .crop
            .clone()
            .ok_or(GameError::PlotEmpty)?;
        if crop.pest.is_some() {
            return Err(GameError::PestBlocking);
        }
        if crop.withered {
            return Err(GameError::CropWithered);
        }
        if crop.ready_at > now {
            return Err(GameError::NotReady);
        }
        if crop.remaining == 0 {
            return Err(GameError::NothingToHarvest);
        }

        let def = self
            .content
            .crop(&crop.crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        let plant_yield_level = self.state.skills.planting.skill_level("plant_yield");
        let plant_seed_level = self.state.skills.planting.skill_level("plant_seed");
        let plant_growth_level = self.state.skills.planting.skill_level("plant_growth");
        let tier = self.state.plots[pos].tier.min(PLOT_MAX_TIER);
        let land = plot_tier(tier);
        let base = crop.remaining;
        let yield_bonus = base.saturating_mul(land.yield_pct) / 100;
        let lucky = roll_land(index, crop.planted_at, now) % 100 < u64::from(land.luck_pct);
        let luck_bonus = if lucky { (base / 2).max(1) } else { 0 };
        let mut qty = base + yield_bonus + luck_bonus;
        let disaster_penalty = crop
            .disaster
            .as_ref()
            .filter(|disaster| !disaster.kind.is_instant())
            .map(|disaster| {
                let resist = disaster_resist(&def, disaster.kind);
                disaster.kind.yield_penalty() * (100 - resist) / 100
            })
            .unwrap_or(0);
        if disaster_penalty > 0 {
            qty = (qty * (100 - disaster_penalty) / 100).max(1);
        }
        let off_penalty = if crop.off_season {
            let base = self.state.settings.region.off_season_penalty();
            if self.state.plots[pos].kind == PlotKind::Greenhouse {
                base * greenhouse_off_multiplier(self.state.plots[pos].tier.min(PLOT_MAX_TIER))
                    / 100
            } else {
                base
            }
        } else {
            0
        };
        if off_penalty > 0 {
            qty = (qty * (100 - off_penalty) / 100).max(1);
        }
        let buff_yield = buffs::plant_yield_pct(&self.state.buffs, now);
        let total_yield_pct = (planting::yield_pct(plant_yield_level)
            + buff_yield
            + crate::npc::plant_yield_pct(&self.state.npcs))
        .min(100);
        let plant_yield_bonus = qty.saturating_mul(total_yield_pct) / 100;
        qty = qty.saturating_add(plant_yield_bonus);
        let exp_gain = u32::try_from(
            u64::from(def.exp).saturating_mul(u64::from(1000 + land.exp_tenths)) / 1000,
        )
        .unwrap_or(u32::MAX);
        let coin_bonus = def
            .sell_price
            .saturating_mul(u64::from(qty))
            .saturating_mul(u64::from(land.coin_tenths))
            / 1000;

        let produce_key = format!("crop:{}", def.id);
        *self.state.inventory.entry(produce_key).or_insert(0) += qty;
        let quality_roll = roll_pest(
            (now.max(0) as u64) ^ 0x71a7_0c3d,
            index,
            &def.id,
        ) % 100;
        let prime_cut = 5 + u32::from(tier) * 3 + self.state.level / 3;
        let fine_cut = 25 + u32::from(tier) * 5 + self.state.level;
        let quality = if quality_roll < u64::from(prime_cut) {
            3
        } else if quality_roll < u64::from(fine_cut) {
            2
        } else {
            1
        };
        let today = self.state.daily.day.clone();
        if !today.is_empty() {
            self.state
                .crop_fresh_day
                .insert(def.id.clone(), today.clone());
            let mark = crate::model::CropQualityMark {
                day: today,
                quality,
            };
            let entry = self.state.crop_quality.entry(def.id.clone()).or_default();
            if entry.day == mark.day {
                entry.quality = entry.quality.max(mark.quality);
            } else {
                *entry = mark;
            }
        }
        let seed_returned = plant_seed_level > 0
            && roll_pest((now.max(0) as u64) ^ 0x5eed, index, &def.id) % 100
                < u64::from(planting::seed_return_pct(plant_seed_level));
        if seed_returned {
            *self
                .state
                .inventory
                .entry(format!("seed:{}", def.id))
                .or_insert(0) += 1;
        }
        self.state.coins = self.state.coins.saturating_add(coin_bonus);
        self.state.stats.coins_earned = self.state.stats.coins_earned.saturating_add(coin_bonus);
        self.state.stats.harvests += 1;
        self.state.daily.harvests += 1;
        let planting_exp = planting::harvest_exp(&def, qty, self.state.daily.harvests);
        let planting_pct = buffs::exp_pct(&self.state.buffs, now, DisciplineKind::Planting);
        let planting_levels = self
            .state
            .skills
            .planting
            .add_exp(planting_exp * u64::from(100 + planting_pct) / 100);

        let entry = self
            .state
            .codex
            .entry(def.id.clone())
            .or_insert(CodexEntry {
                first_at: now,
                total: 0,
                best: 0,
            });
        entry.total += u64::from(qty);
        entry.best = entry.best.max(qty);

        let mut parts = vec![format!("你收获了 {}×{}", def.name, qty)];
        if quality >= 2 {
            parts.push(if quality == 3 { "特级品相" } else { "精品品相" }.into());
        }
        if yield_bonus > 0 {
            parts.push(format!("土地收成+{}", yield_bonus));
        }
        if plant_yield_bonus > 0 {
            parts.push(format!("收成加成+{}", plant_yield_bonus));
        }
        if seed_returned {
            parts.push("留种+1".into());
        }
        if lucky {
            parts.push("幸运丰收".into());
        }
        if disaster_penalty > 0 {
            parts.push(format!("灾害减产{}%", disaster_penalty));
        }
        if off_penalty > 0 {
            parts.push(format!("反季减产{}%", off_penalty));
        }
        if coin_bonus > 0 {
            parts.push(format!("土地金币+{}", coin_bonus));
        }
        parts.push(format!("经验+{}", exp_gain));
        let text = parts.join("，");
        self.push_log(now, text.clone());

        if crop.season < crop.seasons {
            if let Some(c) = self.state.plots[pos].crop.as_mut() {
                c.season += 1;
                c.ready_at = now
                    + i64::from(def.regrow_minutes)
                        * 60_000
                        * (100 - i64::from(planting::growth_pct(plant_growth_level)))
                        / 100;
                c.remaining = def.yield_per_season;
                c.watered = false;
                c.disaster = None;
            }
        } else {
            self.state.plots[pos].crop = None;
        }

        let mut effects = vec![Effect::Log { text }];
        for _ in 0..planting_levels {
            let text = format!("种植等级升到 {} 级", self.state.skills.planting.level);
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
        effects.extend(self.add_exp(exp_gain, now));
        Ok(effects)
    }

    fn harvest_all(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if !self.state.facilities.harvester {
            return Err(GameError::NeedHarvester);
        }

        let ready: Vec<u8> = self
            .state
            .plots
            .iter()
            .filter(|p| {
                p.crop
                    .as_ref()
                    .is_some_and(|c| c.ready_at <= now && c.remaining > 0)
            })
            .map(|p| p.index)
            .collect();

        if ready.is_empty() {
            return Err(GameError::NothingToHarvest);
        }

        let mut effects = Vec::new();
        for index in ready {
            effects.extend(self.harvest(index, now)?);
        }
        Ok(effects)
    }

    fn clear(&mut self, index: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        let crop = self.state.plots[pos]
            .crop
            .as_ref()
            .ok_or(GameError::PlotEmpty)?;
        let def = self
            .content
            .crop(&crop.crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        self.state.plots[pos].crop = None;

        let text = format!("你铲掉了 {}", def.name);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn unlock_plot(&mut self, index: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        if self.state.plots[pos].unlocked {
            return Err(GameError::AlreadyUnlocked);
        }
        let need = plot_unlock_level(index);
        if self.state.level < need {
            return Err(GameError::LevelTooLow { need });
        }
        let cost = plot_unlock_cost(index);
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        self.state.plots[pos].unlocked = true;
        self.state.stats.plots_unlocked += 1;

        let text = format!("你开垦了 {} 号地", index);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn upgrade_plot(&mut self, index: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        if !self.state.plots[pos].unlocked {
            return Err(GameError::PlotLocked);
        }
        let tier = self.state.plots[pos].tier.min(PLOT_MAX_TIER);
        if tier >= PLOT_MAX_TIER {
            return Err(GameError::PlotMaxTier);
        }
        let is_greenhouse = self.state.plots[pos].kind == PlotKind::Greenhouse;
        let info = if is_greenhouse {
            greenhouse_tier(tier)
        } else {
            plot_tier(tier)
        };
        if self.state.level < info.upgrade_level {
            return Err(GameError::LevelTooLow {
                need: info.upgrade_level,
            });
        }
        if self.state.coins < info.upgrade_cost {
            return Err(GameError::NotEnoughCoins {
                need: info.upgrade_cost,
            });
        }

        self.state.coins -= info.upgrade_cost;
        self.state.plots[pos].tier = tier + 1;
        self.state.stats.plot_upgrades += 1;
        self.recalc_growth(now);
        let next = if is_greenhouse {
            greenhouse_tier(tier + 1)
        } else {
            plot_tier(tier + 1)
        };
        let text = format!(
            "{} 号地升级为「{}」，花费 {} 金币",
            index, next.name, info.upgrade_cost
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn mitigate_disaster(&mut self, index: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        let disaster = self.state.plots[pos]
            .crop
            .as_ref()
            .and_then(|crop| crop.disaster.clone())
            .ok_or(GameError::NoDisaster)?;
        if disaster.kind == DisasterKind::Hail {
            return Err(GameError::HailNoAction);
        }
        let cost = if disaster.kind == DisasterKind::Blight {
            30
        } else {
            0
        };
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;

        let until = now.min(disaster.ends_at);
        let pause = (until - disaster.since).max(0) * i64::from(disaster.kind.growth_slow()) / 100;
        if let Some(crop) = self.state.plots[pos].crop.as_mut() {
            crop.ready_at = crop.ready_at.saturating_add(pause);
            crop.disaster = None;
        }
        self.state.stats.disasters_mitigated += 1;

        let text = format!(
            "你{}处理了{}的{}，生长恢复",
            disaster.kind.mitigation_label(),
            index,
            disaster.kind.label()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn set_region(&mut self, region: Region, now: i64) -> Result<Vec<Effect>, GameError> {
        self.state.settings.region = region;
        let changed = self.recalc_growth(now);
        let text = if changed > 0 {
            format!(
                "气候区改为{}，{} 块地的生长速度已重算",
                region.label(),
                changed
            )
        } else {
            format!("气候区改为{}", region.label())
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn recalc_growth(&mut self, now: i64) -> usize {
        let content = self.content.clone();
        let mut changed = 0;
        for plot in &mut self.state.plots {
            let greenhouse_level = if plot.kind == PlotKind::Greenhouse {
                Some(plot.tier.min(PLOT_MAX_TIER))
            } else {
                None
            };
            let Some(crop) = plot.crop.as_mut() else {
                continue;
            };
            if crop.ready_at <= now {
                continue;
            }
            let Some(def) = content.crop(&crop.crop_id) else {
                continue;
            };
            let factor = growth_factor_for(
                def,
                self.state.season,
                self.state.settings.region,
                greenhouse_level,
            );
            if factor == crop.growth_factor {
                continue;
            }
            let remaining = crop.ready_at - now;
            let adjusted = remaining
                .saturating_mul(i64::from(crop.growth_factor))
                .checked_div(i64::from(factor))
                .unwrap_or(remaining)
                .max(1);
            crop.ready_at = now + adjusted;
            crop.growth_factor = factor;
            changed += 1;
        }
        changed
    }

    fn craft_specimen(&mut self, crop_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let def = self
            .content
            .crop(crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        let recipe = crate::specimen::recipe(&def);
        let key = format!("crop:{}", def.id);
        let have = *self.state.inventory.get(&key).unwrap_or(&0);
        if have < recipe.need_qty {
            return Err(GameError::NotEnoughItems);
        }
        if self.state.coins < recipe.coins {
            return Err(GameError::NotEnoughCoins { need: recipe.coins });
        }

        let new_have = have - recipe.need_qty;
        if new_have == 0 {
            self.state.inventory.remove(&key);
        } else {
            self.state.inventory.insert(key, new_have);
        }
        self.state.coins -= recipe.coins;
        let featured = crate::specimen::featured_crop(&self.content, &self.state.daily.day)
            .is_some_and(|crop| crop.id == def.id);
        let (reward, seeds) = crate::specimen::roll_reward(&self.content, &def, now);
        let _ = reward;
        let reward_id = reward.id.clone();
        let reward_name = reward.name.clone();
        *self
            .state
            .inventory
            .entry(format!("seed:{reward_id}"))
            .or_insert(0) += seeds;
        *self
            .state
            .specimen_collection
            .entry(def.id.clone())
            .or_insert(0) += 1;
        let mut coin_reward = recipe.coins / 2;
        if featured {
            coin_reward += recipe.coins / 4;
        }
        self.state.coins = self.state.coins.saturating_add(coin_reward);
        self.state.stats.specimens_crafted += 1;
        self.state.stats.rare_seeds_gained += u64::from(seeds);

        let recommendation = if featured { "（今日推荐 +50%）" } else { "" };
        let text = format!(
            "制作{}标本：消耗 {}×{} + {}金币，获得 {}种子×{} + {}金币{}",
            def.name,
            def.name,
            recipe.need_qty,
            recipe.coins,
            reward_name,
            seeds,
            coin_reward,
            recommendation
        );
        self.push_log(now, text.clone());
        Ok(vec![
            Effect::Log { text },
            Effect::Specimen {
                crop_id: def.id.clone(),
                crop_name: def.name.clone(),
                seed_id: reward_id,
                seed_name: reward_name,
                seed_qty: seeds,
                coins: coin_reward,
                featured,
            },
        ])
    }

    fn claim_specimen_milestone(
        &mut self,
        unique: u32,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let current = self
            .state
            .specimen_collection
            .values()
            .filter(|count| **count > 0)
            .count() as u32;
        let Some((need, reward)) = crate::specimen::MILESTONES
            .iter()
            .find(|(need, _)| *need == unique)
        else {
            return Err(GameError::SpecimenMilestoneLocked);
        };
        if current < *need {
            return Err(GameError::SpecimenMilestoneLocked);
        }
        if self.state.specimen_milestones_claimed.contains(need) {
            return Err(GameError::AlreadyClaimed);
        }
        self.state.specimen_milestones_claimed.insert(*need);
        self.state.coins = self.state.coins.saturating_add(*reward);
        let text = format!("标本图鉴达到 {need} 种，领取金币+{reward}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn update_disasters(&mut self, now: i64, today: &str) -> Vec<Effect> {
        let mut effects = Vec::new();
        for pos in 0..self.state.plots.len() {
            let Some(crop) = self.state.plots[pos].crop.clone() else {
                continue;
            };
            let plot = self.state.plots[pos].index;

            if let Some(disaster) = crop.disaster.clone() {
                if now < disaster.ends_at {
                    continue;
                }
                let until = now.min(disaster.ends_at);
                let pause =
                    (until - disaster.since).max(0) * i64::from(disaster.kind.growth_slow()) / 100;
                if let Some(c) = self.state.plots[pos].crop.as_mut() {
                    c.ready_at = c.ready_at.saturating_add(pause);
                    c.disaster = None;
                }
                let text = format!("{}的{}结束了，生长恢复", plot, disaster.kind.label());
                self.push_log(now, text.clone());
                effects.push(Effect::Log { text });
                continue;
            }

            if crop.ready_at <= now || crop.pest.is_some() {
                continue;
            }
            let grow = (crop.ready_at - crop.planted_at).max(1);
            if now - crop.planted_at < grow / 4 {
                continue;
            }
            let Some(kind) = roll_disaster(now, today, plot, crop.planted_at) else {
                continue;
            };
            let Some(def) = self.content.crop(&crop.crop_id).cloned() else {
                continue;
            };
            if kind == DisasterKind::Hail && crop.remaining <= 1 {
                continue;
            }
            let resist = disaster_resist(&def, kind);
            if kind != DisasterKind::Hail
                && resist > 0
                && roll_pest((now / 60_000) as u64, plot, "resist") % 100 < u64::from(resist)
            {
                continue;
            }

            let seed = roll_pest((now / 3_600_000) as u64, plot, "disaster");
            let ends_at = if kind == DisasterKind::Hail {
                now + 30 * 60_000
            } else {
                let hours = match kind {
                    DisasterKind::Drought => 4 + seed % 5,
                    DisasterKind::Flood => 3 + seed % 4,
                    DisasterKind::Frost => 2 + seed % 3,
                    DisasterKind::Blight => 4 + seed % 5,
                    DisasterKind::Typhoon => 2 + seed % 3,
                    DisasterKind::Hail => 0,
                };
                now + i64::try_from(hours).unwrap_or(4) * 3_600_000
            };

            let text = if kind == DisasterKind::Hail {
                let pct = 15 + seed % 11;
                let loss = (crop.remaining * u32::try_from(pct).unwrap_or(15) / 100)
                    .max(1)
                    .min(crop.remaining - 1)
                    * (100 - resist)
                    / 100;
                if let Some(c) = self.state.plots[pos].crop.as_mut() {
                    c.remaining = c.remaining.saturating_sub(loss);
                    c.disaster = Some(Disaster {
                        kind,
                        since: now,
                        ends_at,
                    });
                }
                format!("{}的作物遭了冰雹，减产 {} 个", plot, loss)
            } else {
                if let Some(c) = self.state.plots[pos].crop.as_mut() {
                    c.disaster = Some(Disaster {
                        kind,
                        since: now,
                        ends_at,
                    });
                }
                format!(
                    "{}的作物遭遇{}，{}，预计减产{}%",
                    plot,
                    kind.label(),
                    if kind.growth_slow() >= 100 {
                        "生长暂停"
                    } else {
                        "生长减半"
                    },
                    kind.yield_penalty()
                )
            };
            self.state.stats.disasters_spawned += 1;
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });

            if matches!(
                kind,
                DisasterKind::Flood
                    | DisasterKind::Hail
                    | DisasterKind::Frost
                    | DisasterKind::Typhoon
            ) {
                if let Some(damage) = self.damage_windmill(now) {
                    self.push_log(now, damage.clone());
                    effects.push(Effect::Log { text: damage });
                }
            }
        }
        effects
    }

    fn buy_seed(&mut self, crop_id: &str, qty: u32, now: i64) -> Result<Vec<Effect>, GameError> {
        if qty == 0 {
            return Err(GameError::InvalidQty);
        }
        let def = self
            .content
            .crop(crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        if self.state.level < def.unlock_level {
            return Err(GameError::LevelTooLow {
                need: def.unlock_level,
            });
        }
        let total = def
            .seed_price
            .checked_mul(u64::from(qty))
            .ok_or(GameError::Overflow)?;
        if self.state.coins < total {
            return Err(GameError::NotEnoughCoins { need: total });
        }

        self.state.coins -= total;
        *self
            .state
            .inventory
            .entry(format!("seed:{}", def.id))
            .or_insert(0) += qty;
        self.state.stats.seeds_bought += u64::from(qty);

        let text = format!("你买了 {}种子×{}，花费 {}", def.name, qty, total);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn sell(&mut self, key: &str, qty: u32, now: i64) -> Result<Vec<Effect>, GameError> {
        if qty == 0 {
            return Err(GameError::InvalidQty);
        }
        let (kind, id) = key.split_once(':').ok_or(GameError::UnknownItem)?;
        let (unit_price, label) = match kind {
            "crop" => {
                let def = self
                    .content
                    .crop(id)
                    .cloned()
                    .ok_or(GameError::UnknownCrop)?;
                (def.sell_price, def.name)
            }
            "ranch" => {
                if !crate::ranch::is_sellable(key) {
                    return Err(GameError::OnlyProduceSellable);
                }
                let price = crate::ranch::price_of(key).ok_or(GameError::UnknownItem)?;
                let label = crate::ranch::display_name(key).ok_or(GameError::UnknownItem)?;
                (price, label)
            }
            "fish" => {
                if self.state.fishing.locked_fish.contains(id) {
                    return Err(GameError::FishLocked);
                }
                let def = fishing::fish(id).ok_or(GameError::UnknownFish)?;
                let price = self
                    .state
                    .fishing
                    .fish_batches
                    .get(id)
                    .map(|batch| batch.unit_price())
                    .unwrap_or(def.price);
                (price, def.name.to_string())
            }
            "craft" => {
                if let Some(crop_id) = id.strip_prefix("box:") {
                    let def = self.content.crop(crop_id).ok_or(GameError::UnknownCrop)?;
                    (
                        crate::workshop::crop_box_price(def.sell_price),
                        format!("{}礼盒", def.name),
                    )
                } else {
                    let def = crate::workshop::craft_item(key).ok_or(GameError::UnknownItem)?;
                    (def.price, def.name.to_string())
                }
            }
            _ => return Err(GameError::OnlyProduceSellable),
        };
        let have = *self.state.inventory.get(key).unwrap_or(&0);
        if have < qty {
            return Err(GameError::NotEnoughItems);
        }
        let total = if kind == "fish" {
            if let Some(batch) = self.state.fishing.fish_batches.get_mut(id) {
                batch.take_value(qty)
            } else {
                unit_price
                    .checked_mul(u64::from(qty))
                    .ok_or(GameError::Overflow)?
            }
        } else {
            unit_price
                .checked_mul(u64::from(qty))
                .ok_or(GameError::Overflow)?
        };
        let coin_pct = u64::from(
            crate::buffs::coin_gain_pct(&self.state.buffs, now)
                + crate::npc::coin_gain_pct(&self.state.npcs),
        );
        let total = total.saturating_add(total.saturating_mul(coin_pct) / 100);

        if have == qty {
            self.state.inventory.remove(key);
        } else {
            self.state.inventory.insert(key.to_string(), have - qty);
        }
        self.state.coins = self
            .state
            .coins
            .checked_add(total)
            .ok_or(GameError::Overflow)?;
        self.state.stats.coins_earned = self
            .state
            .stats
            .coins_earned
            .checked_add(total)
            .ok_or(GameError::Overflow)?;
        self.state.stats.items_sold += u64::from(qty);

        let text = format!("你卖掉了 {}×{}，金币+{}", label, qty, total);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn buy_bait(
        &mut self,
        bait_type: fishing::BaitType,
        quality: fishing::BaitQuality,
        qty: u32,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if qty == 0 || qty > 99 {
            return Err(GameError::InvalidQty);
        }
        let total = quality
            .price()
            .checked_mul(u64::from(qty))
            .ok_or(GameError::Overflow)?;
        if self.state.coins < total {
            return Err(GameError::NotEnoughCoins { need: total });
        }
        self.state.coins -= total;
        self.state.fishing.add_bait_of(bait_type, quality, qty);
        let text = format!(
            "你买了 {qty} 份{}{}鱼饵，金币-{total}",
            bait_type.label(),
            quality.label()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn set_bait_type(
        &mut self,
        bait_type: fishing::BaitType,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        self.state.fishing.active_bait_type = bait_type;
        let text = format!("当前鱼饵换成{}", bait_type.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn toggle_fish_lock(&mut self, fish_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        if fishing::fish(fish_id).is_none() {
            return Err(GameError::UnknownFish);
        }
        let locked = if self.state.fishing.locked_fish.remove(fish_id) {
            false
        } else {
            self.state.fishing.locked_fish.insert(fish_id.to_string());
            true
        };
        let name = fishing::fish(fish_id)
            .map(|def| def.name)
            .unwrap_or(fish_id);
        let text = if locked {
            format!("已锁定 {name}，不会一键卖出")
        } else {
            format!("已解锁 {name}")
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn sell_all_fish(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let keys: Vec<String> = self
            .state
            .inventory
            .keys()
            .filter(|key| key.starts_with("fish:"))
            .cloned()
            .collect();
        let mut sold = 0u32;
        let mut total = 0u64;
        for key in keys {
            let id = key.strip_prefix("fish:").unwrap_or(&key).to_string();
            if self.state.fishing.locked_fish.contains(&id) {
                continue;
            }
            let Some(qty) = self.state.inventory.get(&key).copied() else {
                continue;
            };
            if qty == 0 {
                continue;
            }
            let value = if let Some(batch) = self.state.fishing.fish_batches.get_mut(&id) {
                batch.take_value(qty)
            } else {
                fishing::fish(&id)
                    .map(|def| def.price.saturating_mul(u64::from(qty)))
                    .unwrap_or(0)
            };
            self.state.inventory.remove(&key);
            sold = sold.saturating_add(qty);
            total = total.saturating_add(value);
        }
        if sold == 0 {
            return Err(GameError::NothingToSell);
        }
        let coin_pct = u64::from(
            crate::buffs::coin_gain_pct(&self.state.buffs, now)
                + crate::npc::coin_gain_pct(&self.state.npcs),
        );
        let total = total.saturating_add(total.saturating_mul(coin_pct) / 100);
        self.state.coins = self.state.coins.saturating_add(total);
        let text = format!("一键卖出 {sold} 条鱼，金币+{total}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn ensure_no_fishing_session(&self, now: i64) -> Result<(), GameError> {
        if let Some(session) = self.state.fishing.session.as_ref() {
            if now < session.expires_at {
                return Err(GameError::FishingSessionActive);
            }
        }
        Ok(())
    }

    fn maintain_rod(&mut self, rod: RodKind, now: i64) -> Result<Vec<Effect>, GameError> {
        self.ensure_no_fishing_session(now)?;
        if !self.state.fishing.owns_rod(rod) {
            return Err(GameError::FishingRodLocked {
                need: rod.unlock_level(),
            });
        }
        if !self.state.fishing.needs_maintenance(rod) {
            let text = format!("{}还不需要保养", rod.label());
            return Ok(vec![Effect::Log { text }]);
        }
        let cost = rod.maintenance_cost();
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        self.state.fishing.maintain_rod(rod);
        let text = format!("{}保养好了，金币-{cost}", rod.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn buy_rod(&mut self, rod: RodKind, now: i64) -> Result<Vec<Effect>, GameError> {
        self.ensure_no_fishing_session(now)?;
        if self.state.fishing.owns_rod(rod) {
            return Err(GameError::FishingRodOwned);
        }
        if !self
            .state
            .fishing
            .rod_unlocked(self.state.skills.fishing.level, rod)
        {
            return Err(GameError::FishingRodLocked {
                need: rod.unlock_level(),
            });
        }
        let cost = rod.cost();
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        self.state.fishing.owned_rods.insert(rod);
        self.state.fishing.rod = rod;
        let text = format!(
            "你换上了{}（框 {}%），金币-{}",
            rod.label(),
            rod.frame_pct(),
            cost
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn switch_rod(&mut self, rod: RodKind, now: i64) -> Result<Vec<Effect>, GameError> {
        self.ensure_no_fishing_session(now)?;
        if !self.state.fishing.owns_rod(rod) {
            return Err(GameError::FishingRodLocked {
                need: rod.unlock_level(),
            });
        }
        if rod == self.state.fishing.rod {
            return Ok(Vec::new());
        }
        self.state.fishing.rod = rod;
        let text = format!("换回{}（框 {}%）", rod.label(), rod.frame_pct());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn cast_line(
        &mut self,
        spot: FishingSpot,
        quality: Option<fishing::BaitQuality>,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self
            .state
            .fishing
            .spot_unlocked(self.state.skills.fishing.level, spot)
        {
            return Err(GameError::FishingSpotLocked {
                need: spot.unlock_level(),
            });
        }
        if let Some(session) = self.state.fishing.session.as_ref() {
            if now < session.expires_at {
                return Err(GameError::FishingSessionActive);
            }
        }
        if self.state.fishing.session.take().is_some() {
            self.state.fishing.stats.fails += 1;
        }
        let quality = quality.unwrap_or(self.state.fishing.active_bait);
        self.state.fishing.active_bait = quality;
        let bait_type = self.state.fishing.active_bait_type;
        if self.state.fishing.bait_count_of(bait_type, quality) == 0 {
            return Err(GameError::FishingBaitEmpty);
        }

        self.state.fishing.stats.casts += 1;
        self.state.fishing.session_seq += 1;
        let seq = self.state.fishing.session_seq;
        let rod = self.state.fishing.rod;
        let saver = fishing::bait_saver_pct(self.state.skills.fishing.skill_level("bait_saver"))
            .saturating_add(rod.bait_save_pct())
            .min(100);
        let saved = saver > 0
            && roll_pest((now.max(0) as u64) ^ seq, spot as u8, "bait-saver") % 100
                < u64::from(saver);
        if !saved {
            self.state.fishing.take_bait_of(bait_type, quality);
        }
        let roll = roll_pest((now.max(0) as u64) ^ seq, spot as u8, "cast");
        let condition = fishing::FishingCondition::at(&self.state.daily.day, now);
        let def = fishing::pick_fish_with_bait_condition(spot, roll, bait_type, condition);
        let personality = fishing::pick_personality(
            def,
            roll_pest(
                (now.max(0) as u64) ^ seq ^ 0x7f4a,
                spot as u8,
                "personality",
            ),
        );
        self.state.fishing.session = Some(FishingSession {
            id: seq,
            spot,
            fish_id: def.id.to_string(),
            started_at: now,
            expires_at: now + FISHING_SESSION_TTL_MS,
            bait_quality: quality,
            personality,
        });

        let text = format!("你在{}抛了一竿，浮标动了……", spot.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn resolve_fishing(&mut self, success: bool, now: i64) -> Result<Vec<Effect>, GameError> {
        let session = self
            .state
            .fishing
            .session
            .take()
            .ok_or(GameError::FishingNoSession)?;
        if now > session.expires_at {
            self.state.fishing.stats.fails += 1;
            let text = "这一竿放太久了，鱼跑了".to_string();
            self.push_log(now, text.clone());
            return Ok(vec![Effect::Log { text }]);
        }
        let def = fishing::fish(&session.fish_id).ok_or(GameError::UnknownFish)?;
        if !success {
            self.state.fishing.stats.fails += 1;
            let text = "鱼挣脱了，这一竿空了".to_string();
            self.push_log(now, text.clone());
            return Ok(vec![Effect::Log { text }]);
        }

        let weight_roll = roll_pest(
            (now.max(0) as u64) ^ session.id,
            session.spot as u8,
            "weight",
        );
        let rod = self.state.fishing.rod;
        let quality_roll = roll_pest(
            (now.max(0) as u64) ^ session.id ^ 0x51ed,
            session.spot as u8,
            "quality",
        );
        let quality = fishing::quality_for_with_bait(rod, session.bait_quality, quality_roll);
        let weight_tenths =
            fishing::weight_tenths_for_bait(rod, session.bait_quality, def, weight_roll, quality);
        let (quality, weight_tenths) = fishing::apply_manual_bonus(quality, weight_tenths);
        let weight_tenths = weight_tenths.saturating_add(fishing::big_fish_bonus_tenths(
            self.state.skills.fishing.skill_level("big_fish"),
        ));
        let market_roll = roll_pest(
            (now.max(0) as u64) ^ session.id ^ 0x9e37,
            session.spot as u8,
            "market",
        );
        let market = fishing::market_pct(market_roll);
        let golden = fishing::golden_session(session.id, session.spot, rod);
        let mut price = fishing::price_for(def, weight_tenths, quality, market);
        price = fishing::sell_price(price, rod);
        if golden {
            price = price.max(fishing::golden_floor_price(session.spot));
        }
        let key = format!("fish:{}", def.id);
        *self.state.inventory.entry(key).or_insert(0) += 1;
        self.state
            .fishing
            .fish_batches
            .entry(def.id.to_string())
            .or_default()
            .add(weight_tenths, quality, price);
        self.state.fishing.stats.catches += 1;
        self.state.daily.fish_catches = self.state.daily.fish_catches.saturating_add(1);
        self.state.fishing.add_rod_wear(rod);
        let is_record =
            self.state
                .fishing
                .record_catch(def, weight_tenths, quality, price, false, now);
        self.state.fishing.quests.on_catch(def, weight_tenths);
        if golden {
            *self
                .state
                .fishing
                .golden_codex
                .entry(def.id.to_string())
                .or_insert(0) += 1;
            self.state.fishing.stats.golden_catches += 1;
        }
        self.state.fishing.last_catch = Some(FishCatchResult {
            fish_id: def.id.to_string(),
            weight_tenths,
            quality,
            price,
            market_pct: market,
            at: now,
            golden,
            is_record,
        });
        let exp = fishing::rod_exp(
            fishing::catch_exp(def.rarity, quality, weight_tenths),
            rod,
        );
        let pct = buffs::exp_pct(&self.state.buffs, now, DisciplineKind::Fishing)
            + crate::npc::fishing_exp_pct(&self.state.npcs);
        let levels = self
            .state
            .skills
            .fishing
            .add_exp(exp * u64::from(100 + pct) / 100);
        let mut effects = Vec::new();
        for _ in 0..levels {
            let text = format!("钓鱼等级升到 {} 级", self.state.skills.fishing.level);
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
        let mut text = format!(
            "钓到 {}（{:.1}kg，{}），价值 {} 金币",
            def.name,
            f64::from(weight_tenths) / 10.0,
            fishing::quality_label(quality),
            price
        );
        if golden {
            text.push_str("，金色变异！");
        }
        if is_record {
            text.push_str("，新纪录！");
        }
        self.push_log(now, text.clone());
        effects.push(Effect::Log { text });
        for milestone in self.fishing_milestones() {
            self.push_log(now, milestone.clone());
            effects.push(Effect::Log { text: milestone });
        }
        Ok(effects)
    }

    /// 图鉴里程碑：分钓点集齐 + 35/35 全图鉴；每个只发一次。
    fn fishing_milestones(&mut self) -> Vec<String> {
        let mut texts = Vec::new();
        for spot in FishingSpot::ALL {
            if self.state.fishing.spot_rewards.contains(spot.key()) {
                continue;
            }
            if !self.state.fishing.spot_complete(spot) {
                continue;
            }
            self.state
                .fishing
                .spot_rewards
                .insert(spot.key().to_string());
            let (coins, bait_type, quality, qty) = fishing::spot_reward(spot);
            self.state.coins = self.state.coins.saturating_add(coins);
            self.state.fishing.add_bait_of(bait_type, quality, qty);
            texts.push(format!(
                "{}图鉴集齐：金币+{coins}，{}{}鱼饵×{qty}",
                spot.label(),
                bait_type.label(),
                quality.label()
            ));
        }
        if !self.state.fishing.full_codex_claimed
            && self.state.fishing.caught_count() == fishing::FISH.len()
        {
            self.state.fishing.full_codex_claimed = true;
            self.state.coins = self.state.coins.saturating_add(fishing::FULL_CODEX_REWARD);
            let (bait_type, quality, qty) = fishing::FULL_CODEX_BAIT;
            self.state.fishing.add_bait_of(bait_type, quality, qty);
            texts.push(format!(
                "全图鉴 35/35 达成：金币+{}，{}{}鱼饵×{qty}",
                fishing::FULL_CODEX_REWARD,
                bait_type.label(),
                quality.label()
            ));
        }
        texts
    }
    fn accept_fishing_commission(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.fishing.quests.active.len() >= 2 {
            return Err(GameError::FishingCommissionLimit);
        }
        let Some(offer) = self
            .state
            .fishing
            .quests
            .offers
            .iter()
            .find(|offer| offer.id == id)
        else {
            return Err(GameError::FishingCommissionInvalid);
        };
        let title = offer.title.clone();
        if !self.state.fishing.quests.accept(id) {
            return Err(GameError::FishingCommissionInvalid);
        }
        let text = format!("接下钓鱼委托：{title}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn claim_fishing_commission(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let Some(active) = self
            .state
            .fishing
            .quests
            .active
            .iter()
            .find(|commission| commission.id == id)
        else {
            return Err(GameError::FishingCommissionInvalid);
        };
        if !active.complete() {
            return Err(GameError::FishingCommissionNotReady);
        }
        let commission = self
            .state
            .fishing
            .quests
            .claim_commission(id)
            .ok_or(GameError::FishingCommissionNotReady)?;
        self.state.coins = self.state.coins.saturating_add(commission.reward_coins);
        self.state
            .fishing
            .add_bait(fishing::BaitQuality::Normal, commission.reward_bait);
        let levels = self.state.skills.fishing.add_exp(commission.reward_exp);
        let mut texts = vec![format!(
            "完成{}的钓鱼委托：金币+{}，普通鱼饵+{}，钓鱼经验+{}",
            commission.npc_name,
            commission.reward_coins,
            commission.reward_bait,
            commission.reward_exp
        )];
        for _ in 0..levels {
            texts.push(format!("钓鱼等级升到 {} 级", self.state.skills.fishing.level));
        }
        for text in &texts {
            self.push_log(now, text.clone());
        }
        Ok(texts.into_iter().map(|text| Effect::Log { text }).collect())
    }

    fn refresh_fishing_commissions(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if !self
            .state
            .fishing
            .quests
            .refresh(self.state.skills.fishing.level)
        {
            return Err(GameError::FishingCommissionRefreshUsed);
        }
        let text = "换了一批钓鱼委托".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn claim_fishing_tournament(&mut self, day: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let Some(result) = self.state.fishing.quests.claim_tournament(day) else {
            return Err(GameError::FishingTournamentInvalid);
        };
        self.state.coins = self.state.coins.saturating_add(result.reward_coins);
        self.state
            .fishing
            .add_bait(fishing::BaitQuality::Normal, result.reward_bait);
        let levels = self.state.skills.fishing.add_exp(result.reward_exp);
        let mut texts = vec![format!(
            "{}：金币+{}，普通鱼饵+{}，钓鱼经验+{}",
            result.message, result.reward_coins, result.reward_bait, result.reward_exp
        )];
        for _ in 0..levels {
            texts.push(format!("钓鱼等级升到 {} 级", self.state.skills.fishing.level));
        }
        for text in &texts {
            self.push_log(now, text.clone());
        }
        Ok(texts.into_iter().map(|text| Effect::Log { text }).collect())
    }

    fn set_auto_fishing(
        &mut self,
        spot: FishingSpot,
        enabled: bool,
        rod: Option<RodKind>,
        bait_quality: Option<fishing::BaitQuality>,
        bait_type: Option<fishing::BaitType>,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self
            .state
            .fishing
            .spot_unlocked(self.state.skills.fishing.level, spot)
        {
            return Err(GameError::FishingSpotLocked {
                need: spot.unlock_level(),
            });
        }
        let rod = rod.unwrap_or(self.state.fishing.auto.rod);
        if rod > self.state.fishing.rod {
            return Err(GameError::FishingRodLocked {
                need: rod.unlock_level(),
            });
        }
        let bait_quality = bait_quality.unwrap_or(self.state.fishing.auto.bait_quality);
        let bait_type = bait_type.unwrap_or(self.state.fishing.auto.bait_type);
        self.state.fishing.auto.spot = spot;
        self.state.fishing.auto.rod = rod;
        self.state.fishing.auto.bait_quality = bait_quality;
        self.state.fishing.auto.bait_type = bait_type;
        self.state.fishing.auto.enabled = enabled;
        if enabled {
            self.state.fishing.auto.last_at = now;
        }
        let text = if enabled {
            format!(
                "{}的自动钓鱼开着了（{} + {}{}鱼饵）",
                spot.label(),
                rod.label(),
                bait_type.label(),
                bait_quality.label()
            )
        } else {
            "自动钓鱼收竿了".to_string()
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn upgrade_skill(
        &mut self,
        discipline: DisciplineKind,
        skill: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let def = skills::skill_def(discipline, skill).ok_or(GameError::BadSkill)?;
        let current = self.state.skills.get(discipline).skill_level(skill);
        if current >= def.max_level {
            return Err(GameError::SkillMax);
        }
        let cost = skills::skill_cost(current);
        if self.state.skills.get(discipline).points < cost {
            return Err(GameError::NotEnoughSkillPoints { need: cost });
        }
        let d = self.state.skills.get_mut(discipline);
        d.points -= cost;
        d.perks.insert(skill.to_string(), current + 1);
        let noun = if discipline == DisciplineKind::Fishing {
            "钓术"
        } else {
            "专精"
        };
        let text = format!(
            "{}「{}」升到 {} 级（技能点 -{}）",
            noun,
            def.name,
            current + 1,
            cost
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    /// 兼容旧动作：`upgradeFishingSkill` 仍然只加钓鱼的专精。
    fn upgrade_fishing_skill(&mut self, skill: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        self.upgrade_skill(DisciplineKind::Fishing, skill, now)
            .map_err(|err| match err {
                GameError::BadSkill => GameError::BadFishingSkill,
                GameError::SkillMax => GameError::FishingSkillMax,
                other => other,
            })
    }

    fn use_buff_item(&mut self, key: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let def = buffs::buff_item(key).ok_or(GameError::BadBuffItem)?;
        let have = self.state.inventory.get(key).copied().unwrap_or(0);
        if have == 0 {
            return Err(GameError::NotEnoughItems);
        }
        if have == 1 {
            self.state.inventory.remove(key);
        } else {
            self.state.inventory.insert(key.to_string(), have - 1);
        }
        buffs::apply(&mut self.state.buffs, def, now);
        let text = format!("你享用了{}：{}", def.name, def.desc);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn fishing_auto_tick(&mut self, now: i64) -> bool {
        if !self.state.fishing.auto.enabled {
            return false;
        }
        let last_at = self.state.fishing.auto.last_at;
        if last_at < 0 || now <= last_at {
            return false;
        }
        let spot = self.state.fishing.auto.spot;
        let rod = self.state.fishing.auto.rod;
        let bait_quality = self.state.fishing.auto.bait_quality;
        let bait_type = self.state.fishing.auto.bait_type;
        let interval = fishing::auto_interval_ms_with_skill(
            rod,
            self.state.skills.fishing.skill_level("auto_speed"),
        );
        let elapsed = now - last_at;
        let steps = (elapsed.min(FISHING_AUTO_CAP_MS) / interval) as u32;
        if steps == 0 {
            return false;
        }
        let steps = steps.min(FISHING_AUTO_MAX_STEPS);
        let mut caught = 0u32;
        let mut bait_bought = 0u32;
        let mut bait_used = 0u32;
        let mut coins_spent = 0u64;
        let mut report_catches: BTreeMap<String, u32> = BTreeMap::new();
        let mut stop_message: Option<String> = None;
        for step in 0..steps {
            let at = last_at + i64::from(step) * interval;
            let roll = roll_pest((at.max(0) as u64) / 1000, spot as u8, "auto");
            let condition = fishing::FishingCondition::at(&self.state.daily.day, at);
            let Some(def) = fishing::pick_auto_fish_with_bait_condition(
                spot,
                roll,
                &self.state.fishing.codex,
                bait_type,
                condition,
            )
            else {
                self.state.fishing.auto.enabled = false;
                stop_message = Some(format!(
                    "自动钓鱼：{}还没有已点亮的鱼，先手动钓一条",
                    spot.label()
                ));
                break;
            };

            if self.state.fishing.bait_count_of(bait_type, bait_quality) == 0 {
                let cost = bait_quality.price().saturating_mul(10);
                if self.state.coins < cost {
                    self.state.fishing.auto.enabled = false;
                    stop_message = Some(format!(
                        "金币不够自动买{}{}鱼饵，自动钓鱼停了",
                        bait_type.label(),
                        bait_quality.label()
                    ));
                    break;
                }
                self.state.coins -= cost;
                self.state.fishing.add_bait_of(bait_type, bait_quality, 10);
                bait_bought = bait_bought.saturating_add(10);
                coins_spent = coins_spent.saturating_add(cost);
            }
            let saver = fishing::bait_saver_pct(self.state.skills.fishing.skill_level("bait_saver"))
                .saturating_add(rod.bait_save_pct())
                .min(100);
            let saved = saver > 0
                && roll_pest((at.max(0) as u64) ^ 0x51a7, spot as u8, "auto-bait-saver") % 100
                    < u64::from(saver);
            if !saved && !self.state.fishing.take_bait_of(bait_type, bait_quality) {
                break;
            }
            if !saved {
                bait_used = bait_used.saturating_add(1);
            }

            let key = format!("fish:{}", def.id);
            *self.state.inventory.entry(key).or_insert(0) += 1;
            self.state.fishing.stats.auto_catches += 1;
            self.state.fishing.stats.catches += 1;
            let quality_roll = roll_pest((at.max(0) as u64) ^ 0x51ed, spot as u8, "auto-quality");
            let quality = fishing::quality_for_with_bait(rod, bait_quality, quality_roll);
            let weight_tenths =
                fishing::weight_tenths_for_bait(rod, bait_quality, def, roll, quality);
            let market = fishing::market_pct(roll_pest(
                (at.max(0) as u64) ^ 0x9e37,
                spot as u8,
                "auto-market",
            ));
            let golden = roll_pest((at.max(0) as u64) ^ 0x60d1, spot as u8, "auto-golden") % 1000
                < 15 + rod.golden_bonus_permille();
            let mut price = fishing::price_for(def, weight_tenths, quality, market);
            price = fishing::sell_price(price, rod);
            if golden {
                price = price.max(fishing::golden_floor_price(spot));
            }
            self.state
                .fishing
                .fish_batches
                .entry(def.id.to_string())
                .or_default()
                .add(weight_tenths, quality, price);
            self.state
                .fishing
                .record_catch(def, weight_tenths, quality, price, true, at);
            self.state.fishing.quests.on_catch(def, weight_tenths);
            if golden {
                *self
                    .state
                    .fishing
                    .golden_codex
                    .entry(def.id.to_string())
                    .or_insert(0) += 1;
                self.state.fishing.stats.golden_catches += 1;
            }
            for milestone in self.fishing_milestones() {
                self.push_log(at, milestone);
            }
            let exp = fishing::rod_exp(
                fishing::catch_exp(def.rarity, quality, weight_tenths) * 3 / 5,
                rod,
            );
            let pct = buffs::exp_pct(&self.state.buffs, at, DisciplineKind::Fishing);
            let levels = self
                .state
                .skills
                .fishing
                .add_exp(exp * u64::from(100 + pct) / 100);
            for _ in 0..levels {
                self.push_log(
                    at,
                    format!("钓鱼等级升到 {} 级", self.state.skills.fishing.level),
                );
            }
            self.state.fishing.add_rod_wear(rod);
            *report_catches.entry(def.id.to_string()).or_insert(0) += 1;
            caught += 1;
        }
        self.state.fishing.auto.last_at = if elapsed >= FISHING_AUTO_CAP_MS {
            now
        } else {
            last_at + i64::from(steps) * interval
        };
        let mut changed = caught > 0;
        if caught > 0 {
            self.state.daily.fish_catches = self.state.daily.fish_catches.saturating_add(caught);
            self.state.fishing.last_auto_report = Some(fishing::AutoReport {
                at: now,
                bait_bought,
                bait_used,
                coins_spent,
                catches: report_catches,
            });
            let text = format!(
                "自动钓鱼收上来 {caught} 条鱼（用 {bait_used} 份{}鱼饵，买 {bait_bought} 份花 {coins_spent} 金币）",
                bait_quality.label()
            );
            self.push_log(now, text);
            changed = true;
        }
        if let Some(text) = stop_message {
            self.push_log(now, text);
            changed = true;
        }
        changed
    }

    fn pet_pos(&self, pet_id: &str) -> Result<usize, GameError> {
        self.state
            .pets
            .iter()
            .position(|pet| pet.id == pet_id)
            .ok_or(GameError::PetNotFound)
    }

    fn active_pet_count(&self) -> usize {
        self.state.pets.iter().filter(|pet| !pet.boarded).count()
    }

    fn adopt_pet(
        &mut self,
        name: String,
        breed: PetBreed,
        board: bool,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 12 {
            return Err(GameError::InvalidPetName);
        }
        if !board && self.active_pet_count() >= PET_ACTIVE_LIMIT {
            return Err(GameError::PetLimit);
        }
        self.state.pet_seq += 1;
        let id = format!("pet_{}", self.state.pet_seq);
        let mut pet = PetState::new_breed(name.to_string(), breed, now).with_id(id.clone());
        pet.boarded = board;
        if board {
            pet.satiety = 80;
            pet.energy = 80;
            pet.mood = 60;
        }
        self.state.pets.push(pet);
        let text = if board {
            format!(
                "你寄养领养了{}「{}」，宠物院子每月自动扣 {} 金币",
                breed.label(),
                name,
                PET_BOARD_FEE_PER_MONTH
            )
        } else {
            format!("你领养了{}「{}」", breed.label(), name)
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn feed_pet(
        &mut self,
        pet_id: &str,
        fish_id: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded {
            return Err(GameError::PetBoarded);
        }
        if self.state.pets[pos].species != PetSpecies::Cat {
            return Err(GameError::PetFoodWrongSpecies);
        }
        let def = crate::fishing::fish(fish_id).ok_or(GameError::UnknownFish)?;
        let key = format!("fish:{}", def.id);
        let have = *self.state.inventory.get(&key).unwrap_or(&0);
        if have == 0 {
            return Err(GameError::NotEnoughItems);
        }
        if have == 1 {
            self.state.inventory.remove(&key);
        } else {
            self.state.inventory.insert(key, have - 1);
        }

        let active = self.active_pet_count();
        let bowls = self.state.pet_care.bowls;
        let action_seed = self.state.rev;
        let pet = &mut self.state.pets[pos];
        // 猫爱吃鱼；狗也吃，但不加成。
        let preferred = pet.species == PetSpecies::Cat;
        let bowl_ok = bowls >= active as u8;
        let mut fought = false;
        if !bowl_ok && active > 1 {
            let missing = active.saturating_sub(usize::from(bowls));
            let chance = (20 + missing * 20).min(60);
            let roll = roll_pest((now / 60_000) as u64 ^ action_seed, pos as u8, "bowl");
            if roll % 100 < chance as u64 {
                pet.mood = pet.mood.saturating_sub(5);
                pet.bond = pet.bond.saturating_sub(1);
                fought = true;
            }
        }
        let exp = pet.feed(now, preferred, bowl_ok);
        pet.add_exp(exp);
        let text = if fought {
            format!("{}们为食盆打了一架，{}没吃好", pet.name, pet.name)
        } else {
            format!("你喂了{}一条{}，它吃得很香", pet.name, def.name)
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn buy_pet_food(
        &mut self,
        kind: crate::pet::PetFoodKind,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let cost = kind.cost();
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        self.state.pet_care.add_food(kind, 1);
        let text = format!("你买了 1 份{}，花 {cost} 金币", kind.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn feed_pet_food(
        &mut self,
        pet_id: &str,
        kind: crate::pet::PetFoodKind,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded {
            return Err(GameError::PetBoarded);
        }
        if self.state.pets[pos].species != kind.species() {
            return Err(GameError::PetFoodWrongSpecies);
        }
        if !self.state.pet_care.take_food(kind) {
            return Err(GameError::NotEnoughItems);
        }
        let active = self.active_pet_count();
        let bowls = self.state.pet_care.bowls;
        let action_seed = self.state.rev;
        let bowl_ok = bowls >= active as u8;
        let pet = &mut self.state.pets[pos];
        let mut fought = false;
        if !bowl_ok && active > 1 {
            let missing = active.saturating_sub(usize::from(bowls));
            let chance = (20 + missing * 20).min(60);
            let roll = roll_pest((now / 60_000) as u64 ^ action_seed, pos as u8, "bowl");
            if roll % 100 < chance as u64 {
                pet.mood = pet.mood.saturating_sub(5);
                pet.bond = pet.bond.saturating_sub(1);
                fought = true;
            }
        }
        let exp = if kind.is_staple() {
            pet.feed(now, true, bowl_ok)
        } else {
            let (satiety, mood, bond) = kind.snack_gain();
            pet.snack(satiety, mood, bond)
        };
        pet.add_exp(exp);
        let text = if fought {
            format!("{}们为食盆打了一架，{}没吃好", pet.name, pet.name)
        } else if kind.is_staple() {
            format!("你喂了{}一份{}，它吃得很香", pet.name, kind.label())
        } else {
            format!("你给了{}一份{}，它开心地收下了", pet.name, kind.label())
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn touch_pet(&mut self, pet_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded {
            return Err(GameError::PetBoarded);
        }
        let pet = &mut self.state.pets[pos];
        if now - pet.last_pet_at < PET_TOUCH_COOLDOWN_MS {
            return Err(GameError::PetCooldown);
        }
        let exp = pet.touch(now);
        pet.add_exp(exp);
        let text = format!("你摸了摸{}，它蹭了蹭你的手", pet.name);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn play_pet(&mut self, pet_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded {
            return Err(GameError::PetBoarded);
        }
        let pet = &mut self.state.pets[pos];
        if pet.sickness.is_some() {
            return Err(GameError::PetSick);
        }
        if pet.energy < 15 {
            return Err(GameError::PetTooTired);
        }
        if now - pet.last_play_at < PET_PLAY_COOLDOWN_MS {
            return Err(GameError::PetCooldown);
        }
        let toy = match pet.species {
            PetSpecies::Dog => self.state.pet_care.dog_toy && !self.state.pet_care.dog_toy_broken,
            PetSpecies::Cat => self.state.pet_care.cat_toy && !self.state.pet_care.cat_toy_broken,
        };
        let exp = pet.play(now, toy);
        pet.add_exp(exp);
        let text = format!("你和{}玩了一会儿，它开心地转圈", pet.name);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn heal_pet(&mut self, pet_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].sickness.is_none() {
            return Err(GameError::PetHealthy);
        }
        let cost = 500;
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        let pet = &mut self.state.pets[pos];
        pet.heal(now);
        let text = format!("你带{}看了兽医，它恢复了精神（花费 {cost}）", pet.name);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn dig_pet(&mut self, pet_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded {
            return Err(GameError::PetBoarded);
        }
        let pet = &mut self.state.pets[pos];
        if pet.sickness.is_some() {
            return Err(GameError::PetSick);
        }
        if pet.location != PetLocation::Farm {
            return Err(GameError::PetLocationLocked);
        }
        if pet.energy < 20 {
            return Err(GameError::PetTooTired);
        }
        if now - pet.last_dig_at < pet.dig_cooldown_ms() {
            return Err(GameError::PetCooldown);
        }
        let roll = roll_pest((now / 60_000) as u64, 0, &pet.name);
        let pool: Vec<&CropDef> = self
            .content
            .crops
            .iter()
            .filter(|crop| crop.unlock_level <= pet.level + 6)
            .collect();
        if pool.is_empty() {
            return Err(GameError::UnknownCrop);
        }
        let found = pool[(roll as usize) % pool.len()].clone();
        let qty = if pet.level >= 10
            && roll % 100 < 12 + u64::from(pet.dig_bonus()) + u64::from(pet.talent_dig_double())
        {
            2
        } else {
            1
        };
        let exp = pet.dig(now);
        pet.add_exp(exp);
        pet.add_skill_exp(true, 5);
        *self
            .state
            .inventory
            .entry(format!("seed:{}", found.id))
            .or_insert(0) += qty;
        let text = format!("{}在农场刨了刨，挖到了{}种子×{}", pet.name, found.name, qty);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn set_pet_location(
        &mut self,
        pet_id: &str,
        location: PetLocation,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !location.available() {
            return Err(GameError::PetLocationLocked);
        }
        let pos = self.pet_pos(pet_id)?;
        let pet = &mut self.state.pets[pos];
        pet.location = location;
        let text = format!("{}现在待在{}", pet.name, location.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn rename_pet(
        &mut self,
        pet_id: &str,
        name: String,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 12 {
            return Err(GameError::InvalidPetName);
        }
        let pos = self.pet_pos(pet_id)?;
        let pet = &mut self.state.pets[pos];
        pet.name = name.to_string();
        let text = format!("你给宠物改名为「{}」", name);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn set_pet_boarded(
        &mut self,
        pet_id: &str,
        boarded: bool,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded == boarded {
            return Err(GameError::PetCooldown);
        }
        if !boarded && self.active_pet_count() >= PET_ACTIVE_LIMIT {
            return Err(GameError::PetLimit);
        }
        let pet = &mut self.state.pets[pos];
        pet.boarded = boarded;
        if boarded {
            pet.satiety = 80;
            pet.energy = 80;
            pet.mood = 60;
        }
        let text = if boarded {
            format!(
                "{}被送进宠物院子寄养，每月自动扣 {} 金币",
                pet.name, PET_BOARD_FEE_PER_MONTH
            )
        } else {
            format!("你把{}从宠物院子接回来了", pet.name)
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn release_pet(&mut self, pet_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        let pet = self.state.pets.remove(pos);
        let text = if pet.boarded {
            format!("你结束了{}的寄养，把它放生了", pet.name)
        } else {
            format!("你把{}放生了", pet.name)
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn use_pet_skill(&mut self, pet_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded {
            return Err(GameError::PetBoarded);
        }
        let active = self.state.pets[pos].active_skill();
        if self.state.pets[pos].skill_remaining_ms(now) > 0 {
            return Err(GameError::PetSkillCooldown);
        }

        if active.kind == ActiveSkillKind::ClearPest {
            let Some(plot) = self
                .state
                .plots
                .iter()
                .position(|plot| plot.crop.as_ref().is_some_and(|crop| crop.pest.is_some()))
            else {
                return Err(GameError::NoPest);
            };
            self.state.pets[pos].last_skill_at = now;
            let pet_name = self.state.pets[pos].name.clone();
            self.clear_pest(plot as u8 + 1, now)?;
            let text = format!("{}循着动静扑过去，清掉了一块地的草虫", pet_name);
            self.push_log(now, text.clone());
            return Ok(vec![Effect::Log { text }]);
        }

        let pet = &mut self.state.pets[pos];
        let text = match active.kind {
            ActiveSkillKind::Seed => {
                let roll = roll_pest((now / 60_000) as u64, 0, &pet.name);
                let pool: Vec<&CropDef> = self
                    .content
                    .crops
                    .iter()
                    .filter(|crop| crop.unlock_level <= pet.level + 6)
                    .collect();
                let Some(found) = pool.get((roll as usize) % pool.len().max(1)).cloned() else {
                    return Err(GameError::UnknownCrop);
                };
                *self
                    .state
                    .inventory
                    .entry(format!("seed:{}", found.id))
                    .or_insert(0) += 1;
                format!("{}跑了一圈，叼回了{}种子×1", pet.name, found.name)
            }
            ActiveSkillKind::Mood => {
                pet.mood = (pet.mood + active.amount as u32).min(100);
                format!("{}的心情好起来了", pet.name)
            }
            ActiveSkillKind::Energy => {
                pet.energy = (pet.energy + active.amount as u32).min(100);
                format!("{}恢复了一些精力", pet.name)
            }
            ActiveSkillKind::Coins => {
                self.state.coins = self.state.coins.saturating_add(active.amount);
                format!("{}扒拉出了 {} 金币", pet.name, active.amount)
            }
            ActiveSkillKind::GuardExp => {
                pet.add_skill_exp(false, active.amount);
                format!("{}巡逻了一圈，守卫经验 +{}", pet.name, active.amount)
            }
            ActiveSkillKind::ClearPest => unreachable!(),
        };
        pet.last_skill_at = now;
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn train_pet(
        &mut self,
        pet_id: &str,
        kind: PetTrainingKind,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self.state.facilities.training_center {
            return Err(GameError::TrainingLocked);
        }
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].boarded {
            return Err(GameError::PetBoarded);
        }
        if !self.state.pets[pos].can_train(kind) {
            return Err(GameError::TrainingMax);
        }
        let pet = &mut self.state.pets[pos];
        pet.train(kind);
        let text = format!(
            "{}完成了{}训练（训练点 {}/{}）",
            pet.name,
            kind.label(),
            pet.training_spent(),
            pet.training_cap()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn choose_pet_talent(
        &mut self,
        pet_id: &str,
        talent: PetTalent,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        let pet = &mut self.state.pets[pos];
        if talent.species() != pet.species {
            return Err(GameError::PetTalentInvalid);
        }
        if pet.level < talent.level_need() {
            return Err(GameError::PetTalentLocked {
                need: talent.level_need(),
            });
        }
        if !pet.talents.pick(talent) {
            return Ok(Vec::new());
        }
        let text = format!("{}学会了天赋「{}」", pet.name, talent.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn settle_boarding_debt(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.boarding_debt == 0 {
            return Err(GameError::NoBoardingDebt);
        }
        let pay = self.state.coins.min(self.state.boarding_debt);
        self.state.coins -= pay;
        self.state.boarding_debt -= pay;
        let text = if self.state.boarding_debt == 0 {
            format!("你补缴了宠物院子欠费 {pay}，账已结清")
        } else {
            format!(
                "你补缴了宠物院子欠费 {pay}，还欠 {}",
                self.state.boarding_debt
            )
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn buy_ranch_animal(
        &mut self,
        pen: u8,
        kind: RanchAnimalKind,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        self.state.ranch.ensure();
        let target = self.state.ranch.pen(pen).ok_or(GameError::RanchPenEmpty)?;
        if target.animal.is_some() {
            return Err(GameError::RanchPenOccupied);
        }
        let cost = kind.cost();
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        let temper = crate::ranch::roll_trait(now ^ (i64::from(pen) * 7_919), kind.label());
        self.state.ranch.line_seq = self.state.ranch.line_seq.saturating_add(1);
        let line_id = self.state.ranch.line_seq;
        if let Some(target) = self.state.ranch.pen_mut(pen) {
            let mut animal = RanchAnimal::new(kind, now);
            animal.temper = temper;
            animal.line_id = line_id;
            target.animal = Some(animal);
        }
        self.state
            .ranch
            .codex_animals
            .insert(format!("{}:{}", kind.key(), temper.key()));
        let text = format!(
            "你买了一只{}（{}，血统 #{line_id}），住进了 {pen} 号畜栏，花费 {cost}",
            kind.label(),
            temper.label()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn feed_ranch_animal(&mut self, pen: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let animal = self
            .state
            .ranch
            .pen(pen)
            .and_then(|pen| pen.animal.as_ref())
            .ok_or(GameError::RanchPenEmpty)?;
        if animal.sick {
            return Err(GameError::RanchAnimalSick);
        }
        let Some(key) = self
            .state
            .inventory
            .iter()
            .find(|(key, qty)| key.starts_with("crop:") && **qty > 0)
            .map(|(key, _)| key.clone())
        else {
            return Err(GameError::NotEnoughItems);
        };
        let left = self.state.inventory.get(&key).copied().unwrap_or(0);
        let trait_save = animal.temper.feed_save_bonus();
        let feed_save = (ranching::feed_save_chance(
            self.state.skills.ranching.skill_level("ranch_feed"),
        ) + trait_save)
            .min(100);
        let saved = feed_save > 0
            && roll_pest(now.max(0) as u64 ^ 0xfeed, pen, "feed") % 100 < u64::from(feed_save);
        if !saved {
            if left <= 1 {
                self.state.inventory.remove(&key);
            } else {
                self.state.inventory.insert(key, left - 1);
            }
        }
        let care = ranching::care_pct(self.state.skills.ranching.skill_level("ranch_care"));
        let name = if let Some(pen) = self.state.ranch.pen_mut(pen) {
            if let Some(animal) = pen.animal.as_mut() {
                animal.feed_with_care(care);
                animal.name.clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let note = if saved {
            "（省料，这份饲料没消耗）"
        } else {
            ""
        };
        let text = format!("你喂了{name}一份作物，它看起来好多了{note}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn clean_ranch_pen(&mut self, pen: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let today = self.state.daily.day.clone();
        let target = self
            .state
            .ranch
            .pen(pen)
            .ok_or(GameError::RanchPenEmpty)?;
        if target.animal.is_none() {
            return Err(GameError::RanchPenEmpty);
        }
        if !target.can_clean(&today) {
            return Err(GameError::RanchPenCleanDone);
        }
        let (gain, clean) = {
            let target = self
                .state
                .ranch
                .pen_mut(pen)
                .ok_or(GameError::RanchPenEmpty)?;
            let gain = target.clean();
            target.last_clean_day = today;
            (gain, target.cleanliness)
        };
        let text = if gain > 0 {
            format!("你清理了 {pen} 号畜栏，卫生回到 {clean}（+{gain}）")
        } else {
            format!("你清理了 {pen} 号畜栏，本来就很干净（卫生 {clean}）")
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn collect_ranch_product(&mut self, pen: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let (kind, base_quality, name, care_chance, trait_yield) = {
            let pen = self.state.ranch.pen(pen).ok_or(GameError::RanchPenEmpty)?;
            let animal = pen.animal.as_ref().ok_or(GameError::RanchPenEmpty)?;
            (
                animal.kind,
                animal.quality,
                animal.name.clone(),
                animal.quality_chance(pen.cleanliness),
                animal.temper.yield_bonus(),
            )
        };
        let ready = self
            .state
            .ranch
            .pen_mut(pen)
            .and_then(|pen| pen.animal.as_mut())
            .map(|animal| animal.collect(now))
            .unwrap_or(false);
        if !ready {
            return Err(GameError::RanchProductNotReady);
        }
        let product = kind.product_name();
        let quality_level =
            ranching::quality_chance(self.state.skills.ranching.skill_level("ranch_quality"))
                + buffs::ranch_quality_pct(&self.state.buffs, now)
                + crate::npc::ranch_quality_pct(&self.state.npcs);
        let yield_level =
            ranching::yield_chance(self.state.skills.ranching.skill_level("ranch_yield"))
                + trait_yield;
        let mut quality = base_quality;
        if quality < 3 {
            let chance = care_chance + quality_level;
            if chance > 0
                && roll_pest(now.max(0) as u64 ^ 0x7137, pen, kind.product_key()) % 100
                    < u64::from(chance)
            {
                quality = (quality + 1).min(3);
            }
        }
        let special = crate::ranch::special_chance(quality) > 0
            && roll_pest(now.max(0) as u64 ^ 0x5eed, pen, kind.product_key()) % 100
                < u64::from(crate::ranch::special_chance(quality));
        let extra = !special
            && yield_level > 0
            && roll_pest(now.max(0) as u64 ^ 0x9e11, pen, kind.product_key()) % 100
                < u64::from(yield_level);
        let qty = if extra { 2 } else { 1 };
        let key = if special {
            kind.special_key().to_string()
        } else {
            crate::ranch::with_quality(kind.product_key(), quality)
        };
        *self.state.inventory.entry(key).or_insert(0) += qty;
        if special {
            self.state
                .ranch
                .codex_specials
                .insert(kind.special_key().to_string());
        }
        if let Some(pen) = self.state.ranch.pen_mut(pen) {
            pen.get_dirty();
        }
        self.state.daily.ranch_collects = self.state.daily.ranch_collects.saturating_add(1);
        let exp = ranching::collect_exp(quality) + if special { 10 } else { 0 };
        let pct = buffs::exp_pct(&self.state.buffs, now, DisciplineKind::Ranching);
        let levels = self
            .state
            .skills
            .ranching
            .add_exp(exp * u64::from(100 + pct) / 100);
        let text = if special {
            format!(
                "你从{name}那里收了 {}（{}产物，出了特殊产物）",
                kind.special_name(),
                crate::ranch::quality_name(quality)
            )
        } else {
            let mut parts = Vec::new();
            if quality > 1 {
                parts.push(format!(
                    "{}{}",
                    crate::ranch::quality_name(quality),
                    product
                ));
            } else {
                parts.push(product.to_string());
            }
            if extra {
                parts.push("增产+1".into());
            }
            format!("你从{name}那里收了 {}", parts.join("，"))
        };
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        for _ in 0..levels {
            let text = format!("畜牧等级升到 {} 级", self.state.skills.ranching.level);
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
        Ok(effects)
    }

    fn heal_ranch_animal(&mut self, pen: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let animal = self
            .state
            .ranch
            .pen(pen)
            .and_then(|pen| pen.animal.as_ref())
            .ok_or(GameError::RanchPenEmpty)?;
        if !animal.sick {
            return Err(GameError::RanchAnimalHealthy);
        }
        let cost = 300;
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        let care = ranching::care_pct(self.state.skills.ranching.skill_level("ranch_care"));
        let name = if let Some(pen) = self.state.ranch.pen_mut(pen) {
            if let Some(animal) = pen.animal.as_mut() {
                animal.heal_with_care(care);
                animal.name.clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let text = format!("你请兽医看了{name}，花费 {cost}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn expand_ranch(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if !self.state.ranch.can_expand() {
            return Err(GameError::RanchMaxPens);
        }
        let cost = self.state.ranch.next_expand_cost();
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        self.state.ranch.expand();
        let text = format!(
            "你扩建了一个畜栏，花费 {cost}，现在有 {} 个畜栏",
            self.state.ranch.pens.len()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn buy_ranch_feed(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let cost = 300;
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        *self.state.inventory.entry("ranch:feed".into()).or_insert(0) += 1;
        let text = "你买了一份牧场专用饲料".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn feed_ranch_premium(&mut self, pen: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let have = *self.state.inventory.get("ranch:feed").unwrap_or(&0);
        if have == 0 {
            return Err(GameError::RanchNoFeed);
        }
        let animal = self
            .state
            .ranch
            .pen(pen)
            .and_then(|pen| pen.animal.as_ref())
            .ok_or(GameError::RanchPenEmpty)?;
        if animal.sick {
            return Err(GameError::RanchAnimalSick);
        }
        let name = {
            let animal = self
                .state
                .ranch
                .pen_mut(pen)
                .and_then(|pen| pen.animal.as_mut())
                .unwrap();
            animal.feed_premium();
            animal.name.clone()
        };
        if have == 1 {
            self.state.inventory.remove("ranch:feed");
        } else {
            self.state.inventory.insert("ranch:feed".into(), have - 1);
        }
        let text = format!("你给{name}喂了专用饲料，品质提升");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    /// 繁殖条件检查：预览与真繁殖共用一份，避免“能点但生不了”。
    pub(crate) fn ranch_breed_blockers(
        &self,
        a: u8,
        b: u8,
        now: i64,
    ) -> Result<Vec<BreedBlocker>, GameError> {
        let pen_a = self.state.ranch.pen(a).ok_or(GameError::RanchPenEmpty)?;
        let pen_b = self.state.ranch.pen(b).ok_or(GameError::RanchPenEmpty)?;
        let animal_a = pen_a.animal.as_ref().ok_or(GameError::RanchPenEmpty)?;
        let animal_b = pen_b.animal.as_ref().ok_or(GameError::RanchPenEmpty)?;
        if a == b {
            return Ok(vec![BreedBlocker::SamePen]);
        }
        let mut blockers = Vec::new();
        for (pen_no, pen, animal) in [(a, pen_a, animal_a), (b, pen_b, animal_b)] {
            if animal.sick {
                blockers.push(BreedBlocker::Sick {
                    pen: pen_no,
                    name: animal.name.clone(),
                });
            }
            if animal.is_young(now) {
                blockers.push(BreedBlocker::Young {
                    pen: pen_no,
                    name: animal.name.clone(),
                    remaining_ms: animal.young_remaining(now),
                });
            }
            if !animal.breed_ready(now) {
                blockers.push(BreedBlocker::Cooldown {
                    pen: pen_no,
                    name: animal.name.clone(),
                    remaining_ms: animal.breed_remaining(now),
                });
            }
            if animal.health < RANCH_BREED_MIN_HEALTH {
                blockers.push(BreedBlocker::LowHealth {
                    pen: pen_no,
                    name: animal.name.clone(),
                    health: animal.health,
                });
            }
            if pen.cleanliness < RANCH_DIRTY_LINE {
                blockers.push(BreedBlocker::DirtyPen {
                    pen: pen_no,
                    cleanliness: pen.cleanliness,
                });
            }
        }
        if !self
            .state
            .ranch
            .pens
            .iter()
            .any(|pen| pen.animal.is_none())
        {
            blockers.push(BreedBlocker::NoEmptyPen);
        }
        if *self.state.inventory.get("ranch:feed").unwrap_or(&0) == 0 {
            blockers.push(BreedBlocker::NoFeed);
        }
        Ok(blockers)
    }

    fn breed_ranch_pair(&mut self, a: u8, b: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let blockers = self.ranch_breed_blockers(a, b, now)?;
        if let Some(first) = blockers.first() {
            return Err(first.error());
        }
        let target = self
            .state
            .ranch
            .pens
            .iter()
            .position(|pen| pen.animal.is_none())
            .ok_or(GameError::RanchNoEmptyPen)?;
        let plan = {
            let left = self
                .state
                .ranch
                .pen(a)
                .and_then(|pen| pen.animal.as_ref())
                .unwrap();
            let right = self
                .state
                .ranch
                .pen(b)
                .and_then(|pen| pen.animal.as_ref())
                .unwrap();
            crate::ranch::breed_child(left, right, self.state.ranch.breed_seq)
        };
        // 父母进入 72h 冷却。
        for pen_no in [a, b] {
            if let Some(animal) = self
                .state
                .ranch
                .pen_mut(pen_no)
                .and_then(|pen| pen.animal.as_mut())
            {
                animal.last_breed_at = now.max(1);
            }
        }
        let new_line = plan.line_id == 0;
        let line_id = if new_line {
            self.state.ranch.line_seq + 1
        } else {
            plan.line_id
        };
        let mut child = RanchAnimal::new(plan.kind, now);
        child.quality = plan.quality;
        child.generation = plan.generation;
        child.line_id = line_id;
        child.temper = plan.temper;
        child.inbred = plan.inbred;
        child.parents = plan.parents;
        child.adult_at = now + RANCH_YOUNG_MS;
        child.product_ready_at = child.adult_at + child.product_interval();
        child.name = format!("{}幼崽", plan.kind.label());
        self.state.ranch.pens[target].animal = Some(child);
        self.state
            .ranch
            .codex_animals
            .insert(format!("{}:{}", plan.kind.key(), plan.temper.key()));
        if new_line {
            self.state.ranch.line_seq = line_id;
        }
        self.state.ranch.breed_seq = self.state.ranch.breed_seq.saturating_add(1);
        let feed = *self.state.inventory.get("ranch:feed").unwrap_or(&0);
        if feed <= 1 {
            self.state.inventory.remove("ranch:feed");
        } else {
            self.state.inventory.insert("ranch:feed".into(), feed - 1);
        }
        let note = if plan.inbred {
            "（近亲，品质上限受罚）"
        } else {
            ""
        };
        let text = format!(
            "两只动物繁育出{}幼崽（第{}代，血统 #{line_id}）{note}",
            plan.kind.label(),
            plan.generation + 1
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn rename_ranch_animal(
        &mut self,
        pen: u8,
        name: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let name = name.trim();
        let count = name.chars().count();
        if count == 0 || count > 12 {
            return Err(GameError::InvalidRanchName);
        }
        let animal = self
            .state
            .ranch
            .pen_mut(pen)
            .and_then(|pen| pen.animal.as_mut())
            .ok_or(GameError::RanchPenEmpty)?;
        animal.name = name.to_string();
        let text = format!("你给 {pen} 号畜栏的动物改名叫「{name}」");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn accept_ranch_order(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.ranch_orders.find_offer(id).is_none() {
            return Err(GameError::RanchOrderNotFound);
        }
        if self.state.ranch_orders.active.len() >= crate::ranch_orders::RANCH_ORDER_MAX_ACTIVE {
            return Err(GameError::RanchOrderLimit);
        }
        let order = self
            .state
            .ranch_orders
            .accept(id)
            .ok_or(GameError::RanchOrderLimit)?;
        let text = format!(
            "你接了{}的牧场订单：{}{}×{}",
            order.npc_name,
            order.quality_label(),
            order.item_name,
            order.qty
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn deliver_ranch_order(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let order = self
            .state
            .ranch_orders
            .find_active(id)
            .cloned()
            .ok_or(GameError::RanchOrderNotFound)?;
        let key = order.item_key();
        let have = self.state.inventory.get(&key).copied().unwrap_or(0);
        if have < order.qty {
            return Err(GameError::RanchOrderShort);
        }
        if have == order.qty {
            self.state.inventory.remove(&key);
        } else {
            self.state.inventory.insert(key, have - order.qty);
        }
        self.state.ranch_orders.remove_active(id);
        self.state.coins = self.state.coins.saturating_add(order.reward_coins);
        if order.reward_feed > 0 {
            *self.state.inventory.entry("ranch:feed".into()).or_insert(0) += order.reward_feed;
        }
        let pct = buffs::exp_pct(&self.state.buffs, now, DisciplineKind::Ranching);
        let levels = self
            .state
            .skills
            .ranching
            .add_exp(order.reward_exp * u64::from(100 + pct) / 100);
        let text = format!(
            "你交给{} {}{}×{}，收到 {} 金币、{} 牧场经验、{} 份专用饲料",
            order.npc_name,
            order.quality_label(),
            order.item_name,
            order.qty,
            order.reward_coins,
            order.reward_exp,
            order.reward_feed
        );
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        for _ in 0..levels {
            let text = format!("畜牧等级升到 {} 级", self.state.skills.ranching.level);
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
        Ok(effects)
    }

    fn enter_ranch_contest(&mut self, pen: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let week = crate::orders::week_start(&self.state.daily.day);
        self.state.ranch_contest.ensure_week(&week);
        if self.state.ranch_contest.entered {
            return Err(GameError::RanchContestEntered);
        }
        let animal = self
            .state
            .ranch
            .pen(pen)
            .and_then(|pen| pen.animal.as_ref())
            .ok_or(GameError::RanchPenEmpty)?;
        if animal.sick {
            return Err(GameError::RanchAnimalSick);
        }
        if animal.is_young(now) {
            return Err(GameError::RanchAnimalYoung);
        }
        let score = crate::ranch_contest::score(animal);
        let rank = crate::ranch_contest::rank_for(&week, score);
        let (reward_coins, medal) = crate::ranch_contest::reward_for(rank);
        let name = animal.name.clone();
        let kind_label = animal.kind.label().to_string();
        self.state.ranch_contest.entered = true;
        self.state.ranch_contest.medals = self.state.ranch_contest.medals.saturating_add(medal);
        if self.state.ranch_contest.best_rank == 0 || rank < self.state.ranch_contest.best_rank {
            self.state.ranch_contest.best_rank = rank;
        }
        self.state.ranch_contest.result = Some(crate::ranch_contest::RanchContestResult {
            week: week.clone(),
            animal_name: name.clone(),
            kind_label,
            score,
            rank,
            reward_coins,
            medal,
        });
        self.state.coins = self.state.coins.saturating_add(reward_coins);
        let exp = 20 + u64::from(score) / 5;
        let pct = buffs::exp_pct(&self.state.buffs, now, DisciplineKind::Ranching);
        let levels = self
            .state
            .skills
            .ranching
            .add_exp(exp * u64::from(100 + pct) / 100);
        let text = format!(
            "你带{name}参加牧场评比：得分 {score}，第 {rank} 名，拿到 {reward_coins} 金币{}",
            if medal > 0 {
                format!("和 {medal} 枚奖章")
            } else {
                String::new()
            }
        );
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        for _ in 0..levels {
            let text = format!("畜牧等级升到 {} 级", self.state.skills.ranching.level);
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
        Ok(effects)
    }

    fn build_pond(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.pond.built() {
            return Err(GameError::PondExists);
        }
        if self.state.level < crate::pond::POND_BUILD_LEVEL {
            return Err(GameError::LevelTooLow {
                need: crate::pond::POND_BUILD_LEVEL,
            });
        }
        let cost = crate::pond::POND_BUILD_COST;
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        let mut pond = crate::pond::Pond::new(1, now);
        pond.set_season(self.state.season);
        self.state.pond.ponds.push(pond);
        let text = format!("你挖了一口鱼塘，花费 {cost}。钓到的鱼可以放进来养大");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn stock_pond(&mut self, fish_id: &str, qty: u32, now: i64) -> Result<Vec<Effect>, GameError> {
        let def = crate::fishing::fish(fish_id).ok_or(GameError::UnknownFish)?;
        if def.rarity == crate::fishing::FishRarity::King {
            return Err(GameError::PondKingRefused);
        }
        let qty = qty.clamp(1, 5);
        let key = format!("fish:{fish_id}");
        let have = self.state.inventory.get(&key).copied().unwrap_or(0);
        if have < qty {
            return Err(GameError::NotEnoughItems);
        }
        let (total, species, has_species) = {
            let pond = self.state.pond.pond().ok_or(GameError::PondMissing)?;
            (
                pond.total_count(),
                pond.species_count(),
                pond.fish.iter().any(|fish| fish.fish_id == fish_id),
            )
        };
        if total + qty > crate::pond::POND_CAPACITY {
            return Err(GameError::PondFull);
        }
        if !has_species && species >= crate::pond::POND_SPECIES_CAP {
            return Err(GameError::PondSpeciesFull);
        }
        let (weight, quality) = self
            .state
            .fishing
            .fish_batches
            .get(fish_id)
            .map(|batch| {
                (
                    batch.avg_weight_tenths().max(1),
                    batch.avg_quality().clamp(1, 3) as u8,
                )
            })
            .unwrap_or((12, 1));
        if have == qty {
            self.state.inventory.remove(&key);
        } else {
            self.state.inventory.insert(key, have - qty);
        }
        if let Some(pond) = self.state.pond.pond_mut() {
            pond.stock(fish_id, qty, weight, quality, now);
        }
        self.state.pond.note_stock();
        let today = self.state.daily.day.clone();
        if let Some(pond) = self.state.pond.pond().cloned() {
            self.state.pond.ensure_order(&today, &pond);
        }
        let text = format!(
            "你把 {qty} 条{}{}放进鱼塘（平均 {:.1} kg）",
            crate::ranch::quality_name(quality),
            def.name,
            f64::from(weight) / 10.0
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn feed_pond(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let today = self.state.daily.day.clone();
        {
            let pond = self.state.pond.pond_mut().ok_or(GameError::PondMissing)?;
            if pond.last_feed_day != today {
                pond.last_feed_day = today.clone();
                pond.feed_today = 0;
            }
            if pond.feed_today >= crate::pond::POND_FEED_PER_DAY {
                return Err(GameError::PondFeedLimit);
            }
        }
        let Some(key) = self
            .state
            .inventory
            .iter()
            .find(|(key, qty)| key.starts_with("crop:") && **qty > 0)
            .map(|(key, _)| key.clone())
        else {
            return Err(GameError::NotEnoughItems);
        };
        let left = self.state.inventory.get(&key).copied().unwrap_or(0);
        if left <= 1 {
            self.state.inventory.remove(&key);
        } else {
            self.state.inventory.insert(key, left - 1);
        }
        if let Some(pond) = self.state.pond.pond_mut() {
            pond.feed();
            pond.feed_today = pond.feed_today.saturating_add(1);
        }
        let text = "你给鱼塘投喂了一份作物，鱼开始抢食".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn clean_pond(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let today = self.state.daily.day.clone();
        let pond = self.state.pond.pond_mut().ok_or(GameError::PondMissing)?;
        if !today.is_empty() && pond.last_clean_day == today {
            return Err(GameError::PondCleanDone);
        }
        pond.clean();
        pond.last_clean_day = today;
        let text = format!(
            "你清理了鱼塘，水质回到 {}，氨氮降到 {}",
            pond.cleanliness, pond.ammonia
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn install_pond_aerator(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let pond = self.state.pond.pond_mut().ok_or(GameError::PondMissing)?;
        if pond.aerator {
            return Err(GameError::PondUpgradeOwned);
        }
        let cost = crate::pond::POND_AERATOR_COST;
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        if let Some(pond) = self.state.pond.pond_mut() {
            pond.aerator = true;
        }
        let text = format!("你在鱼塘装了增氧机，花费 {cost}，溶氧上限提高");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn plant_pond_weeds(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let pond = self.state.pond.pond_mut().ok_or(GameError::PondMissing)?;
        if pond.plants {
            return Err(GameError::PondUpgradeOwned);
        }
        let cost = crate::pond::POND_PLANTS_COST;
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        if let Some(pond) = self.state.pond.pond_mut() {
            pond.plants = true;
        }
        let text = format!("你在鱼塘种了水草，花费 {cost}，水质更稳");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn harvest_pond(&mut self, fish_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let (count, weight, quality, name) = {
            let pond = self.state.pond.pond().ok_or(GameError::PondMissing)?;
            let fish = pond
                .fish
                .iter()
                .find(|fish| fish.fish_id == fish_id)
                .ok_or(GameError::PondFishNotReady)?;
            let market = crate::pond::market_weight_tenths(fish_id);
            if fish.weight_tenths < market {
                return Err(GameError::PondFishNotReady);
            }
            let def = crate::fishing::fish(fish_id).ok_or(GameError::UnknownFish)?;
            (
                fish.count,
                fish.weight_tenths,
                fish.quality,
                def.name.to_string(),
            )
        };
        let def = crate::fishing::fish(fish_id).ok_or(GameError::UnknownFish)?;
        let price = crate::pond::harvest_price(def, weight, quality);
        if let Some(pond) = self.state.pond.pond_mut() {
            pond.remove_fish(fish_id);
        }
        self.state.pond.note_harvest(count);
        *self
            .state
            .inventory
            .entry(format!("fish:{fish_id}"))
            .or_insert(0) += count;
        for _ in 0..count {
            self.state
                .fishing
                .fish_batches
                .entry(fish_id.to_string())
                .or_default()
                .add(weight, u32::from(quality), price);
        }
        let text = format!(
            "你从鱼塘捕了 {count} 条{name}（平均 {:.1} kg，{}）",
            f64::from(weight) / 10.0,
            crate::ranch::quality_name(quality)
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn toggle_pond_fry(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let pond = self.state.pond.pond_mut().ok_or(GameError::PondMissing)?;
        pond.auto_fry = !pond.auto_fry;
        if pond.auto_fry {
            pond.last_fry_at = now;
        }
        let text = if pond.auto_fry {
            "你留下了亲鱼：每天会自动孵一窝鱼苗（最多 3 条）".to_string()
        } else {
            "你停掉了留种，鱼塘不再自动出鱼苗".to_string()
        };
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn deliver_pond_order(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let order = self
            .state
            .pond
            .order
            .clone()
            .ok_or(GameError::PondOrderMissing)?;
        let can = self
            .state
            .pond
            .pond()
            .map(|pond| order.can_deliver(pond))
            .unwrap_or(false);
        if !can {
            return Err(GameError::PondOrderShort);
        }
        if let Some(pond) = self.state.pond.pond_mut() {
            pond.take_fish(&order.fish_id, order.qty);
        }
        self.state.pond.order = None;
        self.state.pond.order_done = true;
        self.state.coins = self.state.coins.saturating_add(order.reward_coins);
        let text = format!(
            "你把 {} 条{}交给鱼塘订单，收到 {} 金币",
            order.qty, order.fish_name, order.reward_coins
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn ranch_item_count(&self, base: &str) -> u32 {
        (1..=3)
            .map(|quality| {
                self.state
                    .inventory
                    .get(&crate::ranch::with_quality(base, quality))
                    .copied()
                    .unwrap_or(0)
            })
            .sum()
    }

    fn build_workshop(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.workshop.built {
            return Err(GameError::RanchWorkshopOwned);
        }
        if self.state.level < WORKSHOP_LEVEL {
            return Err(GameError::LevelTooLow {
                need: WORKSHOP_LEVEL,
            });
        }
        if self.state.coins < WORKSHOP_COST {
            return Err(GameError::NotEnoughCoins {
                need: WORKSHOP_COST,
            });
        }
        self.state.coins -= WORKSHOP_COST;
        self.state.workshop.built = true;
        self.state.workshop.ensure();
        let today = self.state.daily.day.clone();
        self.state.orders.day.clear();
        self.state.orders.ensure_day(
            &today,
            self.state.level,
            true,
            &self.content,
        );
        let text = format!("你在牧场边搭起一间制造屋，花费 {WORKSHOP_COST}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn expand_workshop(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if !self.state.workshop.can_expand() {
            return Err(GameError::WorkshopMaxSlots);
        }
        let cost = self.state.workshop.next_expand_cost();
        let level = self.state.workshop.next_expand_level();
        if self.state.level < level {
            return Err(GameError::LevelTooLow { need: level });
        }
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        self.state.workshop.expand();
        let text = format!(
            "你给制造屋扩了一个加工槽，花费 {cost}，现在有 {} 个槽",
            self.state.workshop.slot_count()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn install_equipment(
        &mut self,
        slot: u8,
        kind: EquipmentKind,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if kind == EquipmentKind::Manual || slot == 0 || slot >= self.state.workshop.slot_count() {
            return Err(GameError::WorkshopSlotInvalid);
        }
        if self.state.workshop.equipment(slot).is_some() {
            return Err(GameError::WorkshopSlotOccupied);
        }
        if self.state.level < kind.level() {
            return Err(GameError::LevelTooLow { need: kind.level() });
        }
        if self.state.coins < kind.cost() {
            return Err(GameError::NotEnoughCoins { need: kind.cost() });
        }
        self.state.coins -= kind.cost();
        if !self.state.workshop.install(slot, kind) {
            return Err(GameError::WorkshopSlotInvalid);
        }
        let text = format!(
            "你在制造屋 {slot} 号槽装上了{}，花费 {}",
            kind.label(),
            kind.cost()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn upgrade_equipment(&mut self, slot: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if slot == 0 {
            return Err(GameError::WorkshopUpgradeManual);
        }
        if self.state.workshop.equipment(slot).is_none() {
            return Err(GameError::WorkshopEquipmentMissing);
        }
        if self.state.workshop.job(slot).is_some() {
            return Err(GameError::RanchCraftBusy);
        }
        let Some(cost) = self.state.workshop.equipment_upgrade_cost(slot) else {
            return Err(GameError::WorkshopUpgradeMax);
        };
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        let Some(kind) = self.state.workshop.equipment(slot) else {
            return Err(GameError::WorkshopEquipmentMissing);
        };
        self.state.coins -= cost;
        if !self.state.workshop.upgrade_equipment(slot) {
            return Err(GameError::WorkshopUpgradeMax);
        }
        let level = self.state.workshop.equipment_level(slot);
        let speed = self.state.workshop.equipment_speed_pct(slot);
        let yield_pct = self.state.workshop.equipment_yield_pct(slot);
        let text = format!(
            "你把{}升到 {level} 级：加工时间 -{speed}%、取货增产 +{yield_pct}%，花费 {cost}",
            kind.label()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn start_ranch_craft(
        &mut self,
        recipe: RanchRecipe,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if self.state.workshop.job(0).is_some() {
            return Err(GameError::RanchCraftBusy);
        }
        let speed =
            (processing::speed_pct(self.state.skills.processing.skill_level("craft_speed"))
                + buffs::craft_speed_pct(&self.state.buffs, now)
                    + crate::npc::craft_speed_pct(&self.state.npcs))
            .min(40);
        let quality_chance =
            processing::quality_chance(self.state.skills.processing.skill_level("craft_quality"));
        let save_chance =
            processing::save_chance(self.state.skills.processing.skill_level("craft_save"));
        for (base, need) in recipe.inputs() {
            if self.ranch_item_count(base) < *need {
                return Err(GameError::RanchIngredientsShort);
            }
        }
        // 从最低品质开始扣，投入原料里最低的品质决定加工品品质。
        let mut quality = 3u8;
        for (base, need) in recipe.inputs() {
            let mut left = *need;
            for tier in 1..=3u8 {
                let key = crate::ranch::with_quality(base, tier);
                let have = self.state.inventory.get(&key).copied().unwrap_or(0);
                if have == 0 {
                    continue;
                }
                let take = have.min(left);
                if crate::ranch::has_quality(base) {
                    quality = quality.min(tier);
                }
                if have == take {
                    self.state.inventory.remove(&key);
                } else {
                    self.state.inventory.insert(key, have - take);
                }
                left -= take;
                if left == 0 {
                    break;
                }
            }
        }
        let output = recipe.output_name();
        let input_quality = quality;
        if quality_chance > 0
            && roll_pest(now.max(0) as u64 ^ 0x9a11, 0, "craft-quality") % 100
                < u64::from(quality_chance)
        {
            quality = (quality + 1).min(3);
        }
        let saved = save_chance > 0
            && roll_pest(now.max(0) as u64 ^ 0x5a17, 0, "craft-save") % 100
                < u64::from(save_chance);
        if saved {
            if let Some((base, _)) = recipe.inputs().first() {
                *self
                    .state
                    .inventory
                    .entry(crate::ranch::with_quality(base, input_quality))
                    .or_insert(0) += 1;
            }
        }
        let suffix = if quality > 1 {
            crate::ranch::quality_name(quality)
        } else {
            ""
        };
        let duration = recipe.duration_ms() * (100 - i64::from(speed)) / 100;
        if let Some(job) = self.state.workshop.job_mut(0) {
            *job = Some(WorkshopJob::Ranch {
                recipe,
                started_at: now,
                ready_at: now + duration,
                quality,
            });
        }
        let note = if saved {
            "（省料，退回 1 份主料）"
        } else {
            ""
        };
        let text = format!("制造屋开始做{suffix}{output}{note}，过几个小时来取");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn start_crop_craft(&mut self, crop_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if self.state.workshop.job(0).is_some() {
            return Err(GameError::RanchCraftBusy);
        }
        let speed =
            (processing::speed_pct(self.state.skills.processing.skill_level("craft_speed"))
                + buffs::craft_speed_pct(&self.state.buffs, now)
                    + crate::npc::craft_speed_pct(&self.state.npcs))
            .min(40);
        let save_chance =
            processing::save_chance(self.state.skills.processing.skill_level("craft_save"));
        let def = self
            .content
            .crop(crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        let key = format!("crop:{crop_id}");
        let have = self.state.inventory.get(&key).copied().unwrap_or(0);
        if have < CROP_BOX_INPUT {
            return Err(GameError::NotEnoughItems);
        }
        if have == CROP_BOX_INPUT {
            self.state.inventory.remove(&key);
        } else {
            self.state.inventory.insert(key, have - CROP_BOX_INPUT);
        }
        let saved = save_chance > 0
            && roll_pest(now.max(0) as u64 ^ 0x5a17, 0, "craft-save") % 100
                < u64::from(save_chance);
        if saved {
            *self
                .state
                .inventory
                .entry(format!("crop:{crop_id}"))
                .or_insert(0) += 1;
        }
        let duration = CROP_BOX_INTERVAL_MS * (100 - i64::from(speed)) / 100;
        if let Some(job) = self.state.workshop.job_mut(0) {
            *job = Some(WorkshopJob::CropBox {
                crop_id: crop_id.to_string(),
                started_at: now,
                ready_at: now + duration,
            });
        }
        let note = if saved {
            "（省料，退回 1 份）"
        } else {
            ""
        };
        let hours = (duration + 3_599_999) / 3_600_000;
        let text = format!(
            "制造屋开始做{}礼盒（{} 份{}）{note}，{} 小时后取",
            def.name, CROP_BOX_INPUT, def.name, hours
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn start_equipment_craft(
        &mut self,
        recipe: EquipmentRecipe,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        self.start_equipment_craft_for(recipe, now, String::new())
    }

    fn start_equipment_craft_for(
        &mut self,
        recipe: EquipmentRecipe,
        now: i64,
        auto_order: String,
    ) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        let equipment = recipe.equipment();
        let installed = self
            .state
            .workshop
            .equipment
            .iter()
            .flatten()
            .any(|kind| *kind == equipment);
        if !installed {
            return Err(GameError::WorkshopEquipmentMissing);
        }
        let Some(slot) = self.state.workshop.first_idle_slot(equipment) else {
            return Err(GameError::RanchCraftBusy);
        };
        let speed = (processing::speed_pct(
            self.state.skills.processing.skill_level("craft_speed"),
        ) + buffs::craft_speed_pct(&self.state.buffs, now)
                    + crate::npc::craft_speed_pct(&self.state.npcs)
            + self.state.workshop.equipment_speed_pct(slot))
        .min(50);
        let save_chance =
            processing::save_chance(self.state.skills.processing.skill_level("craft_save"));
        for (key, need) in recipe.inputs() {
            if self.workshop_item_count(key) < *need {
                return Err(GameError::RanchIngredientsShort);
            }
        }
        for (key, need) in recipe.inputs() {
            self.consume_workshop_input(key, *need)?;
        }
        let saved = save_chance > 0
            && roll_pest(now.max(0) as u64 ^ 0x5a17, 0, "craft-save") % 100
                < u64::from(save_chance);
        if saved {
            if let Some((key, _)) = recipe.inputs().first() {
                *self.state.inventory.entry((*key).to_string()).or_insert(0) += 1;
            }
        }
        let duration = recipe.duration_ms() * (100 - i64::from(speed)) / 100;
        if let Some(job) = self.state.workshop.job_mut(slot) {
            *job = Some(WorkshopJob::Equipment {
                recipe,
                started_at: now,
                ready_at: now + duration,
                auto_order,
            });
        }
        let note = if saved {
            "（省料，退回 1 份主料）"
        } else {
            ""
        };
        let hours = (duration + 3_599_999) / 3_600_000;
        let text = format!(
            "{}开始做{}{note}，{} 小时后取",
            equipment.label(),
            recipe.label(),
            hours
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn start_fish_craft(
        &mut self,
        recipe: crate::workshop::FishRecipe,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if self.state.workshop.job(0).is_some() {
            return Err(GameError::RanchCraftBusy);
        }
        for (key, need) in recipe.other_inputs() {
            if self.workshop_item_count(key) < *need {
                return Err(GameError::RanchIngredientsShort);
            }
        }
        let save_chance =
            processing::save_chance(self.state.skills.processing.skill_level("craft_save"));
        let saved = recipe.fish_count() > 1
            && save_chance > 0
            && roll_pest(now.max(0) as u64 ^ 0x50a7, 0, "fish-save") % 100 < u64::from(save_chance);
        let fish_needed = recipe
            .fish_count()
            .saturating_sub(if saved { 1 } else { 0 })
            .max(1);
        let mut stock: Vec<(String, u32, u64)> = self
            .state
            .inventory
            .iter()
            .filter_map(|(key, qty)| {
                let id = key.strip_prefix("fish:")?;
                if *qty == 0 || self.state.fishing.locked_fish.contains(id) {
                    return None;
                }
                let def = fishing::fish(id)?;
                Some((id.to_string(), *qty, def.price))
            })
            .collect();
        stock.sort_by_key(|(_, _, price)| *price);
        let available: u32 = stock.iter().map(|(_, qty, _)| *qty).sum();
        if available < fish_needed {
            return Err(GameError::NotEnoughItems);
        }
        let mut left = fish_needed;
        for (id, have, _) in &stock {
            if left == 0 {
                break;
            }
            let take = (*have).min(left);
            let key = format!("fish:{id}");
            if take == *have {
                self.state.inventory.remove(&key);
            } else {
                self.state.inventory.insert(key, *have - take);
            }
            left -= take;
        }
        for (key, need) in recipe.other_inputs() {
            self.consume_workshop_input(key, *need)?;
        }
        let speed =
            (processing::speed_pct(self.state.skills.processing.skill_level("craft_speed"))
                + buffs::craft_speed_pct(&self.state.buffs, now)
                    + crate::npc::craft_speed_pct(&self.state.npcs))
            .min(40);
        let duration = recipe.duration_ms() * (100 - i64::from(speed)) / 100;
        if let Some(job) = self.state.workshop.job_mut(0) {
            *job = Some(WorkshopJob::Fish {
                recipe,
                started_at: now,
                ready_at: now + duration,
            });
        }
        let minutes = (duration + 59_999) / 60_000;
        let note = if saved {
            "（省料少用1条鱼）"
        } else {
            ""
        };
        let text = format!(
            "制造屋开始做{}×{}，用掉 {fish_needed} 条鱼{note}，约 {minutes} 分钟后取",
            recipe.label(),
            recipe.output_qty()
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn start_bait_craft(
        &mut self,
        recipe: fishing::BaitRecipe,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if self.state.workshop.job(0).is_some() {
            return Err(GameError::RanchCraftBusy);
        }
        for (key, need) in recipe.inputs() {
            if self.workshop_item_count(key) < *need {
                return Err(GameError::RanchIngredientsShort);
            }
        }
        for (key, need) in recipe.inputs() {
            self.consume_workshop_input(key, *need)?;
        }
        let speed =
            (processing::speed_pct(self.state.skills.processing.skill_level("craft_speed"))
                + buffs::craft_speed_pct(&self.state.buffs, now)
                    + crate::npc::craft_speed_pct(&self.state.npcs))
            .min(40);
        let duration = recipe.duration_ms() * (100 - i64::from(speed)) / 100;
        if let Some(job) = self.state.workshop.job_mut(0) {
            *job = Some(WorkshopJob::Bait {
                recipe,
                started_at: now,
                ready_at: now + duration,
            });
        }
        let minutes = (duration + 59_999) / 60_000;
        let text = format!(
            "制造屋开始做{}×{}，约 {} 分钟后取",
            recipe.label(),
            recipe.output_qty(),
            minutes
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn workshop_item_count(&self, key: &str) -> u32 {
        if key.starts_with("ranch:") {
            self.ranch_item_count(key)
        } else {
            self.state.inventory.get(key).copied().unwrap_or(0)
        }
    }

    fn consume_workshop_input(&mut self, key: &str, qty: u32) -> Result<(), GameError> {
        if !key.starts_with("ranch:") {
            let have = self.state.inventory.get(key).copied().unwrap_or(0);
            if have < qty {
                return Err(GameError::NotEnoughItems);
            }
            if have == qty {
                self.state.inventory.remove(key);
            } else {
                self.state.inventory.insert(key.to_string(), have - qty);
            }
            return Ok(());
        }
        let mut left = qty;
        for tier in 1..=3u8 {
            let quality_key = crate::ranch::with_quality(key, tier);
            let have = self.state.inventory.get(&quality_key).copied().unwrap_or(0);
            if have == 0 {
                continue;
            }
            let take = have.min(left);
            if have == take {
                self.state.inventory.remove(&quality_key);
            } else {
                self.state.inventory.insert(quality_key, have - take);
            }
            left -= take;
            if left == 0 {
                break;
            }
        }
        if left > 0 {
            return Err(GameError::NotEnoughItems);
        }
        Ok(())
    }

    fn collect_craft(&mut self, slot: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let Some(job) = self.state.workshop.job(slot).cloned() else {
            return Err(GameError::RanchCraftMissing);
        };
        if now < job.ready_at() {
            return Err(GameError::RanchCraftNotReady);
        }
        let auto_meta = if let WorkshopJob::Equipment {
            recipe,
            auto_order,
            ..
        } = &job
        {
            (*auto_order == "workshop").then_some(*recipe)
        } else {
            None
        };
        let quality = job.quality();
        let mut bait_output: Option<(fishing::BaitType, u32)> = None;
        let mut fish_output: Option<(String, u32)> = None;
        let level_yield = if matches!(&job, WorkshopJob::Equipment { .. }) {
            self.state.workshop.equipment_yield_pct(slot)
        } else {
            0
        };
        let (key, label) = match job {
            WorkshopJob::Ranch {
                recipe, quality, ..
            } => {
                let suffix = if quality > 1 {
                    crate::ranch::quality_name(quality)
                } else {
                    ""
                };
                (
                    crate::ranch::with_quality(recipe.output_key(), quality),
                    format!("{suffix}{}", recipe.output_name()),
                )
            }
            WorkshopJob::CropBox { crop_id, .. } => {
                let def = self.content.crop(&crop_id).ok_or(GameError::UnknownCrop)?;
                (crop_box_key(&crop_id), format!("{}礼盒", def.name))
            }
            WorkshopJob::Equipment { recipe, .. } => {
                (recipe.output_key().to_string(), recipe.label().to_string())
            }
            WorkshopJob::Bait { recipe, .. } => {
                bait_output = Some((recipe.bait_type(), recipe.output_qty()));
                (String::new(), recipe.label().to_string())
            }
            WorkshopJob::Fish { recipe, .. } => {
                fish_output = Some((recipe.output_key().to_string(), recipe.output_qty()));
                (String::new(), recipe.label().to_string())
            }
        };
        if let Some(slot_job) = self.state.workshop.job_mut(slot) {
            *slot_job = None;
        }
        let yield_chance = (processing::yield_chance(
            self.state.skills.processing.skill_level("craft_yield"),
        ) + level_yield)
            .min(100);
        let extra = bait_output.is_none()
            && yield_chance > 0
            && roll_pest(now.max(0) as u64 ^ 0x77a1, slot, "craft-yield") % 100
                < u64::from(yield_chance);
        let qty = if let Some((_, bait_qty)) = bait_output {
            bait_qty
        } else if let Some((_, fish_qty)) = fish_output.as_ref() {
            if extra {
                *fish_qty + 1
            } else {
                *fish_qty
            }
        } else if extra {
            2
        } else {
            1
        };
        if let Some((bait_type, _)) = bait_output {
            self.state
                .fishing
                .add_bait_of(bait_type, fishing::BaitQuality::Normal, qty);
        } else if let Some((fish_key, _)) = &fish_output {
            *self.state.inventory.entry(fish_key.clone()).or_insert(0) += qty;
        } else {
            *self.state.inventory.entry(key).or_insert(0) += qty;
        }
        let exp = processing::collect_exp(quality);
        let pct = buffs::exp_pct(&self.state.buffs, now, DisciplineKind::Processing);
        let levels = self
            .state
            .skills
            .processing
            .add_exp(exp * u64::from(100 + pct) / 100);
        let mut report_craft = None;
        let mut report_log = None;
        if let Some(recipe) = auto_meta {
            if let Some(task) = self.state.workshop.auto_task.as_mut() {
                if task.recipe == recipe {
                    task.done = task.done.saturating_add(qty);
                    if task.done >= task.qty {
                        report_craft = Some((recipe.label().to_string(), task.done));
                    }
                }
            }
            if let Some((name, finished)) = report_craft {
                self.state.workshop.auto_task = None;
                self.state.workshop.auto_report =
                    Some(crate::workshop::WorkshopAutoReport {
                        at: now,
                        product_name: name.clone(),
                        qty: finished,
                    });
                let text = format!("制造屋自动做完成：{name}×{finished}");
                self.push_log(now, text.clone());
                report_log = Some(text);
            }
        }
        let note = if extra { "，出品+1" } else { "" };
        let destination = if bait_output.is_some() {
            "鱼饵篓"
        } else {
            "仓库"
        };
        let text = format!("你从制造屋取出了{label}×{qty}{note}，放进了{destination}");
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        if let Some(text) = report_log {
            effects.push(Effect::Log { text });
        }
        for _ in 0..levels {
            let text = format!("加工等级升到 {} 级", self.state.skills.processing.level);
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
        self.advance_auto_orders(now, &mut effects);
        self.advance_workshop_task(now, &mut effects);
        Ok(effects)
    }

    fn ranch_job_tick(&mut self, now: i64, today: &str) -> bool {
        if !self.state.ranch.has_animal() {
            return false;
        }
        let mut rewards: Vec<(String, Option<RanchJobReward>)> = Vec::new();
        for pet in &mut self.state.pets {
            if pet.boarded || pet.location != PetLocation::Ranch || pet.sickness.is_some() {
                continue;
            }
            if pet.last_job_day == today {
                continue;
            }
            pet.last_job_day = today.to_string();
            let before = pet.job.level;
            pet.job.gain(pet.talent_job_exp());
            let level = pet.job.level;
            pet.bond = (pet.bond + 1).min(100);
            pet.mood = (pet.mood + 1).min(100);
            let name = pet.name.clone();
            let roll = job_roll(day_seed(today) as i64, &name);
            let chance = match pet.species {
                PetSpecies::Dog => 50 + (level - 1) * 6 + pet.talent_ranch_chance(),
                PetSpecies::Cat => 20 + (level - 1) * 5,
            };
            if roll % 100 < u64::from(chance) {
                match pet.species {
                    PetSpecies::Dog => {
                        let kinds = [
                            RanchAnimalKind::Chicken,
                            RanchAnimalKind::Duck,
                            RanchAnimalKind::Cow,
                            RanchAnimalKind::Sheep,
                        ];
                        let kind = kinds[((roll >> 8) % 4) as usize];
                        let text = format!("{name}在牧场转了一圈，叼回一份{}", kind.product_name());
                        rewards.push((text, Some(RanchJobReward::Item(kind.product_key()))));
                    }
                    PetSpecies::Cat => {
                        let text = format!("{name}抓了几只老鼠，带回一份专用饲料");
                        rewards.push((text, Some(RanchJobReward::Feed)));
                    }
                }
            } else {
                let base = match pet.species {
                    PetSpecies::Dog => 40,
                    PetSpecies::Cat => 50,
                };
                let coins = base + u64::from(level) * 5;
                let text = match pet.species {
                    PetSpecies::Dog => format!("{name}在牧场守了一天，帮你省下 {coins} 金币"),
                    PetSpecies::Cat => format!("{name}看着饲料槽，帮你省下 {coins} 金币"),
                };
                rewards.push((text, Some(RanchJobReward::Coins(coins))));
            }
            if level > before {
                let text = format!("{name}的牧场岗位升到 {level} 级");
                rewards.push((text, None));
            }
        }
        let changed = !rewards.is_empty();
        for (text, reward) in rewards {
            match reward {
                Some(RanchJobReward::Coins(coins)) => {
                    self.state.coins = self.state.coins.saturating_add(coins);
                    self.state.stats.coins_earned =
                        self.state.stats.coins_earned.saturating_add(coins);
                }
                Some(RanchJobReward::Item(key)) => {
                    *self.state.inventory.entry(key.to_string()).or_insert(0) += 1;
                }
                Some(RanchJobReward::Feed) => {
                    *self.state.inventory.entry("ranch:feed".into()).or_insert(0) += 1;
                }
                None => {}
            }
            self.push_log(now, text);
        }
        changed
    }

    fn fishing_job_tick(&mut self, now: i64, today: &str) -> bool {
        if self.state.pets.is_empty() {
            return false;
        }
        let fishing_level = self.state.skills.fishing.level;
        let pool = fishing::pet_fish_pool(&self.state.fishing, fishing_level, 70);
        let pool_keen = fishing::pet_fish_pool(&self.state.fishing, fishing_level, 120);
        let mut rewards: Vec<(String, Option<FishingJobReward>)> = Vec::new();
        for pet in &mut self.state.pets {
            if pet.boarded || pet.location != PetLocation::Fishing || pet.sickness.is_some() {
                continue;
            }
            if pet.last_job_day == today {
                continue;
            }
            pet.last_job_day = today.to_string();
            let before = pet.fishing_job.level;
            pet.fishing_job.gain(pet.talent_job_exp());
            let job_level = pet.fishing_job.level;
            pet.bond = (pet.bond + 1).min(100);
            pet.mood = (pet.mood + 1).min(100);
            let name = pet.name.clone();
            // 牧场岗位用当天同一个种子；这里加盐，免得同一只宠物在两个岗位摇出同样结果。
            let roll = job_roll(day_seed(today) as i64 ^ 0x4649_5348, &name);
            match pet.species {
                PetSpecies::Dog => {
                    let chance = 50 + (job_level - 1) * 5;
                    if roll % 100 < u64::from(chance) {
                        let kinds = [
                            fishing::BaitType::Dough,
                            fishing::BaitType::Corn,
                            fishing::BaitType::Worm,
                        ];
                        let kind = kinds[((roll >> 8) % 3) as usize];
                        let qty = 1 + ((roll >> 16) % 2) as u32;
                        let text = format!("{name}在钓鱼地刨了刨，叼回 {qty} 份{}", kind.label());
                        rewards.push((text, Some(FishingJobReward::Bait(kind, qty))));
                    } else {
                        let coins = 40 + u64::from(job_level) * 5;
                        let text = format!("{name}在岸边守了一天鱼护，帮你省下 {coins} 金币");
                        rewards.push((text, Some(FishingJobReward::Coins(coins))));
                    }
                }
                PetSpecies::Cat => {
                    let chance = 45 + (job_level - 1) * 6 + pet.talent_fishing_chance();
                    let pool = if pet.talents.has(PetTalent::KeenEye) {
                        &pool_keen
                    } else {
                        &pool
                    };
                    let fish = if roll % 100 < u64::from(chance) {
                        fishing::pick_from_pool(pool, roll >> 8)
                    } else {
                        None
                    };
                    match fish {
                        Some(def) => {
                            let weight = 8 + ((roll >> 24) % 5) as u32;
                            let value = fishing::price_for(def, weight, 1, 100);
                            let text = format!(
                                "{name}在钓鱼地蹲了半天，叼回一条{}（{:.1}kg）",
                                def.name,
                                f64::from(weight) / 10.0
                            );
                            rewards.push((text, Some(FishingJobReward::Fish(def, weight, value))));
                        }
                        None => {
                            let text = format!("{name}在岸边扒拉了半天，带回 2 份面团饵");
                            rewards.push((
                                text,
                                Some(FishingJobReward::Bait(fishing::BaitType::Dough, 2)),
                            ));
                        }
                    }
                }
            }
            if job_level > before {
                let text = format!("{name}的钓鱼地岗位升到 {job_level} 级");
                rewards.push((text, None));
            }
        }
        let changed = !rewards.is_empty();
        for (text, reward) in rewards {
            match reward {
                Some(FishingJobReward::Coins(coins)) => {
                    self.state.coins = self.state.coins.saturating_add(coins);
                    self.state.stats.coins_earned =
                        self.state.stats.coins_earned.saturating_add(coins);
                }
                Some(FishingJobReward::Bait(kind, qty)) => {
                    self.state
                        .fishing
                        .add_bait_of(kind, fishing::BaitQuality::Normal, qty);
                }
                Some(FishingJobReward::Fish(def, weight, value)) => {
                    *self
                        .state
                        .inventory
                        .entry(format!("fish:{}", def.id))
                        .or_insert(0) += 1;
                    self.state
                        .fishing
                        .fish_batches
                        .entry(def.id.to_string())
                        .or_default()
                        .add(weight, 1, value);
                }
                None => {}
            }
            self.push_log(now, text);
        }
        changed
    }

    fn ranch_tick(&mut self, now: i64) -> bool {
        let guarded = self
            .state
            .pets
            .iter()
            .any(|pet| !pet.boarded && pet.location == PetLocation::Ranch);
        let mut changed = false;
        let mut sick = Vec::new();
        for pen in &mut self.state.ranch.pens {
            let pen_clean = pen.cleanliness;
            if let Some(animal) = pen.animal.as_mut() {
                let was_sick = animal.sick;
                animal.tick(now, guarded, pen_clean);
                if !was_sick && animal.sick {
                    sick.push(animal.name.clone());
                }
                changed = true;
            }
        }
        for name in sick {
            let text = format!("{name}生病了，去畜栏看看");
            self.push_log(now, text);
            changed = true;
        }
        changed
    }

    fn buy_pet_supply(&mut self, kind: PetSupplyKind, now: i64) -> Result<Vec<Effect>, GameError> {
        let owned = match kind {
            PetSupplyKind::DogBed => self.state.pet_care.dog_bed,
            PetSupplyKind::CatBed => self.state.pet_care.cat_bed,
            PetSupplyKind::Bowl => self.state.pet_care.bowls >= 3,
            PetSupplyKind::DogToy => self.state.pet_care.dog_toy,
            PetSupplyKind::CatToy => self.state.pet_care.cat_toy,
            PetSupplyKind::Medicine => false,
        };
        if owned {
            return Err(GameError::PetSupplyOwned);
        }
        let cost = kind.cost();
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        match kind {
            PetSupplyKind::DogBed => self.state.pet_care.dog_bed = true,
            PetSupplyKind::CatBed => self.state.pet_care.cat_bed = true,
            PetSupplyKind::Bowl => self.state.pet_care.bowls += 1,
            PetSupplyKind::DogToy => self.state.pet_care.dog_toy = true,
            PetSupplyKind::CatToy => self.state.pet_care.cat_toy = true,
            PetSupplyKind::Medicine => self.state.pet_care.medicine += 1,
        }
        let text = format!("你买了{}，花费 {cost}", kind.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn repair_pet_supply(
        &mut self,
        kind: PetSupplyKind,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let broken = match kind {
            PetSupplyKind::DogBed => self.state.pet_care.dog_bed_broken,
            PetSupplyKind::CatBed => self.state.pet_care.cat_bed_broken,
            PetSupplyKind::DogToy => self.state.pet_care.dog_toy_broken,
            PetSupplyKind::CatToy => self.state.pet_care.cat_toy_broken,
            PetSupplyKind::Bowl | PetSupplyKind::Medicine => false,
        };
        if !broken {
            return Err(GameError::PetSupplyNotBroken);
        }
        let cost = kind.repair_cost();
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        match kind {
            PetSupplyKind::DogBed => self.state.pet_care.dog_bed_broken = false,
            PetSupplyKind::CatBed => self.state.pet_care.cat_bed_broken = false,
            PetSupplyKind::DogToy => self.state.pet_care.dog_toy_broken = false,
            PetSupplyKind::CatToy => self.state.pet_care.cat_toy_broken = false,
            PetSupplyKind::Bowl | PetSupplyKind::Medicine => unreachable!(),
        }
        let text = format!("你修好了{}，花费 {cost}", kind.label());
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn use_pet_medicine(&mut self, pet_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.pet_pos(pet_id)?;
        if self.state.pets[pos].sickness.is_none() {
            return Err(GameError::PetHealthy);
        }
        if self.state.pet_care.medicine == 0 {
            return Err(GameError::NoPetMedicine);
        }
        self.state.pet_care.medicine -= 1;
        let pet = &mut self.state.pets[pos];
        pet.heal(now);
        let text = format!("你给{}用了药，它好多了", pet.name);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn pet_tick(&mut self, now: i64, today: &str) -> bool {
        let dog_bed = self.state.pet_care.dog_bed && !self.state.pet_care.dog_bed_broken;
        let cat_bed = self.state.pet_care.cat_bed && !self.state.pet_care.cat_bed_broken;
        let mut changed = false;
        let mut sick = Vec::new();
        let mut activity_logs = Vec::new();
        for pet in &mut self.state.pets {
            if pet.boarded {
                continue;
            }
            let bed = match pet.species {
                PetSpecies::Dog => dog_bed,
                PetSpecies::Cat => cat_bed,
            };
            let was_sick = pet.sickness.is_some();
            changed |= pet.tick(now, today, bed);
            if !was_sick && pet.sickness.is_some() {
                let kind = pet
                    .sickness
                    .as_ref()
                    .map(|s| s.kind.label())
                    .unwrap_or("生病");
                sick.push((pet.name.clone(), kind.to_string()));
            }
            if pet.last_activity_day != today {
                pet.last_activity_day = today.to_string();
                let activity = pet.activity(now);
                if pet.last_activity == Some(activity) {
                    pet.activity_streak = pet.activity_streak.saturating_add(1);
                } else {
                    pet.activity_streak = 1;
                }
                pet.last_activity = Some(activity);
                let streak = pet.activity_streak;
                if let Some(text) = pet.daily_activity_bonus(activity, streak) {
                    activity_logs.push(format!("{}{}", pet.name, text));
                }
                changed = true;
            }
        }
        for (name, kind) in sick {
            let text = format!("{name}有点{kind}，需要看兽医");
            self.push_log(now, text);
            changed = true;
        }
        for text in activity_logs {
            self.push_log(now, text);
        }
        changed
    }

    fn boarding_charge(&mut self, now: i64, today: &str) -> bool {
        let month = today.get(0..7).unwrap_or(today).to_string();
        if self.state.last_boarding_month == month {
            return false;
        }
        if self.state.last_boarding_month.is_empty() {
            self.state.last_boarding_month = month;
            return false;
        }
        self.state.last_boarding_month = month;
        let boarded = self.state.pets.iter().filter(|pet| pet.boarded).count() as u64;
        if boarded == 0 && self.state.boarding_debt == 0 {
            return false;
        }
        let fee = boarded * PET_BOARD_FEE_PER_MONTH;
        let due = fee.saturating_add(self.state.boarding_debt);
        let paid = self.state.coins.min(due);
        self.state.coins -= paid;
        self.state.boarding_debt = due - paid;
        let text = if self.state.boarding_debt > 0 {
            format!(
                "宠物院子月费 {fee}，支付 {paid} 后仍欠 {}，请尽快补上",
                self.state.boarding_debt
            )
        } else {
            format!("宠物院子自动扣除寄养月费 {fee}")
        };
        self.push_log(now, text);
        true
    }

    fn boarding_settle(&mut self, now: i64) -> bool {
        if self.state.boarding_debt == 0 || self.state.coins == 0 {
            return false;
        }
        let pay = self.state.coins.min(self.state.boarding_debt);
        self.state.coins -= pay;
        self.state.boarding_debt -= pay;
        let text = if self.state.boarding_debt == 0 {
            format!("你补缴了宠物院子欠费 {pay}，账已结清")
        } else {
            format!(
                "你补缴了宠物院子欠费 {pay}，还欠 {}",
                self.state.boarding_debt
            )
        };
        self.push_log(now, text);
        true
    }

    fn pet_mischief(&mut self, now: i64, today: &str) -> bool {
        let mut changed = false;
        let mut total_loss = 0u64;
        let mut messages = Vec::new();
        let mut trample = Vec::new();
        let seed = day_seed(today);
        let available = self.state.coins;
        let mut care = self.state.pet_care;
        let has_crops = self
            .state
            .plots
            .iter()
            .any(|plot| plot.crop.as_ref().is_some_and(|crop| crop.remaining > 1));

        for (index, pet) in self.state.pets.iter_mut().enumerate() {
            if pet.boarded || pet.last_mischief_day == today {
                continue;
            }
            pet.last_mischief_day = today.to_string();
            if pet.sickness.is_some() || pet.mood >= 70 {
                continue;
            }

            let mischief_chance = pet.mischief_chance();
            let roll = roll_pest(seed, index as u8, "mischief");
            if mischief_chance > 0 && roll % 100 < u64::from(mischief_chance) {
                let damaged = match pet.species {
                    PetSpecies::Dog if care.dog_bed && !care.dog_bed_broken => {
                        care.dog_bed_broken = true;
                        Some("狗窝")
                    }
                    PetSpecies::Dog if care.dog_toy && !care.dog_toy_broken => {
                        care.dog_toy_broken = true;
                        Some("狗狗玩具")
                    }
                    PetSpecies::Cat if care.cat_bed && !care.cat_bed_broken => {
                        care.cat_bed_broken = true;
                        Some("猫窝")
                    }
                    PetSpecies::Cat if care.cat_toy && !care.cat_toy_broken => {
                        care.cat_toy_broken = true;
                        Some("猫猫玩具")
                    }
                    _ => None,
                };
                if let Some(item) = damaged {
                    messages.push(format!("{}把{}咬坏了，需要修补", pet.name, item));
                } else {
                    let mut loss = 50 + roll % 150;
                    let left = available.saturating_sub(total_loss);
                    loss = loss.min(left);
                    if loss > 0 {
                        total_loss += loss;
                        messages.push(format!("{}把家里拆了一角，损失 {} 金币", pet.name, loss));
                    } else {
                        messages.push(format!("{}想拆家，被你及时拦住了", pet.name));
                    }
                }
                changed = true;
            }

            let trample_base = 8u32.saturating_sub(pet.training.mischief * 2);
            if has_crops
                && trample_base > 0
                && roll_pest(seed, index as u8, "trample") % 1000 < u64::from(trample_base)
            {
                let amount = if roll.is_multiple_of(2) { 1 } else { 2 };
                trample.push((pet.name.clone(), amount));
                changed = true;
            }
        }

        self.state.pet_care = care;
        if total_loss > 0 {
            self.state.coins = self.state.coins.saturating_sub(total_loss);
        }
        for text in messages {
            self.push_log(now, text);
        }
        for (name, amount) in trample {
            if let Some(pos) = self
                .state
                .plots
                .iter()
                .position(|plot| plot.crop.as_ref().is_some_and(|crop| crop.remaining > 1))
            {
                let mut loss = amount;
                let mut crop_name = String::new();
                if let Some(crop) = self.state.plots[pos].crop.as_mut() {
                    loss = loss.min(crop.remaining.saturating_sub(1));
                    crop.remaining = crop.remaining.saturating_sub(loss);
                    crop_name = self
                        .content
                        .crop(&crop.crop_id)
                        .map(|def| def.name.clone())
                        .unwrap_or_else(|| crop.crop_id.clone());
                }
                if loss > 0 {
                    self.push_log(now, format!("{}踩坏了{}×{}，减产了", name, crop_name, loss));
                }
            }
        }
        changed
    }

    fn npc_raid(&mut self, now: i64, today: &str) -> Option<String> {
        if self.state.last_raid_day == today {
            return None;
        }
        self.state.last_raid_day = today.to_string();
        let roll = roll_pest(day_seed(today), 0, "raid");
        if roll % 100 >= 12 {
            return None;
        }
        let target = self
            .state
            .plots
            .iter()
            .enumerate()
            .find(|(_, plot)| plot.crop.as_ref().is_some_and(|crop| crop.remaining > 1))
            .map(|(pos, _)| pos)?;
        let crop_id = self.state.plots[target].crop.as_ref()?.crop_id.clone();
        let name = self
            .content
            .crop(&crop_id)
            .map(|crop| crop.name.clone())
            .unwrap_or(crop_id);
        let base_loss = 1 + (roll % 2) as u32;
        let mut loss = base_loss;
        let mut guard_text = String::new();
        let best = self
            .state
            .pets
            .iter()
            .filter(|pet| !pet.boarded)
            .max_by_key(|pet| {
                pet.bond
                    + pet.energy
                    + if pet.species == PetSpecies::Dog {
                        30
                    } else {
                        0
                    }
            })
            .cloned();
        if let Some(mut pet) = best {
            let (scared, reduced) = guard_result_full(
                pet.species,
                pet.bond,
                pet.energy,
                pet.sickness.is_some(),
                base_loss,
                roll >> 17,
                pet.guard_bonus(),
                pet.talent_loss_cut(),
            );
            pet.add_skill_exp(false, 5);
            if scared {
                loss = 0;
                guard_text = format!("，{}把偷菜贼吓跑了", pet.name);
            } else if reduced < base_loss {
                loss = reduced;
                guard_text = format!("，{}把损失压到 {} 个", pet.name, loss);
            }
            let signature_save = pet.breed.signature().raid_loss_reduction;
            if loss > 0 && signature_save > 0 {
                loss = loss.saturating_sub(signature_save);
                guard_text = if loss == 0 {
                    format!("，{}守住了田，没让偷菜贼得手", pet.name)
                } else {
                    format!("，{}把损失压到 {} 个", pet.name, loss)
                };
            }
            if scared || loss < base_loss {
                pet.energy = pet.energy.saturating_sub(10);
                pet.mood = (pet.mood + 3).min(100);
                if let Some(slot) = self.state.pets.iter_mut().find(|slot| slot.id == pet.id) {
                    slot.energy = pet.energy;
                    slot.mood = pet.mood;
                    slot.guard_skill = pet.guard_skill;
                    slot.guard_exp = pet.guard_exp;
                }
            }
        }
        if loss > 0 {
            if let Some(crop) = self.state.plots[target].crop.as_mut() {
                let max_loss = crop.remaining.saturating_sub(1);
                loss = loss.min(max_loss);
                crop.remaining = crop.remaining.saturating_sub(loss);
            }
        }
        let text = if loss == 0 && guard_text.is_empty() {
            "偷菜贼摸到田边，没找到可偷的作物".to_string()
        } else if loss == 0 {
            format!("偷菜贼摸到田边{guard_text}")
        } else {
            format!("偷菜贼偷走了{}×{}{}", name, loss, guard_text)
        };
        self.push_log(now, text.clone());
        Some(text)
    }

    fn buy_facility(&mut self, kind: FacilityKind, now: i64) -> Result<Vec<Effect>, GameError> {
        if kind == FacilityKind::Windmill {
            let count = self.state.facilities.windmills;
            if count >= facility_max_count(kind) as u8 {
                return Err(GameError::FacilityMax);
            }
            let need = facility_unlock_level(kind);
            let cost = facility_next_cost(kind, u32::from(count));
            if self.state.level < need {
                return Err(GameError::FacilityLocked { need });
            }
            if self.state.coins < cost {
                return Err(GameError::NotEnoughCoins { need: cost });
            }
            self.state.coins -= cost;
            self.state.facilities.windmills += 1;
            self.state.facilities.windmill = true;
            let text = format!(
                "你建设了第 {} 台风车（+1% 生长），花费 {cost}",
                self.state.facilities.windmills
            );
            self.push_log(now, text.clone());
            return Ok(vec![Effect::Log { text }]);
        }

        if kind == FacilityKind::Greenhouse {
            let count = self.state.facilities.greenhouses;
            if count >= facility_max_count(kind) as u8 {
                return Err(GameError::FacilityMax);
            }
            let need = facility_unlock_level(kind);
            let cost = facility_next_cost(kind, u32::from(count));
            if self.state.level < need {
                return Err(GameError::FacilityLocked { need });
            }
            if self.state.coins < cost {
                return Err(GameError::NotEnoughCoins { need: cost });
            }
            self.state.coins -= cost;
            self.state.facilities.greenhouses += 1;
            self.state.facilities.greenhouse = true;
            self.ensure_greenhouses();
            let text = format!(
                "你建设了第 {} 座温室（3 块地），花费 {cost}",
                self.state.facilities.greenhouses
            );
            self.push_log(now, text.clone());
            return Ok(vec![Effect::Log { text }]);
        }

        let (owned, name, need, cost) = match kind {
            FacilityKind::Sprinkler => (
                self.state.facilities.sprinkler,
                facility_name(kind),
                facility_unlock_level(kind),
                facility_cost(kind),
            ),
            FacilityKind::Harvester => (
                self.state.facilities.harvester,
                facility_name(kind),
                facility_unlock_level(kind),
                facility_cost(kind),
            ),
            FacilityKind::TrainingCenter => (
                self.state.facilities.training_center,
                facility_name(kind),
                facility_unlock_level(kind),
                facility_cost(kind),
            ),
            FacilityKind::Windmill | FacilityKind::Greenhouse => unreachable!(),
        };
        if owned {
            return Err(GameError::FacilityOwned);
        }
        if self.state.level < need {
            return Err(GameError::FacilityLocked { need });
        }
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }

        self.state.coins -= cost;
        match kind {
            FacilityKind::Sprinkler => self.state.facilities.sprinkler = true,
            FacilityKind::Harvester => self.state.facilities.harvester = true,
            FacilityKind::TrainingCenter => self.state.facilities.training_center = true,
            FacilityKind::Windmill | FacilityKind::Greenhouse => unreachable!(),
        }

        let text = format!("你建设了 {name}，花费 {cost}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn repair_windmills(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let damaged = self.state.facilities.damaged_windmills;
        if damaged == 0 {
            return Err(GameError::NoDamagedWindmill);
        }
        let cost = u64::from(damaged) * 3_000;
        if self.state.coins < cost {
            return Err(GameError::NotEnoughCoins { need: cost });
        }
        self.state.coins -= cost;
        self.state.facilities.damaged_windmills = 0;
        let text = format!("你修复了 {damaged} 台风车，花费 {cost}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn damage_windmill(&mut self, now: i64) -> Option<String> {
        if self.state.facilities.windmills == 0
            || self.state.facilities.damaged_windmills >= self.state.facilities.windmills
        {
            return None;
        }
        let roll = roll_pest((now / 60_000) as u64, 0, "windmill");
        if roll % 1000 >= 5 {
            return None;
        }
        self.state.facilities.damaged_windmills += 1;
        Some(format!(
            "气候灾害损坏了 1 台风车（损坏 {}/{}），可维修",
            self.state.facilities.damaged_windmills, self.state.facilities.windmills
        ))
    }

    fn npc_pos(&self, npc_id: &str) -> Result<usize, GameError> {
        self.state
            .npcs
            .iter()
            .position(|npc| npc.id == npc_id)
            .ok_or(GameError::UnknownNpc)
    }

    fn help_npc(
        &mut self,
        npc_id: &str,
        plot_index: u8,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if self.state.daily.helps >= HELP_DAILY_LIMIT {
            return Err(GameError::NpcHelpLimit);
        }
        let pos = self.npc_pos(npc_id)?;
        let npc_name = self.state.npcs[pos].name.clone();
        let crop = self.state.npcs[pos]
            .plots
            .iter()
            .find(|plot| plot.index == plot_index)
            .and_then(|plot| plot.crop.clone())
            .ok_or(GameError::NpcPlotEmpty)?;
        if crop.helped {
            return Err(GameError::NpcAlreadyHelped);
        }
        if crop.ready_at <= now {
            return Err(GameError::AlreadyReady);
        }
        let def = self
            .content
            .crop(&crop.crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;

        if let Some(plot) = self.state.npcs[pos]
            .plots
            .iter_mut()
            .find(|plot| plot.index == plot_index)
        {
            if let Some(c) = plot.crop.as_mut() {
                c.helped = true;
            }
        }
        self.state.npcs[pos].friendship = (self.state.npcs[pos].friendship + 2).min(100);
        self.state.daily.helps += 1;
        self.state.stats.npc_helps += 1;

        let text = format!("你帮 {npc_name} 给 {} 浇了水", def.name);
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        effects.extend(self.add_exp(2, now));
        Ok(effects)
    }

    fn steal_npc(
        &mut self,
        npc_id: &str,
        plot_index: u8,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if self.state.daily.steals >= STEAL_DAILY_LIMIT {
            return Err(GameError::NpcStealLimit);
        }
        let pos = self.npc_pos(npc_id)?;
        let npc_name = self.state.npcs[pos].name.clone();
        let crop = self.state.npcs[pos]
            .plots
            .iter()
            .find(|plot| plot.index == plot_index)
            .and_then(|plot| plot.crop.clone())
            .ok_or(GameError::NpcPlotEmpty)?;
        if crop.ready_at > now {
            return Err(GameError::NpcNotReady);
        }
        if crop.remaining <= 1 {
            return Err(GameError::NpcLeaveSome);
        }
        let def = self
            .content
            .crop(&crop.crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;
        let take = (crop.remaining / 5).max(1).min(crop.remaining - 1);

        if let Some(plot) = self.state.npcs[pos]
            .plots
            .iter_mut()
            .find(|plot| plot.index == plot_index)
        {
            if let Some(c) = plot.crop.as_mut() {
                c.remaining -= take;
            }
        }
        self.state.npcs[pos].friendship = (self.state.npcs[pos].friendship - 2).max(0);
        self.state.daily.steals += 1;
        self.state.stats.npc_steals += 1;
        *self
            .state
            .inventory
            .entry(format!("crop:{}", def.id))
            .or_insert(0) += take;

        let text = format!("你从 {npc_name} 地里摘了 {}×{}", def.name, take);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn help_npc_ranch(
        &mut self,
        npc_id: &str,
        index: u8,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if self.state.daily.ranch_helps >= RANCH_HELP_DAILY_LIMIT {
            return Err(GameError::NpcRanchHelpLimit);
        }
        let pos = self.npc_pos(npc_id)?;
        let npc_name = self.state.npcs[pos].name.clone();
        let animal_pos =
            animal_index(&self.state.npcs[pos].ranch, index).ok_or(GameError::NpcRanchEmpty)?;
        let label = {
            let animal = &mut self.state.npcs[pos].ranch[animal_pos];
            if animal.helped {
                return Err(GameError::NpcRanchAlreadyHelped);
            }
            animal.helped = true;
            animal.kind.label()
        };
        self.state.npcs[pos].friendship = (self.state.npcs[pos].friendship + 2).min(100);
        self.state.daily.ranch_helps += 1;
        self.state.stats.npc_helps += 1;
        let text = format!("你帮 {npc_name} 的{label}添了草料");
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        effects.extend(self.add_exp(2, now));
        Ok(effects)
    }

    fn steal_npc_ranch(
        &mut self,
        npc_id: &str,
        index: u8,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if self.state.daily.ranch_steals >= RANCH_STEAL_DAILY_LIMIT {
            return Err(GameError::NpcRanchStealLimit);
        }
        let pos = self.npc_pos(npc_id)?;
        let npc_name = self.state.npcs[pos].name.clone();
        let animal_pos =
            animal_index(&self.state.npcs[pos].ranch, index).ok_or(GameError::NpcRanchEmpty)?;
        let animal = &mut self.state.npcs[pos].ranch[animal_pos];
        if animal.ready_at > now {
            return Err(GameError::NpcRanchNotReady);
        }
        if animal.remaining == 0 {
            return Err(GameError::NpcRanchNotReady);
        }
        animal.remaining -= 1;
        animal.ready_at = now + animal.kind.product_interval_ms();
        let product = animal.kind.product_key();
        let name = animal.kind.product_name();
        *self.state.inventory.entry(product.to_string()).or_insert(0) += 1;
        self.state.npcs[pos].friendship = (self.state.npcs[pos].friendship - 2).max(0);
        self.state.daily.ranch_steals += 1;
        self.state.stats.npc_steals += 1;
        let text = format!("你从 {npc_name} 的牧场顺走了一份{name}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn npc_greeting(&mut self, npc_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.npc_pos(npc_id)?;
        let npc_name = self.state.npcs[pos].name.clone();
        self.push_npc_message(NpcMessage {
            at: now,
            npc_id: npc_id.into(),
            text: "我来串门啦，你的菜长得真好。".into(),
            from_player: true,
        });
        self.push_npc_message(NpcMessage {
            at: now,
            npc_id: npc_id.into(),
            text: "欢迎常来，下次给你留点种子。".into(),
            from_player: false,
        });
        self.state.npcs[pos].friendship = (self.state.npcs[pos].friendship + 1).min(100);
        self.state.npcs[pos].last_greeting_at = now;

        let text = format!("你给 {npc_name} 留了句话");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn ask_npc_help(&mut self, npc_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.npc_pos(npc_id)?;
        let today = self.state.daily.day.clone();
        if self.state.npcs[pos].last_help_day == today {
            return Err(GameError::NpcHelpDone);
        }
        let npc_name = self.state.npcs[pos].name.clone();
        let text = match npc_id {
            "ahe" => {
                let mut count = 0;
                for plot in &mut self.state.plots {
                    let Some(crop) = plot.crop.as_mut() else {
                        continue;
                    };
                    if crop.ready_at <= now || crop.watered {
                        continue;
                    }
                    let remaining = crop.ready_at - now;
                    crop.ready_at = now + (remaining * 90 / 100).max(1_000);
                    crop.watered = true;
                    count += 1;
                }
                if count == 0 {
                    return Err(GameError::NpcHelpNothing);
                }
                format!("阿禾帮你把 {count} 块地浇了一遍，菜长得更稳了")
            }
            "xiaoman" => {
                buffs::apply(&mut self.state.buffs, &XIAOMAN_HELP_BUFF, now);
                "小满帮你把加工台收拾顺了，今天做东西会快一点".to_string()
            }
            "qingxing" => {
                let mut healed = 0;
                for pen in &mut self.state.ranch.pens {
                    if let Some(animal) = pen.animal.as_mut() {
                        if animal.sick {
                            animal.sick = false;
                            animal.health = 100;
                            healed += 1;
                        }
                    }
                }
                if healed > 0 {
                    format!("青杏帮你看了牲畜，{healed} 只已经缓过来了")
                } else {
                    buffs::apply(&mut self.state.buffs, &QINGXING_HELP_BUFF, now);
                    "青杏帮你把畜栏看了一圈，今天牲畜状态会稳一点".to_string()
                }
            }
            "laotao" => {
                let mut count = 0;
                for slot in 0..self.state.workshop.slots {
                    if let Some(Some(job)) = self.state.workshop.jobs.get_mut(usize::from(slot)) {
                        let ready_at = job_ready_at_mut(job);
                        if *ready_at > now {
                            let remaining = *ready_at - now;
                            *ready_at = now + (remaining * 90 / 100).max(1_000);
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    format!("老陶帮你把 {count} 个加工槽调顺了，正在做的东西快了一点")
                } else {
                    buffs::apply(&mut self.state.buffs, &LAOTAO_HELP_BUFF, now);
                    "老陶帮你把设备擦了一遍，今天加工会顺一点".to_string()
                }
            }
            "liuyue" => {
                let count = self.state.pond.ponds.len();
                for pond in &mut self.state.pond.ponds {
                    pond.clean();
                }
                if count > 0 {
                    format!("六月帮你把 {count} 个鱼塘看了一遍，水稳了")
                } else {
                    buffs::apply(&mut self.state.buffs, &LIUYUE_HELP_BUFF, now);
                    "六月陪你在河边坐了一会儿，今天钓鱼会顺一点".to_string()
                }
            }
            _ => return Err(GameError::NpcHelpNothing),
        };
        self.state.npcs[pos].last_help_day = today;
        let log = format!("{npc_name}搭了把手：{text}");
        self.push_log(now, log.clone());
        Ok(vec![Effect::Log { text: log }])
    }

    fn chat_with_npc(&mut self, npc_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.npc_pos(npc_id)?;
        let today = {
            let day = &self.state.daily.day;
            if day.is_empty() {
                (now / 86_400_000).to_string()
            } else {
                day.clone()
            }
        };
        let day_index = now / 86_400_000;
        let farm_days = if self.state.created_at > 0 {
            ((now - self.state.created_at) / 86_400_000 + 1).max(1) as u32
        } else {
            1
        };
        let owner_name = self.state.owner_name.clone();
        let npc_name = self.state.npcs[pos].name.clone();

        // 有一段故事已经触发但还没选：把选择再端上来，不重复加好感/金币。
        let pending = self.state.npcs[pos].pending_event.clone();
        if !pending.is_empty() {
            if let Some(event) = crate::npc_story::event_by_id(&pending) {
                return Ok(vec![Effect::NpcTalk {
                    npc_id: npc_id.into(),
                    npc_name,
                    title: event.title.to_string(),
                    text: event.text.to_string(),
                    reward: String::new(),
                    favor: 0,
                    tag: String::new(),
                    event_id: event.id.to_string(),
                    choices: npc_choice_views(event.id),
                }]);
            }
            self.state.npcs[pos].pending_event.clear();
        }

        if self.state.npcs[pos].last_chat_day == today {
            let npc = &self.state.npcs[pos];
            let (line, note) = crate::npc_story::line_for(
                &npc.id,
                npc.friendship,
                day_index,
                &today,
                farm_days,
                &owner_name,
            );
            return Ok(vec![Effect::NpcTalk {
                npc_id: npc_id.into(),
                npc_name,
                title: "闲聊".to_string(),
                text: line,
                reward: String::new(),
                favor: 0,
                tag: note.unwrap_or_default(),
                event_id: String::new(),
                choices: Vec::new(),
            }]);
        }

        self.state.npcs[pos].last_chat_day = today.clone();
        self.state.npcs[pos].friendship = (self.state.npcs[pos].friendship + 1).min(100);

        if let Some(event) =
            crate::npc_story::next_event(npc_id, self.state.npcs[pos].friendship, &self.state.npcs[pos].seen_events)
        {
            let npc = &mut self.state.npcs[pos];
            npc.seen_events.push(event.id.to_string());
            npc.pending_event = event.id.to_string();
            npc.memories.push(crate::npc::NpcMemory {
                event_id: event.id.to_string(),
                day: day_index,
                season: crate::npc_story::season_of(&today).to_string(),
                last_recall_day: 0,
            });
            self.push_log(now, format!("{npc_name}想跟你说件事"));
            return Ok(vec![Effect::NpcTalk {
                npc_id: npc_id.into(),
                npc_name,
                title: event.title.to_string(),
                text: event.text.to_string(),
                reward: String::new(),
                favor: 0,
                tag: String::new(),
                event_id: event.id.to_string(),
                choices: npc_choice_views(event.id),
            }]);
        }

        let current_season = crate::npc_story::season_of(&today);
        let recall_id = {
            let npc = &self.state.npcs[pos];
            npc.memories
                .iter()
                .filter(|memory| {
                    memory.season == current_season
                        && day_index - memory.day >= 3
                        && day_index - memory.last_recall_day >= 3
                })
                .filter_map(|memory| {
                    crate::npc_story::event_by_id(&memory.event_id)
                        .map(|event| (memory.event_id.clone(), event.threshold))
                })
                .max_by_key(|(_, threshold)| *threshold)
                .map(|(event_id, _)| event_id)
        };
        if let Some(event_id) = recall_id {
            if let Some(memory) = self.state.npcs[pos]
                .memories
                .iter_mut()
                .find(|memory| memory.event_id == event_id)
            {
                memory.last_recall_day = day_index;
            }
            return Ok(vec![Effect::NpcTalk {
                npc_id: npc_id.into(),
                npc_name,
                title: "旧事".to_string(),
                text: crate::npc_story::recall_line(&event_id).to_string(),
                reward: String::new(),
                favor: 1,
                tag: "记得".to_string(),
                event_id: String::new(),
                choices: Vec::new(),
            }]);
        }

        let npc = &self.state.npcs[pos];
        let (line, note) = crate::npc_story::line_for(
            &npc.id,
            npc.friendship,
            day_index,
            &today,
            farm_days,
            &owner_name,
        );
        self.push_log(now, format!("你和{npc_name}聊了几句"));
        Ok(vec![Effect::NpcTalk {
            npc_id: npc_id.into(),
            npc_name,
            title: "闲聊".to_string(),
            text: line,
            reward: String::new(),
            favor: 1,
            tag: note.unwrap_or_default(),
            event_id: String::new(),
            choices: Vec::new(),
        }])
    }

    fn npc_story_choice(
        &mut self,
        npc_id: &str,
        event_id: &str,
        choice_id: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let pos = self.npc_pos(npc_id)?;
        if self.state.npcs[pos].pending_event != event_id {
            return Err(GameError::NpcChoiceUnavailable);
        }
        let event = crate::npc_story::event_by_id(event_id).ok_or(GameError::NpcChoiceUnavailable)?;
        let choice = crate::npc_story::choices(event_id)
            .iter()
            .find(|choice| choice.id == choice_id)
            .ok_or(GameError::NpcChoiceUnavailable)?;
        let npc = &mut self.state.npcs[pos];
        npc.pending_event.clear();
        npc.choices
            .insert(event_id.to_string(), choice_id.to_string());
        npc.friendship = (npc.friendship + event.favor).min(100);
        let reward = event.reward;
        self.state.coins = self.state.coins.saturating_add(reward);
        let npc_name = npc.name.clone();
        self.push_log(now, format!("{npc_name}跟你说完了「{}」", event.title));
        Ok(vec![Effect::NpcTalk {
            npc_id: npc_id.into(),
            npc_name,
            title: event.title.to_string(),
            text: choice.reply.to_string(),
            reward: format!("{reward} 金币"),
            favor: event.favor,
            tag: "选择".to_string(),
            event_id: String::new(),
            choices: Vec::new(),
        }])
    }

    fn give_npc_birthday_gift(
        &mut self,
        npc_id: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let pos = self.npc_pos(npc_id)?;
        let today = self.state.daily.day.clone();
        let year = crate::npc_story::year_of(&today);
        if !crate::npc_story::is_birthday(npc_id, &today)
            || self.state.npcs[pos].birthday_gift_year == year
        {
            return Err(GameError::NpcBirthdayGiftUnavailable);
        }
        let gift = crate::npc_story::birthday_gift(npc_id)
            .ok_or(GameError::NpcBirthdayGiftUnavailable)?;
        let have = *self.state.inventory.get(gift.item_key).unwrap_or(&0);
        if have < gift.qty {
            return Err(GameError::NotEnoughItems);
        }
        if have == gift.qty {
            self.state.inventory.remove(gift.item_key);
        } else {
            self.state
                .inventory
                .insert(gift.item_key.into(), have - gift.qty);
        }
        self.state.coins = self.state.coins.saturating_add(gift.reward);
        let npc = &mut self.state.npcs[pos];
        npc.birthday_gift_year = year;
        npc.friendship = (npc.friendship + gift.favor).min(100);
        let npc_name = npc.name.clone();
        self.push_log(now, format!("你送了{npc_name}一份生日礼物"));
        Ok(vec![Effect::NpcTalk {
            npc_id: npc_id.into(),
            npc_name,
            title: "生日礼物".to_string(),
            text: gift.reply.to_string(),
            reward: format!("{} 金币", gift.reward),
            favor: gift.favor,
            tag: "生日".to_string(),
            event_id: String::new(),
            choices: Vec::new(),
        }])
    }

    fn give_npc_gift(&mut self, npc_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.npc_pos(npc_id)?;
        let today = self.state.daily.day.clone();
        if crate::npc_story::is_birthday(npc_id, &today)
            || self.state.npcs[pos].last_gift_day == today
        {
            return Err(GameError::NpcGiftUnavailable);
        }
        let gift = crate::npc_story::free_gift(npc_id).ok_or(GameError::NpcGiftUnavailable)?;
        let have = *self.state.inventory.get(gift.item_key).unwrap_or(&0);
        if have < gift.qty {
            return Err(GameError::NotEnoughItems);
        }
        if have == gift.qty {
            self.state.inventory.remove(gift.item_key);
        } else {
            self.state
                .inventory
                .insert(gift.item_key.into(), have - gift.qty);
        }
        let npc = &mut self.state.npcs[pos];
        npc.last_gift_day = today;
        npc.friendship = (npc.friendship + gift.favor).min(100);
        let npc_name = npc.name.clone();
        self.push_log(now, format!("你送了{npc_name}一份小礼物"));
        Ok(vec![Effect::NpcTalk {
            npc_id: npc_id.into(),
            npc_name,
            title: "小礼物".to_string(),
            text: gift.reply.to_string(),
            reward: String::new(),
            favor: gift.favor,
            tag: "礼物".to_string(),
            event_id: String::new(),
            choices: Vec::new(),
        }])
    }

    fn celebrate_festival(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let today = self.state.daily.day.clone();
        let name = crate::npc_story::festival_of(&today).ok_or(GameError::FestivalUnavailable)?;
        let year = crate::npc_story::year_of(&today);
        if self.state.festival_claims.get(name) == Some(&year) {
            return Err(GameError::FestivalUnavailable);
        }
        let reward = crate::npc_story::festival_reward();
        self.state.coins = self.state.coins.saturating_add(reward);
        self.state.festival_claims.insert(name.to_string(), year);
        let text = format!("你参加了{name}，收到 {reward} 金币的小回礼");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn claim_handbook_goal(
        &mut self,
        id: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let goal = crate::handbook::goal(id).ok_or(GameError::HandbookGoalUnavailable)?;
        if !crate::handbook::is_done(goal, &self.state)
            || self.state.handbook_claimed.contains(goal.id)
        {
            return Err(GameError::HandbookGoalUnavailable);
        }
        self.state.handbook_claimed.insert(goal.id.to_string());
        self.state.coins = self.state.coins.saturating_add(goal.coins);
        for (key, qty) in goal.seeds {
            *self.state.inventory.entry((*key).to_string()).or_insert(0) += *qty;
        }
        let text = format!("农场手册：{}，收下 {}", goal.title, goal.reward_text);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn help_npc_request(&mut self, npc_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.npc_pos(npc_id)?;
        let week = now / 86_400_000 / 7;
        let (title, item_key, qty, reward, souvenir) = {
            let npc = &self.state.npcs[pos];
            if npc.request_week == week {
                return Err(GameError::NpcRequestDone);
            }
            let request = crate::npc_story::request(&npc.id).ok_or(GameError::UnknownNpc)?;
            (
                request.title,
                request.item_key,
                request.qty,
                request.reward,
                request.souvenir,
            )
        };
        let have = *self.state.inventory.get(item_key).unwrap_or(&0);
        if have < qty {
            return Err(GameError::NotEnoughItems);
        }
        if have == qty {
            self.state.inventory.remove(item_key);
        } else {
            self.state.inventory.insert(item_key.into(), have - qty);
        }
        self.state.coins = self.state.coins.saturating_add(reward);
        let npc = &mut self.state.npcs[pos];
        npc.request_week = week;
        npc.request_done = true;
        npc.friendship = (npc.friendship + 3).min(100);
        npc.souvenirs.push(souvenir.to_string());
        let text = format!(
            "你帮{npc_name}完成了「{title}」，收到 {reward} 金币和{souvenir}",
            npc_name = npc.name
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn advance_npcs(&mut self, now: i64) -> bool {
        let crops = self.content.crops.clone();
        let mut changed = false;
        for npc_index in 0..self.state.npcs.len() {
            let level = self.state.npcs[npc_index].level;
            let eligible: Vec<CropDef> = crops
                .iter()
                .filter(|crop| crop.unlock_level <= level)
                .cloned()
                .collect();
            if eligible.is_empty() {
                continue;
            }
            let plots = self.state.npcs[npc_index].plots.len();
            for plot_index in 0..plots {
                let should_replant =
                    match self.state.npcs[npc_index].plots[plot_index].crop.as_ref() {
                        Some(crop) => now >= crop.ready_at + 45 * 60_000,
                        None => true,
                    };
                if !should_replant {
                    continue;
                }
                let def = &eligible[(npc_index + plot_index) % eligible.len()];
                self.state.npcs[npc_index].plots[plot_index].crop = Some(NpcCrop {
                    crop_id: def.id.clone(),
                    planted_at: now,
                    ready_at: now + i64::from(def.grow_minutes) * 60_000,
                    remaining: def.yield_per_season,
                    helped: false,
                });
                changed = true;
            }
        }
        changed
    }

    fn npc_visits(&mut self, now: i64) -> Vec<Effect> {
        let gap = now - self.state.last_npc_tick_at;
        if gap < NPC_VISIT_GAP_MS {
            return Vec::new();
        }
        self.state.last_npc_tick_at = now;
        if self.state.npcs.is_empty() {
            return Vec::new();
        }
        let count = ((gap / NPC_VISIT_GAP_MS) as usize).clamp(1, NPC_VISIT_CAP);
        let mut effects = Vec::new();
        for i in 0..count {
            let index =
                (roll_pest((now / 60_000) as u64, i as u8, "npc") as usize) % self.state.npcs.len();
            let name = self.state.npcs[index].name.clone();
            let npc_id = self.state.npcs[index].id.clone();
            let text = if i % 2 == 0 {
                format!("{name} 来你的农场转了一圈，说菜长得不错。")
            } else {
                format!("{name} 路过，给你留了一句问候。")
            };
            self.push_npc_message(NpcMessage {
                at: now,
                npc_id,
                text: text.clone(),
                from_player: false,
            });
            self.state.stats.npc_visits += 1;
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
        effects
    }

    fn push_npc_message(&mut self, message: NpcMessage) {
        self.state.npc_messages.push(message);
        if self.state.npc_messages.len() > 30 {
            self.state.npc_messages.remove(0);
        }
    }

    fn dismiss_guide(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        self.state.settings.onboarded = true;
        let text = "新手引导已关闭".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn show_guide(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        self.state.settings.onboarded = false;
        let text = "新手引导已重新显示".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn update_settings(&mut self, exp_rate: u32, now: i64) -> Result<Vec<Effect>, GameError> {
        if ![1, 2, 5].contains(&exp_rate) {
            return Err(GameError::InvalidExpRate);
        }
        self.state.settings.exp_rate = exp_rate;
        let text = format!("经验倍率设为 {exp_rate}×");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn clear_pest(&mut self, index: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let pos = self.plot_pos(index)?;
        let crop = self.state.plots[pos]
            .crop
            .clone()
            .ok_or(GameError::PlotEmpty)?;
        let pest = crop.pest.clone().ok_or(GameError::NoPest)?;
        let def = self
            .content
            .crop(&crop.crop_id)
            .cloned()
            .ok_or(GameError::UnknownCrop)?;

        if let Some(c) = self.state.plots[pos].crop.as_mut() {
            c.pest = None;
        }
        self.state.stats.pests_cleared += 1;
        self.state.daily.pests_cleared += 1;

        let text = format!("你清理了 {} 的{}", def.name, pest.kind.label());
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        effects.extend(self.add_exp(2, now));
        Ok(effects)
    }

    fn claim_sign_in(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.daily.sign_in {
            return Err(GameError::AlreadySignedIn);
        }
        self.state.daily.sign_in = true;
        self.state.stats.sign_ins += 1;
        self.state.coins = self.state.coins.saturating_add(20);
        *self
            .state
            .inventory
            .entry("seed:xiaobaicai".into())
            .or_insert(0) += 2;

        let text = "签到成功：金币+20，小青菜种子+2".to_string();
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        effects.extend(self.add_exp(10, now));
        Ok(effects)
    }

    fn claim_daily(&mut self, id: u8, now: i64) -> Result<Vec<Effect>, GameError> {
        let task = DAILY_TASKS
            .iter()
            .find(|task| task.id == id)
            .ok_or(GameError::BadDailyTask)?;
        if crate::daily::daily_claimed(&self.state.daily, id) {
            return Err(GameError::AlreadyClaimed);
        }
        if daily_progress(&self.state.daily, id) < task.target {
            return Err(GameError::DailyNotDone);
        }

        mark_claimed(&mut self.state.daily, id);
        self.state.stats.daily_claimed += 1;
        self.state.coins = self.state.coins.saturating_add(task.reward_coins);
        let text = format!("领取日常「{}」：金币+{}", task.name, task.reward_coins);
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        effects.extend(self.add_exp(task.reward_exp, now));
        Ok(effects)
    }

    fn update_profile(&mut self, owner_name: String, now: i64) -> Result<Vec<Effect>, GameError> {
        let name = owner_name.trim();
        if name.is_empty() || name.chars().count() > 12 {
            return Err(GameError::BadPlayerName);
        }
        self.state.owner_name = name.to_string();
        let text = format!("玩家昵称改为「{name}」");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn accept_farm_order(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {        let Some(order) = self.state.orders.find_offer(id).cloned() else {
            return Err(GameError::FarmOrderInvalid);
        };
        if order.expires_day.as_str() < self.state.daily.day.as_str() {
            return Err(GameError::FarmOrderExpired);
        }
        if self.state.orders.active.len() >= crate::orders::FARM_ORDER_MAX_ACTIVE {
            return Err(GameError::FarmOrderLimit);
        }
        if !self.state.orders.accept(id) {
            return Err(GameError::FarmOrderInvalid);
        }
        let text = format!(
            "接下 {} 的订单：{}×{}，{} 前交付",
            order.npc_name,
            order.item_name_or_crop(),
            order.qty,
            order.expires_day
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn deliver_farm_order(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let Some(order) = self.state.orders.find_active(id).cloned() else {
            return Err(GameError::FarmOrderInvalid);
        };
        if order.expires_day.as_str() < self.state.daily.day.as_str() {
            return Err(GameError::FarmOrderExpired);
        }
        let key = order.delivery_key();
        let have = self.state.inventory.get(&key).copied().unwrap_or(0);
        if have < order.qty {
            return Err(GameError::FarmOrderNotEnough);
        }
        if order.fresh
            && self.state.crop_fresh_day.get(&order.crop_id) != Some(&self.state.daily.day)
        {
            return Err(GameError::FarmOrderNotFresh);
        }
        if order.min_quality > 0 {
            let ok = self
                .state
                .crop_quality
                .get(&order.crop_id)
                .map(|mark| {
                    mark.day == self.state.daily.day && mark.quality >= order.min_quality
                })
                .unwrap_or(false);
            if !ok {
                return Err(GameError::FarmOrderNotQuality);
            }
        }
        if have == order.qty {
            self.state.inventory.remove(&key);
        } else {
            self.state.inventory.insert(key, have - order.qty);
        }
        self.state.coins = self.state.coins.saturating_add(order.reward_coins);
        self.state.daily.orders = self.state.daily.orders.saturating_add(1);
        self.state.stats.orders_delivered = self.state.stats.orders_delivered.saturating_add(1);
        if let Some(npc) = self
            .state
            .npcs
            .iter_mut()
            .find(|npc| npc.id == order.npc_id)
        {
            npc.friendship = (npc.friendship + 2).min(100);
        }
        self.state.orders.remove_active(id);
        let text = format!(
            "交付 {} 的订单：{}×{}，金币+{}，{} 友好度+2",
            order.npc_name,
            order.item_name_or_crop(),
            order.qty,
            order.reward_coins,
            order.npc_name
        );
        self.push_log(now, text.clone());
        let mut effects = vec![Effect::Log { text }];
        effects.extend(self.add_exp(order.reward_exp as u32, now));
        Ok(effects)
    }

    fn start_order_craft(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let Some(order) = self.state.orders.find_active(id).cloned() else {
            return Err(GameError::FarmOrderInvalid);
        };
        if order.expires_day.as_str() < self.state.daily.day.as_str() {
            return Err(GameError::FarmOrderExpired);
        }
        let recipe = order.recipe.ok_or(GameError::FarmOrderInvalid)?;
        self.start_equipment_craft(recipe, now)
    }

    fn auto_craft_order(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let Some(order) = self.state.orders.find_active(id).cloned() else {
            return Err(GameError::FarmOrderInvalid);
        };
        if order.expires_day.as_str() < self.state.daily.day.as_str() {
            return Err(GameError::FarmOrderExpired);
        }
        if self.state.inventory.get(&order.delivery_key()).copied().unwrap_or(0) >= order.qty {
            return Ok(vec![Effect::Log {
                text: "仓库里已经有成品了，直接去交付吧".into(),
            }]);
        }
        let recipe = order.recipe.ok_or(GameError::FarmOrderInvalid)?;
        let mut started = Vec::new();
        self.plan_recipe(recipe, now, 0, id, &mut started)?;
        if started.is_empty() {
            let text = "自动排程没找到能开工的步骤：先看看缺原料还是缺设备".to_string();
            self.push_log(now, text.clone());
            return Ok(vec![Effect::Log { text }]);
        }
        if let Some(active) = self
            .state
            .orders
            .active
            .iter_mut()
            .find(|active| active.id == id)
        {
            active.auto = true;
        }
        let text = format!(
            "{}的{}订单开始自动排程：先做{}",
            order.npc_name,
            order.item_name_or_crop(),
            started.join("、")
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn plan_recipe(
        &mut self,
        recipe: EquipmentRecipe,
        now: i64,
        depth: u8,
        order_id: &str,
        started: &mut Vec<String>,
    ) -> Result<bool, GameError> {
        if depth > 4 || self.equipment_job_active(recipe) {
            return Ok(false);
        }
        let mut missing_craft = Vec::new();
        let mut missing_raw = false;
        for (key, need) in recipe.inputs() {
            if self.workshop_item_count(key) >= *need {
                continue;
            }
            if key.starts_with("craft:") {
                missing_craft.push(*key);
            } else {
                missing_raw = true;
            }
        }
        if missing_craft.is_empty() && !missing_raw {
            if self
                .state
                .workshop
                .first_idle_slot(recipe.equipment())
                .is_some()
            {
                self.start_equipment_craft_for(recipe, now, order_id.to_string())?;
                started.push(recipe.label().to_string());
                return Ok(true);
            }
            return Ok(false);
        }
        let mut any = false;
        for key in missing_craft {
            let Some(sub) = EquipmentRecipe::ALL
                .iter()
                .copied()
                .find(|candidate| candidate.output_key() == key)
            else {
                continue;
            };
            if self.plan_recipe(sub, now, depth + 1, order_id, started)? {
                any = true;
            }
        }
        Ok(any)
    }

    fn equipment_job_active(&self, recipe: EquipmentRecipe) -> bool {
        (0..self.state.workshop.slots).any(|slot| {
            matches!(
                self.state.workshop.job(slot),
                Some(WorkshopJob::Equipment {
                    recipe: active,
                    ..
                }) if *active == recipe
            )
        })
    }

    fn advance_auto_orders(&mut self, now: i64, effects: &mut Vec<Effect>) {
        let ids: Vec<String> = self
            .state
            .orders
            .active
            .iter()
            .filter(|order| order.auto && order.recipe.is_some())
            .map(|order| order.id.clone())
            .collect();
        for id in ids {
            let Some(order) = self
                .state
                .orders
                .active
                .iter()
                .find(|order| order.id == id)
                .cloned()
            else {
                continue;
            };
            if self
                .state
                .inventory
                .get(&order.delivery_key())
                .copied()
                .unwrap_or(0)
                >= order.qty
            {
                if let Some(active) = self
                    .state
                    .orders
                    .active
                    .iter_mut()
                    .find(|active| active.id == id)
                {
                    active.auto = false;
                }
                continue;
            }
            let Some(recipe) = order.recipe else {
                continue;
            };
            let mut started = Vec::new();
            if self.plan_recipe(recipe, now, 0, &id, &mut started).is_ok() && !started.is_empty() {
                let text = format!(
                    "自动排程继续：{} → {}",
                    order.item_name_or_crop(),
                    started.join("、")
                );
                self.push_log(now, text.clone());
                effects.push(Effect::Log { text });
            }
        }
    }

    fn auto_craft_workshop(
        &mut self,
        recipe: EquipmentRecipe,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        if !self.state.workshop.built {
            return Err(GameError::RanchWorkshopMissing);
        }
        if self.state.workshop.auto_task.is_some() {
            return Err(GameError::WorkshopAutoBusy);
        }
        self.state.workshop.auto_task = Some(crate::workshop::WorkshopAutoTask {
            recipe,
            qty: 1,
            done: 0,
            started_at: now,
        });
        let mut started = Vec::new();
        self.plan_recipe(recipe, now, 0, "workshop", &mut started)?;
        if started.is_empty() {
            self.state.workshop.auto_task = None;
            let text = "自动做没找到能开工的步骤：先看看缺原料还是缺设备".to_string();
            self.push_log(now, text.clone());
            return Ok(vec![Effect::Log { text }]);
        }
        let text = format!("制造屋开始自动做{}：{}", recipe.label(), started.join("、"));
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn cancel_workshop_auto(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.workshop.auto_task.take().is_none() {
            return Ok(vec![]);
        }
        let text = "你取消了制造屋的自动做任务".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn advance_workshop_task(&mut self, now: i64, effects: &mut Vec<Effect>) {
        let Some(task) = self.state.workshop.auto_task.clone() else {
            return;
        };
        if task.done >= task.qty {
            return;
        }
        let mut started = Vec::new();
        if self
            .plan_recipe(task.recipe, now, 0, "workshop", &mut started)
            .is_ok()
            && !started.is_empty()
        {
            let text = format!(
                "制造屋自动做继续：{} → {}",
                task.recipe.label(),
                started.join("、")
            );
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
        }
    }

    fn use_farm_card(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        let key = "craft:farm_card";
        let have = self.state.inventory.get(key).copied().unwrap_or(0);
        if have == 0 {
            return Err(GameError::FarmCardMissing);
        }
        if have == 1 {
            self.state.inventory.remove(key);
        } else {
            self.state.inventory.insert(key.to_string(), have - 1);
        }

        let indices: Vec<u8> = self
            .state
            .plots
            .iter()
            .filter(|plot| plot.unlocked)
            .map(|plot| plot.index)
            .collect();
        let mut pests = 0u32;
        let mut watered = 0u32;
        let mut harvested = 0u32;
        let mut planted = 0u32;
        let mut harvested_crops: Vec<(u8, String)> = Vec::new();

        for index in &indices {
            let pos = self.plot_pos(*index)?;
            let has_pest = self.state.plots[pos]
                .crop
                .as_ref()
                .is_some_and(|crop| crop.pest.is_some());
            if has_pest && self.clear_pest(*index, now).is_ok() {
                pests += 1;
            }
            if self.state.plots[pos]
                .crop
                .as_ref()
                .is_some_and(|crop| crop.withered)
            {
                let _ = self.clear(*index, now);
                continue;
            }
            if self.water(*index, now).is_ok() {
                watered += 1;
            }
            let crop_id = self.state.plots[pos]
                .crop
                .as_ref()
                .map(|crop| crop.crop_id.clone());
            if let Some(crop_id) = crop_id {
                if self.harvest(*index, now).is_ok() {
                    harvested += 1;
                    harvested_crops.push((*index, crop_id));
                }
            }
        }

        for index in indices {
            if self.state.plots[self.plot_pos(index)?].crop.is_some() {
                continue;
            }
            let preferred = harvested_crops
                .iter()
                .find(|(plot, _)| *plot == index)
                .map(|(_, crop_id)| crop_id.clone());
            let seed = preferred
                .filter(|crop_id| {
                    self.state
                        .inventory
                        .get(&format!("seed:{crop_id}"))
                        .copied()
                        .unwrap_or(0)
                        > 0
                })
                .or_else(|| self.best_normal_seed());
            if let Some(crop_id) = seed {
                if self.plant(index, &crop_id, now).is_ok() {
                    planted += 1;
                }
            }
        }

        let text = format!(
            "农场一键卡：除虫 {pests} 次，浇水 {watered} 次，收获 {harvested} 次，补种 {planted} 块"
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn best_normal_seed(&self) -> Option<String> {
        let mut best: Option<&crate::content::CropDef> = None;
        for crop in &self.content.crops {
            if crop.unlock_level > self.state.level {
                continue;
            }
            let have = self
                .state
                .inventory
                .get(&format!("seed:{}", crop.id))
                .copied()
                .unwrap_or(0);
            if have == 0 {
                continue;
            }
            let better = best.is_none_or(|current| {
                crop.unlock_level > current.unlock_level
                    || (crop.unlock_level == current.unlock_level
                        && crop.grow_minutes < current.grow_minutes)
            });
            if better {
                best = Some(crop);
            }
        }
        best.map(|crop| crop.id.clone())
    }

    fn buy_mystery_offer(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let Some(offer) = self.state.mystery.offer(id).cloned() else {
            return Err(GameError::MysteryOfferInvalid);
        };
        if offer.sold {
            return Err(GameError::MysteryOfferSold);
        }
        if self.state.coins < offer.price {
            return Err(GameError::NotEnoughCoins { need: offer.price });
        }
        self.state.coins -= offer.price;
        if !self.state.mystery.mark_sold(id) {
            return Err(GameError::MysteryOfferSold);
        }
        *self
            .state
            .inventory
            .entry(offer.item_key.clone())
            .or_insert(0) += 1;
        if offer.kind == crate::MysteryKind::Furniture {
            self.state.inventory.remove(&offer.item_key);
            if let Some(def) = offer.item_key.strip_prefix("furniture:") {
                self.state.room.ensure_ownership();
                *self
                    .state
                    .room
                    .owned_furniture
                    .entry(def.to_string())
                    .or_insert(0) += 1;
            }
        }
        let text = format!(
            "在神秘商店买下 {}：金币-{}",
            offer.name, offer.price
        );
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn claim_level_reward(&mut self, level: u32, now: i64) -> Result<Vec<Effect>, GameError> {
        if !(LEVEL_REWARD_MIN..=MAX_LEVEL).contains(&level) {
            return Err(GameError::BadLevelReward);
        }
        if self.state.level < level {
            return Err(GameError::LevelTooLow { need: level });
        }
        if self.state.level_rewards_claimed.contains(&level) {
            return Err(GameError::AlreadyClaimed);
        }

        self.state.level_rewards_claimed.insert(level);
        let coins = level_reward::coins_for(level);
        self.state.coins = self.state.coins.saturating_add(coins);
        let mut parts = vec![format!("金币+{coins}")];

        let unlocked_crops: Vec<(String, String)> = self
            .content
            .crops
            .iter()
            .filter(|crop| crop.unlock_level == level)
            .map(|crop| (crop.id.clone(), crop.name.clone()))
            .collect();
        for (crop_id, name) in unlocked_crops {
            *self
                .state
                .inventory
                .entry(format!("seed:{crop_id}"))
                .or_insert(0) += level_reward::NEW_CROP_SEEDS;
            parts.push(format!("{name}种子×{}", level_reward::NEW_CROP_SEEDS));
        }
        for (key, qty) in level_reward::milestone_items(level) {
            *self.state.inventory.entry((*key).to_string()).or_insert(0) += *qty;
            let name = crate::ranch::item_def(key)
                .map(|def| def.name)
                .unwrap_or(key);
            parts.push(format!("{name}×{qty}"));
        }
        for id in level_reward::milestone_furniture(level) {
            if let Some(def) = room::furniture_def(id) {
                self.state.room.ensure_ownership();
                *self
                    .state
                    .room
                    .owned_furniture
                    .entry((*id).to_string())
                    .or_insert(0) += 1;
                parts.push(format!("{}×1", def.name));
            }
        }

        let text = format!("领取 {level} 级奖励：{}", parts.join("、"));
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    pub fn sync(&mut self, now: i64, today: &str) -> Outcome {
        let mut effects = Vec::new();
        let mut changed = false;

        let auto_ready: Vec<u8> = (0..self.state.workshop.slots)
            .filter(|slot| {
                matches!(
                    self.state.workshop.job(*slot),
                    Some(WorkshopJob::Equipment {
                        ready_at,
                        auto_order,
                        ..
                    }) if !auto_order.is_empty() && now >= *ready_at
                )
            })
            .collect();
        for slot in auto_ready {
            if let Ok(mut extra) = self.collect_craft(slot, now) {
                effects.append(&mut extra);
                changed = true;
            }
        }

        self.state.room.ensure_defaults();
        if self.state.room.ensure_decor_shop(today) {
            changed = true;
        }
        // 老档已经领过 10/20/30 级奖励的，补发当时还没有的绑定家具。
        for (level, id) in [(10, "gift_quilt"), (20, "gift_harvest_rug"), (30, "gift_anniversary_lamp")] {
            if self.state.level >= level
                && self.state.level_rewards_claimed.contains(&level)
                && !self.state.room.owned_furniture.contains_key(id)
            {
                *self
                    .state
                    .room
                    .owned_furniture
                    .entry(id.to_string())
                    .or_insert(0) += 1;
                changed = true;
                if let Some(def) = room::furniture_def(id) {
                    let text = format!("补发等级家具：{}", def.name);
                    self.push_log(now, text.clone());
                    effects.push(Effect::Log { text });
                }
            }
        }
        // 成就/纪念日也会送绑定家具；用拥有数当“已发过”标记，不重复发。
        let days = ((now - self.state.created_at).max(0) / 86_400_000) as u32;
        let mut earned_gifts: Vec<(&str, &str)> = Vec::new();
        if self
            .state
            .achievements
            .get("sign_in_7")
            .copied()
            .unwrap_or(false)
        {
            earned_gifts.push(("gift_signin_lantern", "成就「签到七天」"));
        }
        if days >= 30 {
            earned_gifts.push(("gift_anniversary_quilt", "开垦满 30 天"));
        }
        for (id, reason) in earned_gifts {
            if self.state.room.owned_furniture.contains_key(id) {
                continue;
            }
            *self
                .state
                .room
                .owned_furniture
                .entry(id.to_string())
                .or_insert(0) += 1;
            changed = true;
            if let Some(def) = room::furniture_def(id) {
                let text = format!("{reason}，收到家具：{}", def.name);
                self.push_log(now, text.clone());
                effects.push(Effect::Log { text });
            }
        }

        let buffs_before = self.state.buffs.len();
        buffs::prune(&mut self.state.buffs, now);
        if self.state.buffs.len() != buffs_before {
            changed = true;
        }

        if self.state.daily.day != today {
            self.state.daily = DailyState {
                day: today.to_string(),
                ..DailyState::default()
            };
            let text = format!("新的一天：{today}");
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }

        let season = Season::from_today(today);
        if self.state.season != season {
            self.state.season = season;
            let adjusted = self.recalc_growth(now);
            let text = if adjusted > 0 {
                format!(
                    "换季：{}来了{}，{} 块地生长速度已调整",
                    season.label(),
                    season.emoji(),
                    adjusted
                )
            } else {
                format!("换季：{}来了{}", season.label(), season.emoji())
            };
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }
        if self
            .state
            .fishing
            .quests
            .ensure_day(today, self.state.skills.fishing.level)
        {
            let text = "钓鱼委托和今日比赛更新了".to_string();
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }
        let day_before_orders = self.state.orders.day.clone();
        let expired_orders = self.state.orders.ensure_day(
            today,
            self.state.level,
            self.state.workshop.built,
            &self.content,
        );
        for order in &expired_orders {
            if let Some(npc) = self
                .state
                .npcs
                .iter_mut()
                .find(|npc| npc.id == order.npc_id)
            {
                npc.friendship = (npc.friendship - 1).max(0);
            }
        }
        if day_before_orders != self.state.orders.day {
            let text = format!(
                "村庄订单更新了：{} 条新订单{}",
                self.state.orders.offers.len(),
                if expired_orders.is_empty() {
                    String::new()
                } else {
                    format!("，{} 条过期（NPC 友好度-1）", expired_orders.len())
                }
            );
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }
        let day_before_ranch_orders = self.state.ranch_orders.day.clone();
        let expired_ranch_orders = self
            .state
            .ranch_orders
            .ensure_day(today, self.state.level);
        if day_before_ranch_orders != self.state.ranch_orders.day {
            let text = format!(
                "牧场订单更新了：{} 条新订单{}",
                self.state.ranch_orders.offers.len(),
                if expired_ranch_orders.is_empty() {
                    String::new()
                } else {
                    format!("，{} 条过期", expired_ranch_orders.len())
                }
            );
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }
        let contest_week = crate::orders::week_start(today);
        if self.state.ranch_contest.ensure_week(&contest_week) {
            let text = format!("新的一周牧场评比开始了（{contest_week}）");
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }
        let season = self.state.season;
        let today_owned = today.to_string();
        let mut low_oxygen = false;
        let mut fry_report: Option<(String, u32)> = None;
        let mut order_changed = false;
        if let Some(pond) = self.state.pond.pond_mut() {
            pond.set_season(season);
            if pond.tick(now) {
                changed = true;
            }
            fry_report = pond.try_fry(now);
            if pond.oxygen < 20 && !pond.low_oxygen_logged {
                pond.low_oxygen_logged = true;
                low_oxygen = true;
            } else if pond.oxygen >= 30 {
                pond.low_oxygen_logged = false;
            }
        }
        if let Some(pond) = self.state.pond.pond().cloned() {
            order_changed = self.state.pond.ensure_order(&today_owned, &pond);
        }
        if let Some((fish_id, count)) = fry_report {
            self.state.pond.note_fry();
            let name = crate::fishing::fish(&fish_id)
                .map(|def| def.name)
                .unwrap_or(&fish_id);
            let text = format!("鱼塘里孵出一窝鱼苗：{name}×{count}");
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }
        if order_changed {
            if let Some(order) = &self.state.pond.order {
                let text = format!(
                    "鱼塘来订单了：{} 条{}（单条 ≥{:.1} kg）",
                    order.qty,
                    order.fish_name,
                    f64::from(order.min_weight_tenths) / 10.0
                );
                self.push_log(now, text.clone());
                effects.push(Effect::Log { text });
                changed = true;
            }
        }
        if low_oxygen {
            let text = "鱼塘缺氧，鱼开始浮头：装增氧机或清理鱼塘".to_string();
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }
        if self
            .state
            .mystery
            .ensure_day(today, self.state.level, &self.content)
        {
            let text = "神秘商店换了一批货".to_string();
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            changed = true;
        }

        if self.advance_npcs(now) {
            changed = true;
        }
        let disaster_effects = self.update_disasters(now, today);
        if !disaster_effects.is_empty() {
            changed = true;
            effects.extend(disaster_effects);
        }
        let visits = self.npc_visits(now);
        if !visits.is_empty() {
            changed = true;
            effects.extend(visits);
        }

        let gap = now - self.state.last_tick_at;
        if !(0..=180_000).contains(&gap) {
            self.state.last_tick_at = now;
        } else if gap >= 60_000 {
            self.state.last_tick_at = now;
            let bucket = (now / 60_000) as u64;
            for pos in 0..self.state.plots.len() {
                let Some(crop) = self.state.plots[pos].crop.as_ref() else {
                    continue;
                };
                if crop.pest.is_some() || crop.disaster.is_some() || crop.ready_at <= now {
                    continue;
                }
                let grow = (crop.ready_at - crop.planted_at).max(1);
                if now - crop.planted_at < grow / 4 {
                    continue;
                }
                let plot = self.state.plots[pos].index;
                let crop_id = crop.crop_id.clone();
                let roll = roll_pest(bucket, plot, &crop_id);
                let resist = self
                    .content
                    .crop(&crop_id)
                    .map(|def| u64::from(def.resistances.pest))
                    .unwrap_or(0);
                // 每小时每地块 12%（再乘抗性）：5 块地约一天能见到 3–5 次草虫。
                let chance = 120 * (100 - resist) / 100;
                if chance == 0 || roll % 1000 >= chance {
                    continue;
                }
                let kind = if (roll >> 16) & 1 == 0 {
                    PestKind::Grass
                } else {
                    PestKind::Bug
                };
                if let Some(c) = self.state.plots[pos].crop.as_mut() {
                    let remaining = (c.ready_at - now).max(0);
                    let slow_pct = match kind {
                        PestKind::Bug => 50,
                        PestKind::Grass => 25,
                    };
                    c.ready_at = c.ready_at.saturating_add(remaining * slow_pct / 100);
                    c.pest = Some(Pest { kind, since: now });
                }
                self.state.stats.pests_spawned += 1;
                let text = format!(
                    "{} 上长了{}，生长{}",
                    crop_id,
                    kind.label(),
                    if kind == PestKind::Grass {
                        "变慢"
                    } else {
                        "更慢"
                    }
                );
                self.push_log(now, text.clone());
                effects.push(Effect::Log { text });
                changed = true;
            }
        }

        if !self.state.pets.is_empty() && self.pet_tick(now, today) {
            changed = true;
        }
        if !self.state.pets.is_empty() && self.ranch_job_tick(now, today) {
            changed = true;
        }
        if !self.state.pets.is_empty() && self.fishing_job_tick(now, today) {
            changed = true;
        }
        if self.boarding_charge(now, today) {
            changed = true;
        }
        if self.boarding_settle(now) {
            changed = true;
        }
        if self.pet_mischief(now, today) {
            changed = true;
        }
        if self.fishing_auto_tick(now) {
            changed = true;
        }
        if self.ranch_tick(now) {
            changed = true;
        }
        if let Some(text) = self.npc_raid(now, today) {
            changed = true;
            effects.push(Effect::Log { text });
        }

        if changed {
            self.state.rev += 1;
            self.state.last_seen_at = now;
        }
        Outcome {
            rev: self.state.rev,
            changed,
            effects,
        }
    }

    pub fn check_achievements(&mut self, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        loop {
            let mut earned_any = false;
            for def in ACHIEVEMENTS.iter() {
                if self
                    .state
                    .achievements
                    .get(def.id)
                    .copied()
                    .unwrap_or(false)
                {
                    continue;
                }
                if progress(&self.state, def) < def.target {
                    continue;
                }
                self.state.achievements.insert(def.id.into(), true);
                self.state.coins = self.state.coins.saturating_add(def.reward_coins);
                let text = format!("成就达成：{}（金币+{}）", def.name, def.reward_coins);
                self.push_log(now, text.clone());
                effects.push(Effect::Log { text });
                effects.push(Effect::Achievement {
                    id: def.id.into(),
                    name: def.name.into(),
                });
                effects.extend(self.add_exp(def.reward_exp, now));
                earned_any = true;
            }
            if !earned_any {
                break;
            }
        }
        effects
    }

    fn save_room_layout(
        &mut self,
        room_id: &str,
        layout: RoomLayout,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        self.state.room.ensure_ownership();
        room::validate_layout(room_id, &layout)?;
        room::validate_ownership(room_id, &layout, &self.state.room)?;
        self.state.room.rooms.insert(room_id.to_string(), layout);
        let text = "你重新布置了小屋。".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn buy_furniture(&mut self, def_id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let def = room::furniture_def(def_id)
            .ok_or_else(|| GameError::RoomLayoutInvalid("没有这件家具".into()))?;
        let price = decor::furniture_price(def_id);
        if price == 0 {
            return Err(GameError::RoomLayoutInvalid("这件家具只在赠送/神秘商店出".into()));
        }
        if self.state.coins < price {
            return Err(GameError::NotEnoughCoins { need: price });
        }
        self.state.coins -= price;
        self.state.room.ensure_ownership();
        *self
            .state
            .room
            .owned_furniture
            .entry(def_id.to_string())
            .or_insert(0) += 1;
        let text = format!("买下 {}，金币-{}", def.name, price);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn buy_decor(&mut self, id: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        self.state.room.ensure_ownership();
        let offer = self
            .state
            .room
            .decor_shop
            .offer(id)
            .cloned()
            .ok_or(GameError::DecorOfferInvalid)?;
        if offer.sold {
            return Err(GameError::DecorOfferSold);
        }
        if self.state.coins < offer.price {
            return Err(GameError::NotEnoughCoins { need: offer.price });
        }
        self.state.coins -= offer.price;
        self.state.room.decor_shop.mark_sold(id);
        match offer.kind {
            decor::DecorKind::Floor => {
                self.state.room.owned_floors.insert(offer.item_id.clone());
            }
            decor::DecorKind::Wall => {
                self.state.room.owned_walls.insert(offer.item_id.clone());
            }
        }
        let text = format!("买下{}「{}」，金币-{}", offer.kind.label(), offer.name, offer.price);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn move_fish_to_aquarium(
        &mut self,
        room_id: &str,
        furniture_id: &str,
        fish_id: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let key = format!("fish:{fish_id}");
        if self.state.inventory.get(&key).copied().unwrap_or(0) == 0 {
            return Err(GameError::NotEnoughItems);
        }
        let layout = self
            .state
            .room
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| GameError::RoomLayoutInvalid("这个房间还没开放".into()))?;
        let item = layout
            .furniture
            .iter_mut()
            .find(|item| item.id == furniture_id)
            .ok_or_else(|| GameError::RoomLayoutInvalid("没有这个鱼缸".into()))?;
        let def = room::furniture_def(&item.def)
            .ok_or_else(|| GameError::RoomLayoutInvalid("没有这个鱼缸".into()))?;
        if def.capacity == 0 {
            return Err(GameError::RoomLayoutInvalid("这件家具不是鱼缸".into()));
        }
        if item.fish.len() >= def.capacity as usize {
            return Err(GameError::RoomLayoutInvalid("鱼缸已经满了".into()));
        }
        if let Some(qty) = self.state.inventory.get_mut(&key) {
            *qty -= 1;
            if *qty == 0 {
                self.state.inventory.remove(&key);
            }
        }
        item.fish.push(fish_id.to_string());
        let text = "你把一条鱼放进了鱼缸。".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn remove_fish_from_aquarium(
        &mut self,
        room_id: &str,
        furniture_id: &str,
        fish_id: &str,
        now: i64,
    ) -> Result<Vec<Effect>, GameError> {
        let layout = self
            .state
            .room
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| GameError::RoomLayoutInvalid("这个房间还没开放".into()))?;
        let item = layout
            .furniture
            .iter_mut()
            .find(|item| item.id == furniture_id)
            .ok_or_else(|| GameError::RoomLayoutInvalid("没有这个鱼缸".into()))?;
        let index = item
            .fish
            .iter()
            .position(|id| id == fish_id)
            .ok_or_else(|| GameError::RoomLayoutInvalid("鱼缸里没有这条鱼".into()))?;
        item.fish.remove(index);
        *self
            .state
            .inventory
            .entry(format!("fish:{fish_id}"))
            .or_insert(0) += 1;
        let text = "你把一条鱼捞回了仓库。".to_string();
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn unlock_extra_room(&mut self, now: i64) -> Result<Vec<Effect>, GameError> {
        if self.state.room.is_unlocked(room::EXTRA_ROOM) {
            return Err(GameError::RoomLayoutInvalid("里屋已经解锁了".into()));
        }
        let has_door = self
            .state
            .room
            .layout(room::LIVING_ROOM)
            .and_then(|layout| layout.door.as_ref())
            .is_some();
        if !has_door {
            return Err(GameError::RoomLayoutInvalid("先在小屋装一扇门".into()));
        }
        if self.state.coins < room::EXTRA_ROOM_COST {
            return Err(GameError::NotEnoughCoins {
                need: room::EXTRA_ROOM_COST,
            });
        }
        self.state.coins -= room::EXTRA_ROOM_COST;
        let extra = room::default_extra_layout();
        self.state.room.grant_layout(&extra);
        self.state
            .room
            .rooms
            .insert(room::EXTRA_ROOM.to_string(), extra);
        let text = format!("你推开门，里屋亮了起来（花费 {}）", room::EXTRA_ROOM_COST);
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    fn broadcast(&mut self, text: &str, now: i64) -> Result<Vec<Effect>, GameError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(GameError::EmptyText);
        }
        if trimmed.chars().count() > 40 {
            return Err(GameError::BroadcastTooLong);
        }
        let text = format!("你说：{trimmed}");
        self.push_log(now, text.clone());
        Ok(vec![Effect::Log { text }])
    }

    pub fn apply_content(&mut self, mut pack: ContentPack, now: i64) -> Result<(), GameError> {
        pack.migrate_balance();
        let issues = pack.validate();
        if !issues.is_empty() {
            return Err(GameError::InvalidContent(issues.join("；")));
        }
        if pack.nouns.farm.trim().is_empty() || pack.nouns.owner.trim().is_empty() {
            return Err(GameError::InvalidContent("农场名和主人名不能为空".into()));
        }
        if pack.nouns.plot.trim().is_empty() || pack.nouns.currency.trim().is_empty() {
            return Err(GameError::InvalidContent("地块名和货币名不能为空".into()));
        }

        let mut used = Vec::new();
        for plot in &self.state.plots {
            if let Some(crop) = &plot.crop {
                if pack.crop(&crop.crop_id).is_none() {
                    used.push(crop.crop_id.clone());
                }
            }
        }
        for key in self.state.inventory.keys() {
            if let Some((_, id)) = key.split_once(':') {
                if pack.crop(id).is_none() {
                    used.push(id.to_string());
                }
            }
        }
        used.sort();
        used.dedup();
        if !used.is_empty() {
            return Err(GameError::ContentRemovesUsedCrop(used.join("、")));
        }

        let count = pack.crops.len();
        self.content = pack;
        self.state.farm_name = self.content.nouns.farm.clone();
        self.state.owner_name = self.content.nouns.owner.clone();
        self.state.rev += 1;
        self.state.last_seen_at = now;
        let text = format!("内容包已更新：{count} 种作物");
        self.push_log(now, text);
        Ok(())
    }

    fn add_exp(&mut self, amount: u32, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.state.level >= MAX_LEVEL {
            self.state.exp = 0;
            return effects;
        }

        let rate = self.state.settings.exp_rate.clamp(1, 100);
        self.state.exp += u64::from(amount).saturating_mul(u64::from(rate));
        loop {
            if self.state.level >= MAX_LEVEL {
                self.state.exp = 0;
                break;
            }
            let need = level_need(self.state.level);
            if need == 0 || self.state.exp < need {
                break;
            }
            self.state.exp -= need;
            self.state.level += 1;
            self.state.stats.level_ups += 1;

            let text = format!("农场升到 {} 级", self.state.level);
            self.push_log(now, text.clone());
            effects.push(Effect::Log { text });
            effects.push(Effect::LevelUp {
                level: self.state.level,
            });
        }
        effects
    }

    fn push_log(&mut self, at: i64, text: String) {
        self.state.log.push(LogEntry { at, text });
        if self.state.log.len() > 50 {
            self.state.log.remove(0);
        }
    }
}

fn animal_index(animals: &[crate::npc::NpcRanchAnimal], index: u8) -> Option<usize> {
    if index == 0 {
        return None;
    }
    let pos = usize::from(index - 1);
    (pos < animals.len()).then_some(pos)
}

fn roll_pest(seed: u64, plot: u8, crop_id: &str) -> u64 {
    let mut hash = 1_469_598_103_934_665_603u64 ^ seed;
    for byte in crop_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^= u64::from(plot);
    hash = hash.wrapping_mul(1_099_511_628_211);
    hash ^ (hash >> 33)
}

fn disaster_resist(def: &CropDef, kind: DisasterKind) -> u32 {
    match kind {
        DisasterKind::Drought => u32::from(def.resistances.drought),
        DisasterKind::Flood => u32::from(def.resistances.flood),
        DisasterKind::Frost => u32::from(def.resistances.cold),
        DisasterKind::Blight => u32::from(def.resistances.pest),
        DisasterKind::Typhoon => u32::from(def.resistances.flood),
        DisasterKind::Hail => u32::from(def.resistances.cold) / 2,
    }
}

fn growth_factor_for(
    def: &CropDef,
    season: Season,
    region: Region,
    greenhouse_level: Option<u8>,
) -> u32 {
    if let Some(tier) = greenhouse_level {
        greenhouse_growth_factor(tier)
    } else if def.sow_seasons.is_empty() || def.sow_seasons.contains(&season) {
        1000
    } else {
        region.off_season_growth() * 10
    }
}

fn roll_land(plot: u8, planted_at: i64, now: i64) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ u64::from(plot);
    hash ^= planted_at as u64;
    hash = hash.wrapping_mul(1_099_511_628_211);
    hash ^= now as u64;
    hash = hash.wrapping_mul(1_099_511_628_211);
    hash ^ (hash >> 33)
}

fn roll_wither(plot: u8, now: i64, crop_id: &str) -> u64 {
    let mut hash = 0xD1B5_4A32_D192_ED03u64 ^ u64::from(plot);
    hash ^= now as u64;
    for byte in crop_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^ (hash >> 33)
}

fn roll_disaster(now: i64, today: &str, plot: u8, planted_at: i64) -> Option<DisasterKind> {
    let month = today
        .get(5..7)
        .and_then(|month| month.parse::<u32>().ok())
        .unwrap_or(7);
    let weights: [u64; 6] = match month {
        3..=5 => [1, 1, 2, 4, 3, 0],
        6..=8 => [5, 3, 3, 0, 3, 2],
        9..=11 => [1, 2, 1, 2, 4, 1],
        _ => [0, 0, 0, 5, 1, 0],
    };
    let total: u64 = weights.iter().sum();
    if total == 0 {
        return None;
    }

    let hour = (now / 3_600_000) as u64;
    let roll = roll_pest(hour ^ (planted_at as u64), plot, "disaster");
    // 每小时每地块 0.03%，约 70–90 天一次；由季节权重决定类型。
    if roll % 10_000 >= 3 {
        return None;
    }
    let mut pick = (roll >> 32) % total;
    for (index, weight) in weights.iter().enumerate() {
        if pick < *weight {
            return Some(match index {
                0 => DisasterKind::Drought,
                1 => DisasterKind::Flood,
                2 => DisasterKind::Hail,
                3 => DisasterKind::Frost,
                4 => DisasterKind::Blight,
                _ => DisasterKind::Typhoon,
            });
        }
        pick -= *weight;
    }
    None
}
