use std::collections::BTreeMap;
use std::fs;

use clap::{Parser, Subcommand, ValueEnum};
use farm_core::{
    plot_unlock_cost, plot_unlock_level, review_batch, Action, BaitQuality, DisciplineKind,
    FacilityKind, Game, ItemKind, PetBreed, PetLocation, PetSpecies, PetTrainingKind, PlotStatus,
    RanchAnimalKind, Snapshot, FISHING_BAIT_PRICE, HELP_DAILY_LIMIT, PROFESSION_EXP_BASE,
    PROFESSION_EXP_SLOPE, STEAL_DAILY_LIMIT,
};
use serde_json::json;

#[derive(Parser)]
#[command(name = "farm-cli", about = "文字农场无头工具")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Policy {
    /// 每天早/中/晚各看一次，种 8 小时内能熟的作物
    Casual,
    /// 白天每 30 分钟看一次，种 4 小时内能熟的作物
    Active,
    /// 每 5 分钟看一次，什么赚钱种什么
    Greedy,
}

impl Policy {
    fn name(self) -> &'static str {
        match self {
            Policy::Casual => "casual",
            Policy::Active => "active",
            Policy::Greedy => "greedy",
        }
    }

    fn should_act(self, minute_of_day: u64) -> bool {
        match self {
            Policy::Casual => {
                minute_of_day == 8 * 60 || minute_of_day == 12 * 60 || minute_of_day == 20 * 60
            }
            Policy::Active => {
                (7 * 60..=23 * 60).contains(&minute_of_day) && minute_of_day.is_multiple_of(30)
            }
            Policy::Greedy => minute_of_day.is_multiple_of(5),
        }
    }

    fn max_grow_minutes(self) -> u32 {
        match self {
            Policy::Casual => 8 * 60,
            Policy::Active => 4 * 60,
            Policy::Greedy => u32::MAX,
        }
    }

    fn reserve(self) -> u64 {
        match self {
            Policy::Greedy => 0,
            _ => 300,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// 打印内核版本与开局快照
    Info,
    /// 时间加速模拟：按真实节奏跑 N 天，输出等级/金币/草虫/NPC 曲线
    Sim {
        #[arg(long, default_value_t = 30)]
        days: u32,
        #[arg(long, value_enum, default_value_t = Policy::Active)]
        policy: Policy,
        /// 把完整时间线写成 JSON
        #[arg(long)]
        out: Option<String>,
        /// 只输出 JSON，不打印人话摘要
        #[arg(long)]
        json: bool,
        /// 同时模拟 3 只当值宠物 + 1 只寄养宠物
        #[arg(long)]
        pets: bool,
        /// 同时模拟牧场：买动物、收产物、专用饲料、加工坊与宠物岗位
        #[arg(long)]
        ranch: bool,
        /// 同时模拟钓鱼：自动钓鱼 + 每天主动抛竿
        #[arg(long)]
        fishing: bool,
        /// 模拟吃喝加成食物（只吃模拟器会产出的奶酪/蛋糕）
        #[arg(long)]
        buffs: bool,
    },
    /// 生成一批种子并跑审核/时间加速沙盒
    Seedgen {
        #[arg(long, default_value_t = 24)]
        count: usize,
        #[arg(long, default_value_t = 20261018)]
        seed: u64,
        #[arg(long)]
        out: Option<String>,
    },
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn info() {
    let now = now_ms();
    let game = Game::new(now);
    let snap = game.snapshot(now);
    println!("farm-core {}", farm_core::CORE_VERSION);
    println!(
        "{}（{}） 等级 {}（{}/{}） {} {}",
        snap.farm_name,
        snap.owner_name,
        snap.level,
        snap.exp,
        snap.exp_need,
        snap.coins,
        game.content().nouns.currency
    );
    println!(
        "地块 {} 块，作物 {} 种",
        snap.plots.len(),
        snap.content_crops
    );
}

fn date_for_day(day_index: u32) -> String {
    let mut year = 2026u32;
    let mut remaining = day_index;
    loop {
        let leap = year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400);
        let months = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        for (index, days) in months.iter().enumerate() {
            if remaining < *days {
                return format!("{year}-{:02}-{:02}", index + 1, remaining + 1);
            }
            remaining -= *days;
        }
        year += 1;
    }
}

fn run(game: &mut Game, action: Action, now: i64, errors: &mut u64) -> bool {
    match game.dispatch(action.clone(), now) {
        Ok(_) => true,
        Err(_) => {
            *errors += 1;
            false
        }
    }
}

fn sell_all(game: &mut Game, now: i64, errors: &mut u64, reserve: bool, ranch: bool) {
    let snap = game.snapshot(now);
    for item in &snap.inventory {
        if item.kind == ItemKind::Produce && item.qty > 0 {
            let qty = if reserve {
                item.qty.saturating_sub(3)
            } else {
                item.qty
            };
            if qty == 0 {
                continue;
            }
            run(
                game,
                Action::Sell {
                    key: item.key.clone(),
                    qty,
                },
                now,
                errors,
            );
        }
    }
    if !ranch {
        return;
    }
    // 牧场原料留 2 份给加工坊，加工品直接卖。
    let snap = game.snapshot(now);
    for item in &snap.inventory {
        if item.kind != ItemKind::Ranch || !item.sellable || item.qty == 0 {
            continue;
        }
        let base = item.key.split('.').next().unwrap_or(&item.key);
        let keep = match base {
            "ranch:egg" | "ranch:duck_egg" | "ranch:milk" | "ranch:wool" => 2,
            _ => 0,
        };
        let qty = item.qty.saturating_sub(keep);
        if qty == 0 {
            continue;
        }
        run(
            game,
            Action::Sell {
                key: item.key.clone(),
                qty,
            },
            now,
            errors,
        );
    }
}

fn crop_score(game: &Game, crop_id: &str) -> u64 {
    let Some(def) = game.content().crop(crop_id) else {
        return 0;
    };
    let total_minutes = u64::from(def.grow_minutes)
        + u64::from(def.seasons.saturating_sub(1)) * u64::from(def.regrow_minutes);
    let revenue = u64::from(def.seasons)
        .saturating_mul(u64::from(def.yield_per_season))
        .saturating_mul(def.sell_price);
    let profit = revenue.saturating_sub(def.seed_price);
    profit
        .saturating_mul(1_000)
        .checked_div(total_minutes.max(1))
        .unwrap_or(0)
}

fn best_owned_seed(game: &Game, snap: &Snapshot) -> Option<String> {
    snap.inventory
        .iter()
        .filter(|item| item.kind == ItemKind::Seed && item.qty > 0)
        .filter_map(|item| item.key.strip_prefix("seed:"))
        .filter(|id| {
            game.content().crop(id).is_some_and(|crop| {
                crop.unlock_level <= snap.level
                    && (crop.sow_seasons.is_empty() || crop.sow_seasons.contains(&snap.season))
            })
        })
        .max_by_key(|id| crop_score(game, id))
        .map(str::to_string)
}

fn choose_crop(game: &Game, snap: &Snapshot, policy: Policy) -> Option<(String, u64)> {
    let max_grow = policy.max_grow_minutes();
    let mut candidates: Vec<(String, u64, u32, u64)> = game
        .content()
        .crops
        .iter()
        .filter(|crop| {
            crop.unlock_level <= snap.level
                && crop.seed_price <= snap.coins
                && (crop.sow_seasons.is_empty() || crop.sow_seasons.contains(&snap.season))
        })
        .map(|crop| {
            (
                crop.id.clone(),
                crop.seed_price,
                crop.grow_minutes,
                crop_score(game, &crop.id),
            )
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let short: Vec<_> = candidates
        .iter()
        .filter(|(_, _, grow, _)| *grow <= max_grow)
        .cloned()
        .collect();
    if !short.is_empty() {
        candidates = short;
    }
    candidates
        .into_iter()
        .max_by_key(|(_, _, _, score)| *score)
        .map(|(id, price, _, _)| (id, price))
}

fn npc_actions(game: &mut Game, now: i64, errors: &mut u64) {
    for _ in 0..STEAL_DAILY_LIMIT {
        let snap = game.snapshot(now);
        if snap.steals_left == 0 {
            break;
        }
        let Some((npc_id, plot)) = snap.npcs.iter().find_map(|npc| {
            npc.plots
                .iter()
                .find(|plot| plot.can_steal)
                .map(|plot| (npc.id.clone(), plot.index))
        }) else {
            break;
        };
        if !run(game, Action::StealNpc { npc_id, plot }, now, errors) {
            break;
        }
    }

    for _ in 0..HELP_DAILY_LIMIT {
        let snap = game.snapshot(now);
        if snap.helps_left == 0 {
            break;
        }
        let Some((npc_id, plot)) = snap.npcs.iter().find_map(|npc| {
            npc.plots
                .iter()
                .find(|plot| plot.can_help)
                .map(|plot| (npc.id.clone(), plot.index))
        }) else {
            break;
        };
        if !run(game, Action::HelpNpc { npc_id, plot }, now, errors) {
            break;
        }
    }
}

fn act(
    game: &mut Game,
    now: i64,
    policy: Policy,
    errors: &mut u64,
    pets: bool,
    ranch: bool,
    buffs: bool,
    buff_uses: &mut u64,
) {
    let snap = game.snapshot(now);

    if !snap.daily.sign_in {
        run(game, Action::ClaimSignIn, now, errors);
    }
    for task in &snap.daily.tasks {
        if task.claimable {
            run(game, Action::ClaimDaily { id: task.id }, now, errors);
        }
    }

    // 先处理灾害和草虫，再浇水、收获。
    let snap = game.snapshot(now);
    for plot in &snap.plots {
        if plot.crop.as_ref().is_some_and(|crop| crop.can_mitigate) {
            run(
                game,
                Action::MitigateDisaster { plot: plot.index },
                now,
                errors,
            );
        }
    }

    let snap = game.snapshot(now);
    for plot in &snap.plots {
        let Some(crop) = &plot.crop else { continue };
        if crop.pest.is_some() {
            run(game, Action::ClearPest { plot: plot.index }, now, errors);
        }
    }
    let snap = game.snapshot(now);
    for plot in &snap.plots {
        let Some(crop) = &plot.crop else { continue };
        if crop.can_water {
            run(game, Action::Water { plot: plot.index }, now, errors);
        }
        if crop.can_harvest {
            run(game, Action::Harvest { plot: plot.index }, now, errors);
        }
    }
    // 枯萎的作物不会自己消失；清掉才能重新种。
    let snap = game.snapshot(now);
    for plot in &snap.plots {
        if plot
            .crop
            .as_ref()
            .is_some_and(|crop| crop.status == PlotStatus::Withered)
        {
            run(game, Action::Clear { plot: plot.index }, now, errors);
        }
    }

    npc_actions(game, now, errors);
    if pets {
        pet_actions(game, now, errors);
    }
    if ranch {
        ranch_actions(game, now, errors, pets);
    }
    if buffs {
        buff_actions(game, now, errors, buff_uses);
    }
    sell_all(game, now, errors, pets, ranch);

    // 开地 / 建设施。
    for plot in 5..=8u8 {
        let snap = game.snapshot(now);
        let unlocked = snap.plots[(plot - 1) as usize].unlocked;
        let cost = plot_unlock_cost(plot);
        if !unlocked
            && snap.level >= plot_unlock_level(plot)
            && snap.coins >= cost + policy.reserve()
        {
            run(game, Action::UnlockPlot { plot }, now, errors);
        }
    }
    for kind in [
        FacilityKind::Sprinkler,
        FacilityKind::Harvester,
        FacilityKind::Windmill,
        FacilityKind::Greenhouse,
    ] {
        loop {
            let snap = game.snapshot(now);
            let Some(view) = snap.facilities.iter().find(|f| f.kind == kind) else {
                break;
            };
            if view.count >= view.max_count
                || snap.level < view.unlock_level
                || snap.coins < view.next_cost + policy.reserve()
            {
                break;
            }
            run(game, Action::BuyFacility { kind }, now, errors);
        }
    }

    for _ in 0..24 {
        let snap = game.snapshot(now);
        let Some(plot) = snap
            .plots
            .iter()
            .find(|plot| plot.can_upgrade && snap.coins >= plot.upgrade_cost + policy.reserve())
        else {
            break;
        };
        if !run(game, Action::UpgradePlot { plot: plot.index }, now, errors) {
            break;
        }
    }

    // 空地播种：优先用手里已有的种子，没有再买。
    loop {
        let snap = game.snapshot(now);
        let Some(plot) = snap
            .plots
            .iter()
            .find(|plot| plot.unlocked && plot.crop.is_none())
        else {
            break;
        };

        let crop_id = if let Some(id) = best_owned_seed(game, &snap) {
            id
        } else {
            let Some((id, price)) = choose_crop(game, &snap, policy) else {
                break;
            };
            if snap.coins < price {
                break;
            }
            if !run(
                game,
                Action::BuySeed {
                    crop_id: id.clone(),
                    qty: 1,
                },
                now,
                errors,
            ) {
                break;
            }
            id
        };

        if !run(
            game,
            Action::Plant {
                plot: plot.index,
                crop_id,
            },
            now,
            errors,
        ) {
            break;
        }
    }
}

fn inventory_value(game: &Game, snap: &Snapshot) -> u64 {
    snap.inventory
        .iter()
        .filter_map(|item| {
            if item.sellable {
                return Some(item.unit_price.saturating_mul(u64::from(item.qty)));
            }
            let (kind, id) = item.key.split_once(':')?;
            let def = game.content().crop(id)?;
            let unit = match kind {
                "seed" => def.seed_price,
                _ => 0,
            };
            Some(unit.saturating_mul(u64::from(item.qty)))
        })
        .sum()
}

fn setup_pets(game: &mut Game) {
    for (name, breed, board) in [
        ("豆豆", PetBreed::Corgi, false),
        ("雪球", PetBreed::Ragdoll, false),
        ("小柴", PetBreed::Shiba, false),
        ("布丁", PetBreed::BlueCat, true),
    ] {
        let _ = game.dispatch(
            Action::AdoptPet {
                name: name.into(),
                breed,
                board,
            },
            0,
        );
    }
}

fn pet_actions(game: &mut Game, now: i64, errors: &mut u64) {
    let snap = game.snapshot(now);
    let care = snap.pet_care.clone();
    let reserve = 1_000;
    let has_dog = snap
        .pets
        .iter()
        .any(|pet| !pet.boarded && pet.species == PetSpecies::Dog);
    let has_cat = snap
        .pets
        .iter()
        .any(|pet| !pet.boarded && pet.species == PetSpecies::Cat);

    if care.bowls < 3 && snap.coins >= care.bowl_cost + reserve {
        run(
            game,
            Action::BuyPetSupply {
                kind: farm_core::PetSupplyKind::Bowl,
            },
            now,
            errors,
        );
    }
    if has_dog && !care.dog_bed && snap.coins >= care.dog_bed_cost + reserve {
        run(
            game,
            Action::BuyPetSupply {
                kind: farm_core::PetSupplyKind::DogBed,
            },
            now,
            errors,
        );
    }
    if has_cat && !care.cat_bed && snap.coins >= care.cat_bed_cost + reserve {
        run(
            game,
            Action::BuyPetSupply {
                kind: farm_core::PetSupplyKind::CatBed,
            },
            now,
            errors,
        );
    }
    if has_dog && !care.dog_toy && snap.coins >= care.dog_toy_cost + reserve {
        run(
            game,
            Action::BuyPetSupply {
                kind: farm_core::PetSupplyKind::DogToy,
            },
            now,
            errors,
        );
    }
    if has_cat && !care.cat_toy && snap.coins >= care.cat_toy_cost + reserve {
        run(
            game,
            Action::BuyPetSupply {
                kind: farm_core::PetSupplyKind::CatToy,
            },
            now,
            errors,
        );
    }
    if care.medicine < 5 && snap.coins >= 1_500 {
        let _ = run(
            game,
            Action::BuyPetSupply {
                kind: farm_core::PetSupplyKind::Medicine,
            },
            now,
            errors,
        );
    }
    if care.dog_bed_broken && snap.coins >= care.dog_bed_repair + reserve {
        run(
            game,
            Action::RepairPetSupply {
                kind: farm_core::PetSupplyKind::DogBed,
            },
            now,
            errors,
        );
    }
    if care.cat_bed_broken && snap.coins >= care.cat_bed_repair + reserve {
        run(
            game,
            Action::RepairPetSupply {
                kind: farm_core::PetSupplyKind::CatBed,
            },
            now,
            errors,
        );
    }
    if care.dog_toy_broken && snap.coins >= care.dog_toy_repair + reserve {
        run(
            game,
            Action::RepairPetSupply {
                kind: farm_core::PetSupplyKind::DogToy,
            },
            now,
            errors,
        );
    }
    if care.cat_toy_broken && snap.coins >= care.cat_toy_repair + reserve {
        run(
            game,
            Action::RepairPetSupply {
                kind: farm_core::PetSupplyKind::CatToy,
            },
            now,
            errors,
        );
    }

    let snap = game.snapshot(now);
    if snap.level >= 12
        && !snap
            .facilities
            .iter()
            .any(|facility| facility.kind == FacilityKind::TrainingCenter && facility.owned)
        && snap.coins >= 21_000
    {
        run(
            game,
            Action::BuyFacility {
                kind: FacilityKind::TrainingCenter,
            },
            now,
            errors,
        );
    }

    let snap = game.snapshot(now);
    for pet in snap.pets.iter().filter(|pet| !pet.boarded) {
        if pet.sickness.is_some() {
            if snap.pet_care.medicine > 0 {
                run(
                    game,
                    Action::UsePetMedicine {
                        pet_id: pet.id.clone(),
                    },
                    now,
                    errors,
                );
            } else if snap.coins >= 600 {
                run(
                    game,
                    Action::HealPet {
                        pet_id: pet.id.clone(),
                    },
                    now,
                    errors,
                );
            }
            continue;
        }
        if pet.satiety < 65 {
            if let Some(item) = snap
                .inventory
                .iter()
                .find(|item| item.kind == ItemKind::Produce && item.qty > 0)
            {
                if let Some(crop_id) = item.key.strip_prefix("crop:") {
                    run(
                        game,
                        Action::FeedPet {
                            pet_id: pet.id.clone(),
                            crop_id: crop_id.to_string(),
                        },
                        now,
                        errors,
                    );
                }
            }
        }
        if pet.can_touch && pet.mood < 88 {
            let _ = run(
                game,
                Action::PetCompanion {
                    pet_id: pet.id.clone(),
                },
                now,
                errors,
            );
        }
        if pet.can_play && pet.energy > 35 && pet.mood < 88 {
            let _ = run(
                game,
                Action::PlayWithPet {
                    pet_id: pet.id.clone(),
                },
                now,
                errors,
            );
        }
        if pet.can_dig {
            let _ = run(
                game,
                Action::DigWithPet {
                    pet_id: pet.id.clone(),
                },
                now,
                errors,
            );
        }
        if pet.training_center && pet.training_spent < pet.training_cap {
            let kind = if pet.guard_train <= pet.dig_train
                && pet.guard_train <= pet.mischief_train
                && pet.guard_train <= pet.companion_train
            {
                PetTrainingKind::Guard
            } else if pet.dig_train <= pet.mischief_train && pet.dig_train <= pet.companion_train {
                PetTrainingKind::Dig
            } else if pet.mischief_train <= pet.companion_train {
                PetTrainingKind::Mischief
            } else {
                PetTrainingKind::Companion
            };
            let _ = run(
                game,
                Action::TrainPet {
                    pet_id: pet.id.clone(),
                    kind,
                },
                now,
                errors,
            );
        }
    }
}

fn ranch_actions(game: &mut Game, now: i64, errors: &mut u64, pets: bool) {
    let reserve = 200;
    loop {
        let snap = game.snapshot(now);
        if !snap.ranch.can_expand || snap.coins < snap.ranch.expand_cost + reserve {
            break;
        }
        if !run(game, Action::ExpandRanch, now, errors) {
            break;
        }
    }
    loop {
        let snap = game.snapshot(now);
        let Some(pen) = snap.ranch.pens.iter().find(|pen| pen.animal.is_none()) else {
            break;
        };
        let kind = if snap.coins >= 5_000 + reserve {
            RanchAnimalKind::Cow
        } else if snap.coins >= 3_500 + reserve {
            RanchAnimalKind::Sheep
        } else if snap.coins >= 1_200 + reserve {
            RanchAnimalKind::Duck
        } else if snap.coins >= 800 + reserve {
            RanchAnimalKind::Chicken
        } else {
            break;
        };
        if !run(
            game,
            Action::BuyRanchAnimal {
                pen: pen.index,
                kind,
            },
            now,
            errors,
        ) {
            break;
        }
    }

    let snap = game.snapshot(now);
    if !snap.workshop.built
        && snap.level >= snap.workshop.level
        && snap.coins >= snap.workshop.cost + reserve
    {
        run(game, Action::BuildWorkshop, now, errors);
    }

    let snap = game.snapshot(now);
    for pen in &snap.ranch.pens {
        let Some(animal) = &pen.animal else { continue };
        if animal.sick && snap.coins >= 300 + reserve {
            run(
                game,
                Action::HealRanchAnimal { pen: pen.index },
                now,
                errors,
            );
        }
        if animal.can_collect {
            run(
                game,
                Action::CollectRanchProduct { pen: pen.index },
                now,
                errors,
            );
        }
    }

    let snap = game.snapshot(now);
    for pen in &snap.ranch.pens {
        let Some(animal) = &pen.animal else { continue };
        if animal.can_feed && animal.satiety < 40 {
            run(
                game,
                Action::FeedRanchAnimal { pen: pen.index },
                now,
                errors,
            );
        }
    }

    let snap = game.snapshot(now);
    if snap.ranch.feed_count < 3 && snap.coins >= 300 + reserve {
        run(game, Action::BuyRanchFeed, now, errors);
    }
    let snap = game.snapshot(now);
    if snap.ranch.feed_count > 0 {
        if let Some(pen) = snap.ranch.pens.iter().find(|pen| {
            pen.animal
                .as_ref()
                .is_some_and(|animal| !animal.sick && animal.quality < 3)
        }) {
            run(
                game,
                Action::FeedRanchPremium { pen: pen.index },
                now,
                errors,
            );
        }
    }

    let snap = game.snapshot(now);
    for slot in &snap.workshop.slot_views {
        if slot.job.as_ref().is_some_and(|job| job.can_collect) {
            run(game, Action::CollectCraft { slot: slot.index }, now, errors);
        }
    }
    let snap = game.snapshot(now);
    if snap.workshop.built
        && snap
            .workshop
            .slot_views
            .first()
            .is_some_and(|slot| slot.job.is_none())
    {
        if let Some(recipe) = snap
            .workshop
            .manual_recipes
            .iter()
            .find(|recipe| recipe.can_start)
        {
            run(
                game,
                Action::StartRanchCraft {
                    recipe: recipe.recipe,
                },
                now,
                errors,
            );
        }
    }

    if pets {
        let snap = game.snapshot(now);
        if snap.ranch.pens.iter().any(|pen| pen.animal.is_some()) {
            if let Some(pet) = snap.pets.iter().find(|pet| {
                !pet.boarded && pet.species == PetSpecies::Dog && pet.location != PetLocation::Ranch
            }) {
                run(
                    game,
                    Action::SetPetLocation {
                        pet_id: pet.id.clone(),
                        location: PetLocation::Ranch,
                    },
                    now,
                    errors,
                );
            }
        }
    }
}

#[derive(Default)]
struct FishingSim {
    day: u32,
    remaining_casts: u32,
    rod_spend: u64,
    income: u64,
    active_income: u64,
    active_casts: u64,
}

fn fishing_daily_casts(policy: Policy) -> u32 {
    match policy {
        Policy::Casual => 0,
        Policy::Active => 60,
        Policy::Greedy => 90,
    }
}

fn fishing_success_rate(policy: Policy) -> u32 {
    match policy {
        Policy::Casual => 75,
        Policy::Active => 80,
        Policy::Greedy => 85,
    }
}

fn fishing_success(day: u32, index: u32, rate: u32) -> bool {
    let roll = (u64::from(day) * 97 + u64::from(index) * 31) % 100;
    roll < u64::from(rate)
}

/// 自动钓鱼只在白天（8–20 点）挂着；夜里模拟玩家关掉程序。
fn auto_fishing_hours(now: i64) -> bool {
    let hour = (now / 3_600_000).rem_euclid(24);
    (8..20).contains(&hour)
}

fn fishing_actions(
    game: &mut Game,
    now: i64,
    day: u32,
    policy: Policy,
    sim: &mut FishingSim,
    errors: &mut u64,
) {
    let reserve = policy.reserve();
    let mut snap = game.snapshot(now);

    // 买下一档买得起的鱼竿。
    if let Some(rod) = snap
        .fishing
        .rods
        .iter()
        .filter(|rod| !rod.owned && rod.can_buy && snap.coins >= rod.cost + reserve)
        .max_by_key(|rod| rod.level)
        .cloned()
    {
        if run(game, Action::BuyRod { rod: rod.kind }, now, errors) {
            sim.rod_spend = sim.rod_spend.saturating_add(rod.cost);
            snap = game.snapshot(now);
        }
    }

    // 磨损满了就保养，不修框会变重。
    if snap.fishing.needs_maintenance && snap.coins >= snap.fishing.maintenance_cost + reserve {
        if run(
            game,
            Action::MaintainRod {
                rod: snap.fishing.rod,
            },
            now,
            errors,
        ) {
            sim.rod_spend = sim.rod_spend.saturating_add(snap.fishing.maintenance_cost);
            snap = game.snapshot(now);
        }
    }

    let Some(best_spot) = snap
        .fishing
        .spots
        .iter()
        .filter(|spot| spot.unlocked)
        .max_by_key(|spot| spot.level)
        .map(|spot| spot.spot)
    else {
        return;
    };
    let best_rod = snap
        .fishing
        .rods
        .iter()
        .filter(|rod| rod.owned)
        .max_by_key(|rod| rod.level)
        .map(|rod| rod.kind)
        .unwrap_or(snap.fishing.rod);

    // 自动钓鱼按昼夜开关；只在配置真的变了时才重设，避免重置 lastAt。
    let auto_on = auto_fishing_hours(now);
    let auto = &snap.fishing.auto;
    if auto.enabled != auto_on || (auto_on && (auto.spot != best_spot || auto.rod != best_rod)) {
        run(
            game,
            Action::SetAutoFishing {
                spot: best_spot,
                enabled: auto_on,
                rod: Some(best_rod),
                bait_quality: Some(BaitQuality::Normal),
                bait_type: None,
            },
            now,
            errors,
        );
    }

    // 主动钓鱼：每天固定一小时的量，集中在当天第一次行动时打完。
    if sim.day != day {
        sim.day = day;
        sim.remaining_casts = fishing_daily_casts(policy);
    }
    let rate = fishing_success_rate(policy);
    let daily = fishing_daily_casts(policy);
    while sim.remaining_casts > 0 {
        if game.state().coins < FISHING_BAIT_PRICE * 10 + reserve {
            break;
        }
        if game.state().fishing.bait == 0
            && !run(
                game,
                Action::BuyBait {
                    qty: 10,
                    quality: BaitQuality::Normal,
                    bait_type: Default::default(),
                },
                now,
                errors,
            )
        {
            break;
        }
        if !run(
            game,
            Action::CastLine {
                spot: best_spot,
                quality: None,
            },
            now,
            errors,
        ) {
            break;
        }
        let index = daily.saturating_sub(sim.remaining_casts);
        sim.remaining_casts -= 1;
        sim.active_casts += 1;
        let success = fishing_success(day, index, rate);
        let _ = run(game, Action::ResolveFishing { success }, now, errors);
        if success {
            if let Some(catch) = game.state().fishing.last_catch.as_ref() {
                sim.active_income = sim.active_income.saturating_add(catch.price);
            }
        }
    }

    // 只卖鱼，别动农场/牧场库存。
    if game
        .state()
        .inventory
        .keys()
        .any(|key| key.starts_with("fish:"))
    {
        let before = game.state().coins;
        if run(game, Action::SellAllFish, now, errors) {
            sim.income = sim
                .income
                .saturating_add(game.state().coins.saturating_sub(before));
        }
    }
}

const POINT_ORDER: [(DisciplineKind, [&str; 4]); 4] = [
    (
        DisciplineKind::Planting,
        ["plant_yield", "plant_growth", "plant_water", "plant_seed"],
    ),
    (
        DisciplineKind::Ranching,
        ["ranch_yield", "ranch_quality", "ranch_feed", "ranch_care"],
    ),
    (
        DisciplineKind::Fishing,
        ["bait_saver", "auto_speed", "reel", "big_fish"],
    ),
    (
        DisciplineKind::Processing,
        ["craft_speed", "craft_yield", "craft_quality", "craft_save"],
    ),
];

fn profession_json(game: &Game) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for kind in DisciplineKind::ALL {
        let d = game.state().skills.get(kind);
        map.insert(
            kind.key().into(),
            json!({
                "level": d.level,
                "exp": d.exp,
                "points": d.points,
                "perks": d.perks,
            }),
        );
    }
    serde_json::Value::Object(map)
}

/// 按固定优先级花专业点；每轮每个专业最多升一项，升完重新取快照。
fn spend_points(game: &mut Game, now: i64, errors: &mut u64) {
    loop {
        let snap = game.snapshot(now);
        let mut did = false;
        for (kind, order) in POINT_ORDER {
            let Some(discipline) = snap.skills.disciplines.iter().find(|d| d.kind == kind) else {
                continue;
            };
            let next = order.iter().find(|id| {
                discipline
                    .skills
                    .iter()
                    .find(|skill| skill.id == **id)
                    .is_some_and(|skill| skill.can_upgrade)
            });
            if let Some(id) = next {
                if run(
                    game,
                    Action::UpgradeSkill {
                        discipline: kind,
                        skill: (*id).to_string(),
                    },
                    now,
                    errors,
                ) {
                    did = true;
                    break;
                }
            }
        }
        if !did {
            break;
        }
    }
}

/// 模拟器只吃自己会产出的两种加成食物：蛋糕（全经验）和奶酪（畜牧品质）。
fn buff_actions(game: &mut Game, now: i64, errors: &mut u64, buff_uses: &mut u64) {
    let snap = game.snapshot(now);
    for (key, id) in [
        ("ranch:cake", "buff_all_exp"),
        ("ranch:cheese", "buff_ranch_quality"),
    ] {
        let active = snap.skills.buffs.iter().find(|buff| buff.id == id);
        let near_expiry = active.is_some_and(|buff| buff.remaining_ms < 5 * 60_000);
        let prefix = format!("{key}.");
        let actual = snap
            .inventory
            .iter()
            .find(|item| (item.key == key || item.key.starts_with(&prefix)) && item.qty > 0)
            .map(|item| item.key.clone());
        if let Some(actual) = actual {
            if active.is_none() || near_expiry {
                if run(game, Action::UseBuffItem { key: actual }, now, errors) {
                    *buff_uses += 1;
                }
            }
        }
    }
}

fn sim(
    days: u32,
    policy: Policy,
    out: Option<String>,
    json_only: bool,
    pets: bool,
    ranch: bool,
    fishing: bool,
    buffs: bool,
) {
    let days = days.max(1);
    let total_minutes = u64::from(days) * 24 * 60;
    let mut game = Game::new(0);
    if pets {
        setup_pets(&mut game);
    }
    let mut errors = 0u64;
    let mut timeline = Vec::new();
    let mut level_days: BTreeMap<u32, u32> = BTreeMap::new();
    let mut last_stats = game.state().stats.clone();
    let mut fishing_sim = FishingSim::default();
    let mut last_fishing_catches = 0u64;
    let mut last_fishing_auto = 0u64;
    let mut last_fishing_income = 0u64;
    let mut buff_uses = 0u64;

    for minute in 0..total_minutes {
        let now = i64::try_from(minute * 60_000).unwrap_or(i64::MAX);
        let day = u32::try_from(minute / (24 * 60) + 1).unwrap_or(u32::MAX);
        let today = date_for_day(day - 1);
        let _ = game.sync(now, &today);

        if policy.should_act(minute % (24 * 60)) {
            act(
                &mut game,
                now,
                policy,
                &mut errors,
                pets,
                ranch,
                buffs,
                &mut buff_uses,
            );
            if fishing {
                fishing_actions(&mut game, now, day, policy, &mut fishing_sim, &mut errors);
            }
        }

        if minute % (24 * 60) == 23 * 60 {
            spend_points(&mut game, now, &mut errors);
            let snap = game.snapshot(now);
            let stats = game.state().stats.clone();
            let fish_stats = game.state().fishing.stats.clone();
            let unlocked = snap.plots.iter().filter(|p| p.unlocked).count();
            let liquid = snap.coins + inventory_value(&game, &snap);
            timeline.push(json!({
                "day": day,
                "level": snap.level,
                "exp": snap.exp,
                "coins": snap.coins,
                "liquid": liquid,
                "plots": unlocked,
                "harvests": stats.harvests.saturating_sub(last_stats.harvests),
                "plantings": stats.planted.saturating_sub(last_stats.planted),
                "pestsSpawned": stats.pests_spawned.saturating_sub(last_stats.pests_spawned),
                "pestsCleared": stats.pests_cleared.saturating_sub(last_stats.pests_cleared),
                "disastersSpawned": stats
                    .disasters_spawned
                    .saturating_sub(last_stats.disasters_spawned),
                "disastersMitigated": stats
                    .disasters_mitigated
                    .saturating_sub(last_stats.disasters_mitigated),
                "npcSteals": stats.npc_steals.saturating_sub(last_stats.npc_steals),
                "npcHelps": stats.npc_helps.saturating_sub(last_stats.npc_helps),
                "npcVisits": stats.npc_visits.saturating_sub(last_stats.npc_visits),
                "seedsBought": stats.seeds_bought.saturating_sub(last_stats.seeds_bought),
                "plotUpgrades": stats.plot_upgrades.saturating_sub(last_stats.plot_upgrades),
                "landTiers": snap.plots.iter().map(|plot| u64::from(plot.tier)).sum::<u64>(),
                "achievements": snap.achievements_earned,
                "codex": snap.codex_discovered,
                "fishingCatches": fish_stats.catches.saturating_sub(last_fishing_catches),
                "fishingAuto": fish_stats.auto_catches.saturating_sub(last_fishing_auto),
                "fishingIncome": fishing_sim.income.saturating_sub(last_fishing_income),
                "professions": profession_json(&game),
            }));
            for target in [5, 10, 15, 20, 25, 30] {
                if snap.level >= target {
                    level_days.entry(target).or_insert(day);
                }
            }
            last_stats = stats;
            last_fishing_catches = fish_stats.catches;
            last_fishing_auto = fish_stats.auto_catches;
            last_fishing_income = fishing_sim.income;
        }
    }

    let now = i64::try_from(total_minutes * 60_000).unwrap_or(i64::MAX);
    let snap = game.snapshot(now);
    let stats = game.state().stats.clone();
    let liquid = snap.coins + inventory_value(&game, &snap);
    let fish_stats = game.state().fishing.stats.clone();
    let fishing_bait_spend = fish_stats
        .casts
        .saturating_add(fish_stats.auto_catches)
        .saturating_mul(FISHING_BAIT_PRICE);
    let fishing_spend = fishing_sim.rod_spend.saturating_add(fishing_bait_spend);
    let fishing_net = fishing_sim.income.saturating_sub(fishing_spend);
    let auto_income = fishing_sim.income.saturating_sub(fishing_sim.active_income);
    let active_catches = fish_stats.catches.saturating_sub(fish_stats.auto_catches);
    let report = json!({
        "policy": policy.name(),
        "days": days,
        "curve": {
            "base": PROFESSION_EXP_BASE,
            "slope": PROFESSION_EXP_SLOPE,
        },
        "buffs": buffs,
        "buffUses": buff_uses,
        "final": {
            "level": snap.level,
            "exp": snap.exp,
            "coins": snap.coins,
            "liquid": liquid,
            "plots": snap.plots.iter().filter(|p| p.unlocked).count(),
            "landTiers": snap.plots.iter().map(|plot| u64::from(plot.tier)).sum::<u64>(),
            "facilities": snap.facilities.iter().filter(|f| f.owned).count(),
            "achievements": snap.achievements_earned,
            "codex": snap.codex_discovered,
            "npcFriendship": if snap.npcs.is_empty() { 0 } else {
                snap.npcs.iter().map(|npc| npc.friendship).sum::<i32>() / snap.npcs.len() as i32
            },
        },
        "levelDays": level_days.iter().map(|(k, v)| (k.to_string(), *v)).collect::<BTreeMap<_, _>>(),
        "totals": {
            "harvests": stats.harvests,
            "plantings": stats.planted,
            "pestsSpawned": stats.pests_spawned,
            "pestsCleared": stats.pests_cleared,
            "disastersSpawned": stats.disasters_spawned,
            "disastersMitigated": stats.disasters_mitigated,
            "seedsBought": stats.seeds_bought,
            "npcSteals": stats.npc_steals,
            "npcHelps": stats.npc_helps,
            "npcVisits": stats.npc_visits,
            "plotUpgrades": stats.plot_upgrades,
            "dailyClaimed": stats.daily_claimed,
        },
        "pets": if pets {
            json!({
                "count": snap.pets.len(),
                "active": snap.pets.iter().filter(|pet| !pet.boarded).count(),
                "boarded": snap.pets.iter().filter(|pet| pet.boarded).count(),
                "boardingDebt": snap.boarding_debt,
                "trainingCenter": snap.facilities.iter().any(|f| f.kind == FacilityKind::TrainingCenter && f.owned),
                "care": snap.pet_care,
                "list": snap.pets.iter().map(|pet| json!({
                    "name": pet.name,
                    "breed": pet.breed_label,
                    "boarded": pet.boarded,
                    "level": pet.level,
                    "bond": pet.bond,
                    "satiety": pet.satiety,
                    "energy": pet.energy,
                    "mood": pet.mood,
                    "health": pet.health,
                    "digSkill": pet.dig_skill,
                    "guardSkill": pet.guard_skill,
                    "trainingSpent": pet.training_spent,
                    "trainingCap": pet.training_cap,
                    "companionTrain": pet.companion_train,
                    "guardTrain": pet.guard_train,
                    "digTrain": pet.dig_train,
                    "mischiefTrain": pet.mischief_train,
                })).collect::<Vec<_>>(),
            })
        } else {
            json!(null)
        },
        "ranch": if ranch {
            json!({
                "pens": snap.ranch.pens.len(),
                "animals": snap.ranch.pens.iter().filter(|pen| pen.animal.is_some()).count(),
                "workshop": snap.workshop.built,
                "workshopSlots": snap.workshop.slots,
                "equipment": snap.workshop.slot_views.iter().filter_map(|slot| slot.equipment).collect::<Vec<_>>(),
                "crafting": snap.workshop.slot_views.iter().any(|slot| slot.job.is_some()),
                "quality": snap.ranch.pens.iter().filter_map(|pen| pen.animal.as_ref().map(|animal| animal.quality)).collect::<Vec<_>>(),
                "petsOnRanch": snap.pets.iter().filter(|pet| pet.location == PetLocation::Ranch).count(),
                "jobLevels": snap.pets.iter().filter(|pet| pet.location == PetLocation::Ranch).map(|pet| (pet.name.clone(), pet.job_level)).collect::<Vec<_>>(),
                "products": snap.inventory.iter().filter(|item| item.kind == ItemKind::Ranch && item.sellable).map(|item| (item.key.clone(), item.qty, item.unit_price)).collect::<Vec<_>>(),
            })
        } else {
            json!(null)
        },
        "fishing": if fishing {
            json!({
                "rod": snap.fishing.rod_label,
                "spot": snap.fishing.spots.iter().filter(|spot| spot.unlocked).max_by_key(|spot| spot.level).map(|spot| spot.label.clone()).unwrap_or_default(),
                "casts": fish_stats.casts,
                "catches": fish_stats.catches,
                "autoCatches": fish_stats.auto_catches,
                "activeCatches": active_catches,
                "activeCasts": fishing_sim.active_casts,
                "fails": fish_stats.fails,
                "codex": snap.fishing.codex_caught,
                "income": fishing_sim.income,
                "rodSpend": fishing_sim.rod_spend,
                "baitSpend": fishing_bait_spend,
                "net": fishing_net,
                "autoIncome": auto_income,
                "activeIncome": fishing_sim.income.saturating_sub(auto_income),
            })
        } else {
            json!(null)
        },
        "professions": profession_json(&game),
        "errors": errors,
        "timeline": timeline,
    });

    if let Some(path) = out {
        if let Err(error) = fs::write(&path, serde_json::to_string_pretty(&report).unwrap()) {
            eprintln!("写报告失败：{error}");
        }
    }

    if json_only {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return;
    }

    println!("\n== {} / {} 天 ==", policy.name(), days);
    println!(
        "最终：Lv{}（{}/{}） 金币 {} 流动资金 {} 地块 {} 土地档 {} 设施 {} 成就 {}/24 图鉴 {}/12",
        snap.level,
        snap.exp,
        snap.exp_need,
        snap.coins,
        liquid,
        snap.plots.iter().filter(|p| p.unlocked).count(),
        snap.plots
            .iter()
            .map(|plot| u64::from(plot.tier))
            .sum::<u64>(),
        snap.facilities.iter().filter(|f| f.owned).count(),
        snap.achievements_earned,
        snap.codex_discovered
    );
    let levels = level_days
        .iter()
        .map(|(level, day)| format!("L{level}=D{day}"))
        .collect::<Vec<_>>()
        .join("  ");
    println!("升级日：{levels}");
    let professions = snap
        .skills
        .disciplines
        .iter()
        .map(|d| format!("{} Lv{}（{}点）", d.label, d.level, d.points))
        .collect::<Vec<_>>()
        .join("  ");
    println!(
        "专业：{professions}    曲线 {} + {}×L    buff 使用 {} 次",
        PROFESSION_EXP_BASE, PROFESSION_EXP_SLOPE, buff_uses
    );
    if pets {
        let active = snap.pets.iter().filter(|pet| !pet.boarded).count();
        let boarded = snap.pets.iter().filter(|pet| pet.boarded).count();
        let levels = snap
            .pets
            .iter()
            .map(|pet| {
                format!(
                    "{} Lv{}({})",
                    pet.name,
                    pet.level,
                    if pet.boarded { "寄养" } else { "当值" }
                )
            })
            .collect::<Vec<_>>()
            .join("  ");
        println!(
            "宠物：当值 {active} / 寄养 {boarded} 欠费 {} 训练中心 {}",
            snap.boarding_debt,
            if snap
                .facilities
                .iter()
                .any(|f| f.kind == FacilityKind::TrainingCenter && f.owned)
            {
                "已建"
            } else {
                "未建"
            }
        );
        println!("宠物状态：{levels}");
    }
    if ranch {
        let animals = snap
            .ranch
            .pens
            .iter()
            .filter(|pen| pen.animal.is_some())
            .count();
        let quality = snap
            .ranch
            .pens
            .iter()
            .filter_map(|pen| pen.animal.as_ref().map(|animal| animal.quality))
            .collect::<Vec<_>>();
        let on_ranch = snap
            .pets
            .iter()
            .filter(|pet| pet.location == PetLocation::Ranch)
            .count();
        println!(
            "牧场：{} 栏 / {} 只动物，加工坊 {}，加工中 {}，牧场宠物 {}，品质 {:?}",
            snap.ranch.pens.len(),
            animals,
            if snap.workshop.built {
                "已建"
            } else {
                "未建"
            },
            if snap
                .workshop
                .slot_views
                .iter()
                .any(|slot| slot.job.is_some())
            {
                "是"
            } else {
                "否"
            },
            on_ranch,
            quality
        );
    }
    if fishing {
        println!(
            "钓鱼：抛竿 {} / 上鱼 {}（自动 {}，主动 {}）/ 图鉴 {}/35；收入 {}，竿饵支出 {}，净 {}（自动净约 {}）",
            fish_stats.casts,
            fish_stats.catches,
            fish_stats.auto_catches,
            active_catches,
            snap.fishing.codex_caught,
            fishing_sim.income,
            fishing_spend,
            fishing_net,
            auto_income.saturating_sub(fish_stats.auto_catches.saturating_mul(FISHING_BAIT_PRICE))
        );
    }
    println!(
        "累计：收获 {}，播种 {}，草虫 {}（清 {}），灾害 {}（处理 {}），买种 {}，土地升级 {}，偷菜 {}，帮忙 {}，串门 {}，日常 {}，错误 {}",
        stats.harvests,
        stats.planted,
        stats.pests_spawned,
        stats.pests_cleared,
        stats.disasters_spawned,
        stats.disasters_mitigated,
        stats.seeds_bought,
        stats.plot_upgrades,
        stats.npc_steals,
        stats.npc_helps,
        stats.npc_visits,
        stats.daily_claimed,
        errors
    );

    for row in &timeline {
        let day = row["day"].as_u64().unwrap_or(0);
        if day == 1
            || day == 7
            || day == 14
            || day == 30
            || day == 60
            || day == 90
            || day == days as u64
        {
            println!(
                "D{:>3}  Lv{:<2} 金币 {:>8}  流动 {:>8}  草虫 {:>4}/清 {:>4}  偷 {:>3}  帮 {:>3}",
                day,
                row["level"],
                row["coins"],
                row["liquid"],
                row["pestsSpawned"],
                row["pestsCleared"],
                row["npcSteals"],
                row["npcHelps"]
            );
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Cmd::Info) {
        Cmd::Info => info(),
        Cmd::Sim {
            days,
            policy,
            out,
            json,
            pets,
            ranch,
            fishing,
            buffs,
        } => sim(days, policy, out, json, pets, ranch, fishing, buffs),
        Cmd::Seedgen { count, seed, out } => seedgen(count, seed, out),
    }
}

fn seedgen(count: usize, seed: u64, out: Option<String>) {
    let reviews = review_batch(count, seed);
    let accepted = reviews
        .iter()
        .filter(|review| review.status == farm_core::ReviewStatus::Accepted)
        .count();
    let revised = reviews
        .iter()
        .filter(|review| review.status == farm_core::ReviewStatus::Revised)
        .count();
    let rejected = reviews
        .iter()
        .filter(|review| review.status == farm_core::ReviewStatus::Rejected)
        .count();
    println!("== 种子审核 / {count} 个 / seed {seed} ==");
    println!("通过 {accepted}，修正 {revised}，作废 {rejected}");
    for review in reviews
        .iter()
        .filter(|review| review.status == farm_core::ReviewStatus::Rejected)
        .take(8)
    {
        println!(
            "作废 {}（{:?}）：{}",
            review.crop.name,
            review.metrics,
            review.reasons.join("；")
        );
    }
    if let Some(path) = out {
        let value = json!({
            "seed": seed,
            "count": count,
            "reviews": reviews,
        });
        if let Err(error) = fs::write(&path, serde_json::to_string_pretty(&value).unwrap()) {
            eprintln!("写入 {path} 失败：{error}");
        } else {
            println!("审核报告：{path}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policies_act_at_expected_rhythms() {
        assert!(Policy::Casual.should_act(8 * 60));
        assert!(Policy::Casual.should_act(20 * 60));
        assert!(!Policy::Casual.should_act(9 * 60));
        assert!(Policy::Active.should_act(8 * 60 + 30));
        assert!(!Policy::Active.should_act(23 * 60 + 30));
        assert!(Policy::Greedy.should_act(5));
    }

    #[test]
    fn policy_crop_windows_are_ordered() {
        assert!(Policy::Casual.max_grow_minutes() > Policy::Active.max_grow_minutes());
        assert!(Policy::Active.max_grow_minutes() < Policy::Greedy.max_grow_minutes());
    }

    #[test]
    fn fishing_sim_budget_matches_policy() {
        assert_eq!(fishing_daily_casts(Policy::Casual), 0);
        assert_eq!(fishing_daily_casts(Policy::Active), 60);
        assert_eq!(fishing_daily_casts(Policy::Greedy), 90);
        assert_eq!(fishing_success_rate(Policy::Greedy), 85);
    }

    #[test]
    fn fishing_success_is_deterministic() {
        assert!(!fishing_success(1, 0, 80));
        assert!(fishing_success(1, 1, 80));
        assert!(fishing_success(1, 0, 99));
    }

    #[test]
    fn auto_fishing_only_runs_in_daylight() {
        assert!(!auto_fishing_hours(0));
        assert!(!auto_fishing_hours(7 * 3_600_000));
        assert!(auto_fishing_hours(8 * 3_600_000));
        assert!(auto_fishing_hours(19 * 3_600_000));
        assert!(!auto_fishing_hours(20 * 3_600_000));
        assert!(!auto_fishing_hours(23 * 3_600_000));
    }
}
