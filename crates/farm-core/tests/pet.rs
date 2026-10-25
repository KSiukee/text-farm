use farm_core::{
    guard_result, guard_result_full, Action, ContentPack, FishCodexEntry, Game, GameError,
    PetBreed, PetFoodKind, PetLocation, PetSickness, PetSicknessKind, PetSpecies, PetState,
    PetTalent,
};

fn bait_total(game: &Game) -> u32 {
    let fishing = &game.state().fishing;
    fishing.bait
        + fishing.bait_fine
        + fishing.bait_prime
        + fishing.bait_master
        + fishing.bait_corn.iter().sum::<u32>()
        + fishing.bait_worm.iter().sum::<u32>()
        + fishing.bait_lure.iter().sum::<u32>()
}

fn game_with_pet(species: PetSpecies) -> Game {
    let mut state = Game::new(0).state().clone();
    state.coins = 2_000;
    state.inventory.clear();
    state.pets = vec![PetState::new("豆豆".into(), species, 0).with_id("pet_1".into())];
    state.pet_seq = 1;
    Game::from_parts(state, ContentPack::starter())
}

#[test]
fn breed_profiles_change_interaction_gains() {
    let mut ragdoll = PetState::new_breed("布偶".into(), PetBreed::Ragdoll, 0);
    let mut silver = PetState::new_breed("银渐层".into(), PetBreed::SilverShaded, 0);
    ragdoll.touch(0);
    silver.touch(0);
    assert!(ragdoll.mood > silver.mood);
    let (_, loss) = guard_result(PetSpecies::Dog, 0, 80, false, 4, 0);
    assert_eq!(loss, 0);
}

#[test]
fn boarding_debt_can_be_settled_manually() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.state_mut().coins = 500;
    game.state_mut().boarding_debt = 1_000;
    game.dispatch(Action::SettleBoardingDebt, 0).unwrap();
    assert_eq!(game.state().coins, 0);
    assert_eq!(game.state().boarding_debt, 500);
    game.dispatch(Action::SettleBoardingDebt, 0).unwrap();
    assert_eq!(game.state().boarding_debt, 500);
}

#[test]
fn activity_streak_adds_long_term_bonus() {
    let mut pet = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    let text = pet
        .daily_activity_bonus(farm_core::PetActivity::Sunbathing, 3)
        .unwrap();
    assert!(text.contains("连续3天"));
}

#[test]
fn time_acceleration_shifts_pet_calendar() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.state_mut().pets[0].last_mischief_day = "2026-10-18".into();
    game.state_mut().pets[0].last_activity_day = "2026-10-18".into();
    game.state_mut().last_boarding_month = "2026-10".into();
    game.shift_time(31 * 24 * 60 * 60 * 1000);
    // 时间往前走了 31 天，上次事件日期应落到过去，否则每日判定会被跳过。
    assert_eq!(game.state().pets[0].last_mischief_day, "2026-09-17");
    assert_eq!(game.state().pets[0].last_activity_day, "2026-09-17");
    assert_eq!(game.state().last_boarding_month, "2026-11");
}

#[test]
fn behavior_tree_prioritizes_needs() {
    let mut pet = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    pet.boarded = true;
    assert_eq!(pet.activity(0).label(), "寄养中");
    pet.boarded = false;
    pet.sickness = Some(PetSickness {
        kind: PetSicknessKind::Cold,
        since: 0,
    });
    assert_eq!(pet.activity(0).label(), "养病");
    pet.sickness = None;
    pet.satiety = 10;
    assert_eq!(pet.activity(0).label(), "讨食");
    pet.satiety = 80;
    pet.energy = 10;
    assert_eq!(pet.activity(0).label(), "打盹");
    pet.energy = 80;
    pet.mood = 20;
    assert_eq!(pet.activity(0).label(), "发呆");
    assert!(!pet.activity(0).line().is_empty());
}

#[test]
fn active_skill_works_and_enters_cooldown() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.dispatch(
        Action::UsePetSkill {
            pet_id: "pet_1".into(),
        },
        0,
    )
    .unwrap();
    assert!(game
        .snapshot(0)
        .inventory
        .iter()
        .any(|item| item.key.starts_with("seed:")));
    assert!(matches!(
        game.dispatch(
            Action::UsePetSkill {
                pet_id: "pet_1".into()
            },
            0
        ),
        Err(GameError::PetSkillCooldown)
    ));
}

#[test]
fn breed_signature_skills_are_distinct() {
    let corgi = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    let shepherd = PetState::new_breed("黑背".into(), PetBreed::GermanShepherd, 0);
    let border = PetState::new_breed("边边".into(), PetBreed::BorderCollie, 0);
    assert_eq!(corgi.breed.signature().name, "绕圈牧牛");
    assert_eq!(shepherd.breed.signature().name, "警戒吠叫");
    assert!(shepherd.guard_bonus() > corgi.guard_bonus());
    assert_eq!(border.breed.signature().play_energy_save, 5);
    assert!(PetBreed::DomesticCat.signature().dig_bonus > 0);
}

#[test]
fn individual_personality_differs_within_same_breed() {
    let first = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    let second = PetState::new_breed("小牧".into(), PetBreed::Corgi, 0);
    assert_ne!(first.personality, second.personality);
    assert!(!first.personality.label().is_empty());
    assert!(!first.personality.summary().is_empty());
    assert!(first.mischief_chance() <= 8);
}

#[test]
fn frequent_sync_does_not_freeze_pet_decay() {
    let mut pet = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    pet.tick(30 * 60 * 1000, "2026-10-18", false);
    assert_eq!(pet.satiety, 80);
    pet.tick(90 * 60 * 1000, "2026-10-18", false);
    assert_eq!(pet.satiety, 78);
    assert_eq!(pet.energy, 82);
    pet.tick(120 * 60 * 1000, "2026-10-18", false);
    assert_eq!(pet.satiety, 76);
    assert_eq!(pet.energy, 84);
}

#[test]
fn skills_level_and_shorten_dig_cooldown() {
    let mut pet = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    assert_eq!(pet.dig_skill, 1);
    let before = pet.dig_cooldown_ms();
    assert!(pet.add_skill_exp(true, 40));
    assert_eq!(pet.dig_skill, 2);
    assert!(pet.dig_cooldown_ms() < before);
    let guard = PetState::new_breed("黑背".into(), PetBreed::GermanShepherd, 0);
    assert_eq!(guard.guard_skill, 2);
}

#[test]
fn supplies_can_be_bought_and_medicine_used() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.state_mut().coins = 10_000;
    game.state_mut().pets[0].sickness = Some(PetSickness {
        kind: PetSicknessKind::Cold,
        since: 0,
    });
    game.dispatch(
        Action::BuyPetSupply {
            kind: farm_core::PetSupplyKind::DogBed,
        },
        0,
    )
    .unwrap();
    game.dispatch(
        Action::BuyPetSupply {
            kind: farm_core::PetSupplyKind::Medicine,
        },
        0,
    )
    .unwrap();
    assert!(game.state().pet_care.dog_bed);
    assert_eq!(game.state().pet_care.medicine, 1);
    game.state_mut().pet_care.dog_bed_broken = true;
    let before = game.state().coins;
    game.dispatch(
        Action::RepairPetSupply {
            kind: farm_core::PetSupplyKind::DogBed,
        },
        0,
    )
    .unwrap();
    assert!(!game.state().pet_care.dog_bed_broken);
    assert!(game.state().coins < before);
    game.dispatch(
        Action::UsePetMedicine {
            pet_id: "pet_1".into(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().pets[0].sickness.is_none());
    assert_eq!(game.state().pet_care.medicine, 0);
}

#[test]
fn training_requires_center_and_cannot_max_everything() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.state_mut().pets[0].level = 30;
    assert!(matches!(
        game.dispatch(
            Action::TrainPet {
                pet_id: "pet_1".into(),
                kind: farm_core::PetTrainingKind::Guard
            },
            0
        ),
        Err(GameError::TrainingLocked)
    ));
    game.state_mut().facilities.training_center = true;
    for kind in [
        farm_core::PetTrainingKind::Guard,
        farm_core::PetTrainingKind::Guard,
        farm_core::PetTrainingKind::Dig,
        farm_core::PetTrainingKind::Mischief,
        farm_core::PetTrainingKind::Companion,
        farm_core::PetTrainingKind::Guard,
    ] {
        game.dispatch(
            Action::TrainPet {
                pet_id: "pet_1".into(),
                kind,
            },
            0,
        )
        .unwrap();
    }
    let pet = &game.state().pets[0];
    assert_eq!(pet.training_spent(), 6);
    assert!(!pet.can_train(farm_core::PetTrainingKind::Dig));
}

#[test]
fn training_center_unlocks_at_level_twelve() {
    let mut game = Game::new(0);
    game.state_mut().coins = 30_000;
    game.state_mut().level = 11;
    assert!(matches!(
        game.dispatch(
            Action::BuyFacility {
                kind: farm_core::FacilityKind::TrainingCenter
            },
            0
        ),
        Err(GameError::FacilityLocked { need: 12 })
    ));
    game.state_mut().level = 12;
    game.dispatch(
        Action::BuyFacility {
            kind: farm_core::FacilityKind::TrainingCenter,
        },
        0,
    )
    .unwrap();
    assert!(game.state().facilities.training_center);
    assert!(game.state().coins < 30_000);
}

#[test]
fn training_reduces_mischief_chance() {
    let mut pet = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    let base = pet.mischief_chance();
    assert!(base >= 1);
    pet.training.mischief = 4;
    assert!(pet.mischief_chance() < base);
}

#[test]
fn active_pet_limit_is_three_and_boarded_is_allowed() {
    let mut game = Game::new(0);
    for (name, breed) in [
        ("一号", PetBreed::Corgi),
        ("二号", PetBreed::Ragdoll),
        ("三号", PetBreed::Shiba),
    ] {
        game.dispatch(
            Action::AdoptPet {
                name: name.into(),
                breed,
                board: false,
            },
            0,
        )
        .unwrap();
    }
    assert!(matches!(
        game.dispatch(
            Action::AdoptPet {
                name: "四号".into(),
                breed: PetBreed::Bichon,
                board: false,
            },
            0
        ),
        Err(GameError::PetLimit)
    ));
    game.dispatch(
        Action::AdoptPet {
            name: "四号".into(),
            breed: PetBreed::Bichon,
            board: true,
        },
        0,
    )
    .unwrap();
    let pets = game.snapshot(0).pets;
    assert_eq!(pets.len(), 4);
    assert_eq!(pets.iter().filter(|pet| !pet.boarded).count(), 3);
    assert!(pets.iter().any(|pet| pet.name == "四号" && pet.boarded));
}

#[test]
fn breed_sets_species_and_image() {
    let mut game = Game::new(0);
    game.dispatch(
        Action::AdoptPet {
            name: "雪球".into(),
            breed: PetBreed::Ragdoll,
            board: false,
        },
        0,
    )
    .unwrap();
    let pet = game.snapshot(0).pet.unwrap();
    assert_eq!(pet.breed_label, "布偶猫");
    assert_eq!(pet.species_label, "猫猫");
    assert_eq!(pet.image, "/pets/ragdoll.png");
}

#[test]
fn feeding_consumes_fish_and_raises_satiety() {
    let mut game = game_with_pet(PetSpecies::Cat);
    game.state_mut().inventory.insert("fish:crucian".into(), 1);
    let before = game.snapshot(0).pet.unwrap().satiety;
    game.dispatch(
        Action::FeedPet {
            pet_id: "pet_1".into(),
            fish_id: "crucian".into(),
        },
        0,
    )
    .unwrap();
    let pet = game.snapshot(0).pet.unwrap();
    assert!(pet.satiety > before);
    assert!(pet.bond > 10);
    assert!(!game.state().inventory.contains_key("fish:crucian"));
}

#[test]
fn dogs_refuse_raw_fish() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.state_mut().inventory.insert("fish:crucian".into(), 1);
    assert!(matches!(
        game.dispatch(
            Action::FeedPet {
                pet_id: "pet_1".into(),
                fish_id: "crucian".into(),
            },
            0
        ),
        Err(GameError::PetFoodWrongSpecies)
    ));
}

#[test]
fn buy_and_feed_pet_food() {
    let mut game = game_with_pet(PetSpecies::Cat);
    game.state_mut().coins = 100;
    game.dispatch(
        Action::BuyPetFood {
            kind: PetFoodKind::CatFood,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().pet_care.cat_food, 1);
    assert_eq!(game.state().coins, 70);
    let before = game.snapshot(0).pet.unwrap().satiety;
    game.dispatch(
        Action::FeedPetFood {
            pet_id: "pet_1".into(),
            kind: PetFoodKind::CatFood,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().pet_care.cat_food, 0);
    assert!(game.snapshot(0).pet.unwrap().satiety > before);
    assert!(matches!(
        game.dispatch(
            Action::FeedPetFood {
                pet_id: "pet_1".into(),
                kind: PetFoodKind::DogFood,
            },
            0
        ),
        Err(GameError::PetFoodWrongSpecies)
    ));
}

#[test]
fn touch_and_play_have_cooldowns() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.dispatch(
        Action::PetCompanion {
            pet_id: "pet_1".into(),
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(
            Action::PetCompanion {
                pet_id: "pet_1".into()
            },
            0
        ),
        Err(GameError::PetCooldown)
    ));
    game.dispatch(
        Action::PlayWithPet {
            pet_id: "pet_1".into(),
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(
            Action::PlayWithPet {
                pet_id: "pet_1".into()
            },
            0
        ),
        Err(GameError::PetCooldown)
    ));
    game.dispatch(
        Action::PetCompanion {
            pet_id: "pet_1".into(),
        },
        60 * 60 * 1000,
    )
    .unwrap();
    game.dispatch(
        Action::PlayWithPet {
            pet_id: "pet_1".into(),
        },
        30 * 60 * 1000,
    )
    .unwrap();
}

#[test]
fn mischief_uses_day_seed() {
    let mut game = Game::new(0);
    game.state_mut().coins = 10_000;
    let mut pet =
        PetState::new_breed("豆豆".into(), farm_core::PetBreed::Corgi, 0).with_id("pet_1".into());
    pet.mood = 0;
    pet.last_mischief_day = "2026-11-20".into();
    game.state_mut().pets.push(pet);
    game.state_mut().last_raid_day = "2026-11-21".into();
    game.sync(0, "2026-11-21");
    let texts: Vec<&str> = game
        .state()
        .log
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(
        texts
            .iter()
            .any(|text| text.contains("拆") || text.contains("咬坏") || text.contains("踩坏")),
        "{texts:?}"
    );
}

#[test]
fn sickness_can_be_healed_with_coins() {
    let mut game = game_with_pet(PetSpecies::Cat);
    game.state_mut().pets[0].sickness = Some(PetSickness {
        kind: PetSicknessKind::Cold,
        since: 0,
    });
    assert!(game.snapshot(0).pet.unwrap().can_heal);
    game.dispatch(
        Action::HealPet {
            pet_id: "pet_1".into(),
        },
        0,
    )
    .unwrap();
    assert!(game.snapshot(0).pet.unwrap().sickness.is_none());
    assert_eq!(game.state().coins, 1_500);
}

#[test]
fn dog_digs_a_seed_and_has_cooldown() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.dispatch(
        Action::DigWithPet {
            pet_id: "pet_1".into(),
        },
        0,
    )
    .unwrap();
    assert!(game
        .snapshot(0)
        .inventory
        .iter()
        .any(|item| item.key.starts_with("seed:")));
    assert!(matches!(
        game.dispatch(
            Action::DigWithPet {
                pet_id: "pet_1".into()
            },
            0
        ),
        Err(GameError::PetCooldown)
    ));
    game.dispatch(
        Action::DigWithPet {
            pet_id: "pet_1".into(),
        },
        8 * 60 * 60 * 1000,
    )
    .unwrap();
}

#[test]
fn ranch_fishing_and_pond_are_available() {
    let mut game = game_with_pet(PetSpecies::Dog);
    for location in [PetLocation::Ranch, PetLocation::Fishing, PetLocation::Pond] {
        game.dispatch(
            Action::SetPetLocation {
                pet_id: "pet_1".into(),
                location,
            },
            0,
        )
        .unwrap();
        assert_eq!(game.state().pets[0].location, location);
    }
}

#[test]
fn pet_pond_job_settles_once_per_day_and_levels_up() {
    let mut game = game_with_pet(PetSpecies::Cat);
    game.state_mut().level = 20;
    game.state_mut().coins = 100_000;
    game.dispatch(Action::BuildPond, 0).unwrap();
    game.state_mut()
        .inventory
        .insert("fish:white_bait".into(), 1);
    game.dispatch(
        Action::StockPond {
            fish_id: "white_bait".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    game.state_mut().pets[0].location = PetLocation::Pond;
    game.state_mut().pets[0].pond_job.exp = 32;

    game.sync(1_000_000, "2026-10-19");
    let pet = &game.state().pets[0];
    assert_eq!(pet.last_job_day, "2026-10-19");
    // 32 + 8 = 40，正好升到 2 级并清零
    assert_eq!(pet.pond_job.level, 2);
    assert_eq!(pet.pond_job.exp, 0);

    let level = game.state().pets[0].pond_job.level;
    game.sync(2_000_000, "2026-10-19");
    assert_eq!(game.state().pets[0].pond_job.level, level);
    assert!(game.state().pets[0].milestones.contains("first_pond"));
}

#[test]
fn pet_daily_event_is_recorded_once_per_day() {
    let mut game = game_with_pet(PetSpecies::Cat);
    game.sync(1_000_000, "2026-10-19");
    let event = game.state().pets[0].last_event.clone();
    assert!(!event.is_empty());
    game.sync(2_000_000, "2026-10-19");
    assert_eq!(game.state().pets[0].last_event, event);
}

#[test]
fn pet_fishing_job_settles_once_per_day_and_levels_up() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.state_mut().pets[0].location = PetLocation::Fishing;
    game.state_mut().pets[0].fishing_job.exp = 32;
    let coins_before = game.state().coins;
    let baits_before = bait_total(&game);

    game.sync(1_000_000, "2026-10-19");
    let pet = &game.state().pets[0];
    assert_eq!(pet.last_job_day, "2026-10-19");
    // 32 + 8 = 40，正好升到 2 级并清零
    assert_eq!(pet.fishing_job.level, 2);
    assert_eq!(pet.fishing_job.exp, 0);
    // 狗要么叼回鱼饵，要么省下金币
    assert!(bait_total(&game) > baits_before || game.state().coins > coins_before);

    let coins_after = game.state().coins;
    let baits_after = bait_total(&game);
    game.sync(1_000_000, "2026-10-19");
    assert_eq!(game.state().coins, coins_after);
    assert_eq!(bait_total(&game), baits_after);
    assert_eq!(game.state().pets[0].fishing_job.level, 2);
}

#[test]
fn pet_fishing_job_cat_brings_fish_or_bait() {
    let mut game = game_with_pet(PetSpecies::Cat);
    game.state_mut().fishing.codex.insert(
        "white_bait".into(),
        FishCodexEntry {
            count: 1,
            ..Default::default()
        },
    );
    game.state_mut().pets[0].location = PetLocation::Fishing;
    let fish_before = game
        .state()
        .fishing
        .fish_batches
        .get("white_bait")
        .map(|batch| batch.count)
        .unwrap_or(0);
    let baits_before = bait_total(&game);

    game.sync(1_000_000, "2026-10-19");
    let fish_after = game
        .state()
        .fishing
        .fish_batches
        .get("white_bait")
        .map(|batch| batch.count)
        .unwrap_or(0);
    assert!(fish_after > fish_before || bait_total(&game) > baits_before);
    if fish_after > fish_before {
        assert_eq!(game.state().inventory.get("fish:white_bait"), Some(&1));
    }
    assert_eq!(game.state().pets[0].last_job_day, "2026-10-19");
}

#[test]
fn pet_fishing_job_skips_boarded_and_sick() {
    let mut game = game_with_pet(PetSpecies::Cat);
    game.state_mut().pets[0].location = PetLocation::Fishing;
    game.state_mut().pets[0].boarded = true;
    game.sync(1_000_000, "2026-10-19");
    assert!(game.state().pets[0].last_job_day.is_empty());

    game.state_mut().pets[0].boarded = false;
    game.state_mut().pets[0].sickness = Some(PetSickness {
        kind: PetSicknessKind::Cold,
        since: 0,
    });
    game.sync(2_000_000, "2026-10-20");
    assert!(game.state().pets[0].last_job_day.is_empty());
}

#[test]
fn guard_scares_or_reduces_loss() {
    let (scared, loss) = guard_result(PetSpecies::Dog, 50, 80, false, 4, 0);
    assert!(scared);
    assert_eq!(loss, 0);
    let (scared, loss) = guard_result(PetSpecies::Dog, 0, 80, false, 4, 999);
    assert!(!scared);
    assert_eq!(loss, 2);
    let (scared, loss) = guard_result(PetSpecies::Cat, 100, 10, false, 4, 999);
    assert!(!scared);
    assert_eq!(loss, 4);
}

#[test]
fn boarding_monthly_fee_is_auto_charged() {
    let mut state = Game::new(0).state().clone();
    state.coins = 10_000;
    let mut pet = PetState::new("豆豆".into(), PetSpecies::Dog, 0).with_id("pet_1".into());
    pet.boarded = true;
    state.pets = vec![pet];
    state.pet_seq = 1;
    state.last_boarding_month = "2026-09".into();
    let mut game = Game::from_parts(state, ContentPack::starter());
    game.sync(30 * 24 * 60 * 60 * 1000, "2026-10-01");
    assert_eq!(game.state().coins, 7_000);
    assert_eq!(game.state().boarding_debt, 0);
}

#[test]
fn offline_time_does_not_kill_the_pet() {
    let mut game = game_with_pet(PetSpecies::Dog);
    game.sync(24 * 60 * 60 * 1000, "2026-10-19");
    let pet = game.snapshot(24 * 60 * 60 * 1000).pet.unwrap();
    assert!(pet.satiety < 80);
    assert!(pet.health > 0);
}

#[test]
fn pet_talents_unlock_by_level_and_species() {
    let mut game = game_with_pet(PetSpecies::Dog);
    assert!(matches!(
        game.dispatch(
            Action::ChoosePetTalent {
                pet_id: "pet_1".into(),
                talent: PetTalent::Gatekeeper,
            },
            0
        ),
        Err(GameError::PetTalentLocked { need: 5 })
    ));

    game.state_mut().pets[0].level = 20;
    assert!(matches!(
        game.dispatch(
            Action::ChoosePetTalent {
                pet_id: "pet_1".into(),
                talent: PetTalent::SharpClaw,
            },
            0
        ),
        Err(GameError::PetTalentInvalid)
    ));

    for talent in [PetTalent::Gatekeeper, PetTalent::NightWatch, PetTalent::IronWall] {
        game.dispatch(
            Action::ChoosePetTalent {
                pet_id: "pet_1".into(),
                talent,
            },
            0,
        )
        .unwrap();
    }
    let pet = &game.state().pets[0];
    assert_eq!(pet.talents.tier1, Some(PetTalent::Gatekeeper));
    assert_eq!(pet.talents.tier2, Some(PetTalent::NightWatch));
    assert_eq!(pet.talents.tier3, Some(PetTalent::IronWall));

    // 同层可以改选；不花钱、不占训练点。
    game.dispatch(
        Action::ChoosePetTalent {
            pet_id: "pet_1".into(),
            talent: PetTalent::Fetch,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().pets[0].talents.tier1, Some(PetTalent::Fetch));

    // 视角里三层各两个选项，20 级全解锁。
    let view = game.snapshot(0).pet.unwrap();
    assert_eq!(view.talent_tiers.len(), 3);
    assert!(view.talent_tiers.iter().all(|tier| tier.unlocked));
    assert!(view
        .talent_tiers
        .iter()
        .all(|tier| tier.options.len() == 2));
    assert!(view.talent_tiers[0]
        .options
        .iter()
        .any(|option| option.id == PetTalent::Fetch && option.selected));
}

#[test]
fn pet_talent_effects_apply() {
    let mut dog = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0);
    let mut plain = dog.clone();
    dog.level = 20;
    plain.level = 20;
    dog.dig_skill = 1;
    plain.dig_skill = 1;
    let base_guard = plain.guard_bonus();
    dog.talents.pick(PetTalent::NightWatch);
    assert_eq!(dog.talent_guard_bonus(), 5);
    assert_eq!(dog.guard_bonus(), base_guard + 5);

    dog.talents.pick(PetTalent::Fetch);
    assert_eq!(
        dog.dig_cooldown_ms(),
        8 * 60 * 60 * 1000 - 30 * 60 * 1000
    );
    dog.talents.pick(PetTalent::Leader);
    assert_eq!(dog.talent_job_exp(), 12);
    assert_eq!(dog.talent_loss_cut(), 0);

    let mut cat = PetState::new_breed("雪球".into(), PetBreed::Ragdoll, 0);
    cat.level = 20;
    let mut plain_cat = cat.clone();
    cat.mood = 50;
    plain_cat.mood = 50;
    cat.talents.pick(PetTalent::Clingy);
    cat.touch(0);
    plain_cat.touch(0);
    assert!(cat.mood > plain_cat.mood);

    cat.talents.pick(PetTalent::FishingEye);
    assert_eq!(cat.talent_fishing_chance(), 10);
    cat.talents.pick(PetTalent::KeenEye);
    assert_eq!(cat.talent_fish_price_cap(), 120);
    assert_eq!(plain_cat.talent_fish_price_cap(), 70);

    // 铁壁：守卫失败时再减 1 点损失。
    let (scared, loss) = guard_result_full(PetSpecies::Dog, 10, 80, false, 4, 999, 0, 1);
    assert!(!scared);
    assert_eq!(loss, 1);
}
