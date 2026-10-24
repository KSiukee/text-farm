use farm_core::{
    migrate_save, ranch_contest_rank_for, ranch_contest_score, Action, EquipmentKind,
    EquipmentRecipe, Game, GameError, PetBreed, PetLocation, PetState, RanchAnimal,
    RanchAnimalKind, RanchRecipe, RanchTrait, WorkshopJob, RANCH_BREED_COOLDOWN_MS, RANCH_YOUNG_MS,
};

fn job_ready_at(game: &Game, slot: u8) -> i64 {
    game.state()
        .workshop
        .job(slot)
        .map(WorkshopJob::ready_at)
        .unwrap_or(0)
}

fn job_quality(game: &Game, slot: u8) -> u8 {
    game.state()
        .workshop
        .job(slot)
        .map(WorkshopJob::quality)
        .unwrap_or(0)
}

fn finish_job(game: &mut Game, slot: u8) {
    if let Some(Some(job)) = game.state_mut().workshop.job_mut(slot) {
        match job {
            WorkshopJob::Ranch { ready_at, .. }
            | WorkshopJob::CropBox { ready_at, .. }
            | WorkshopJob::Equipment { ready_at, .. } => *ready_at = 0,
            WorkshopJob::Bait { ready_at, .. } => *ready_at = 0,
            WorkshopJob::Fish { ready_at, .. } => *ready_at = 0,
        }
    }
}

#[test]
fn buy_feed_collect_and_sell_ranch_product() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 1,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().coins, 4_200);
    assert!(game.state().ranch.pen(1).unwrap().animal.is_some());

    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 1);
    game.dispatch(Action::FeedRanchAnimal { pen: 1 }, 0)
        .unwrap();
    let animal = game.state().ranch.pen(1).unwrap().animal.as_ref().unwrap();
    assert_eq!(animal.satiety, 100);
    assert_eq!(game.state().inventory.get("crop:xiaobaicai"), None);

    assert!(matches!(
        game.dispatch(Action::CollectRanchProduct { pen: 1 }, 0),
        Err(GameError::RanchProductNotReady)
    ));
    game.state_mut()
        .ranch
        .pen_mut(1)
        .unwrap()
        .animal
        .as_mut()
        .unwrap()
        .product_ready_at = 0;
    game.dispatch(Action::CollectRanchProduct { pen: 1 }, 0)
        .unwrap();
    assert_eq!(game.state().inventory.get("ranch:egg"), Some(&1));

    game.dispatch(
        Action::Sell {
            key: "ranch:egg".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().coins, 4_245);
}

#[test]
fn ranch_can_expand_to_max_five_pens() {
    let mut game = Game::new(0);
    game.state_mut().coins = 30_000;
    game.dispatch(Action::ExpandRanch, 0).unwrap();
    assert_eq!(game.state().coins, 25_000);
    game.dispatch(Action::ExpandRanch, 0).unwrap();
    assert_eq!(game.state().coins, 5_000);
    assert_eq!(game.state().ranch.pens.len(), 5);
    assert!(matches!(
        game.dispatch(Action::ExpandRanch, 0),
        Err(GameError::RanchMaxPens)
    ));
}

#[test]
fn premium_feed_raises_quality_and_breeding_makes_calf() {
    let mut game = Game::new(0);
    game.state_mut().coins = 50_000;
    for pen in 1..=2 {
        game.dispatch(
            Action::BuyRanchAnimal {
                pen,
                kind: RanchAnimalKind::Chicken,
            },
            0,
        )
        .unwrap();
    }
    game.dispatch(Action::BuyRanchFeed, 0).unwrap();
    game.dispatch(Action::FeedRanchPremium { pen: 1 }, 0)
        .unwrap();
    assert_eq!(
        game.state()
            .ranch
            .pen(1)
            .unwrap()
            .animal
            .as_ref()
            .unwrap()
            .quality,
        2
    );
    game.dispatch(Action::BuyRanchFeed, 0).unwrap();
    game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0)
        .unwrap();
    let child = game.state().ranch.pen(3).unwrap().animal.as_ref().unwrap();
    assert_eq!(child.generation, 1);
    assert_eq!(child.quality, 2);
}

#[test]
fn ranch_pen_validation() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    let first = Action::BuyRanchAnimal {
        pen: 1,
        kind: RanchAnimalKind::Chicken,
    };
    game.dispatch(first.clone(), 0).unwrap();
    assert!(matches!(
        game.dispatch(first, 0),
        Err(GameError::RanchPenOccupied)
    ));
    assert!(matches!(
        game.dispatch(
            Action::BuyRanchAnimal {
                pen: 9,
                kind: RanchAnimalKind::Chicken,
            },
            0,
        ),
        Err(GameError::RanchPenEmpty)
    ));
}

#[test]
fn pet_can_guard_ranch_without_sickness() {
    let mut game = Game::new(0);
    game.state_mut().coins = 10_000;
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 1,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
    let pet = farm_core::PetState::new_breed("豆豆".into(), farm_core::PetBreed::Corgi, 0);
    let pet_id = pet.id.clone();
    game.state_mut().pets.push(pet);
    game.dispatch(
        Action::SetPetLocation {
            pet_id,
            location: farm_core::PetLocation::Ranch,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().pets[0].location, farm_core::PetLocation::Ranch);
}

#[test]
fn quality_changes_product_key_and_price() {
    let mut game = Game::new(0);
    game.state_mut().coins = 10_000;
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 1,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
    animal_mut(&mut game, 1).quality = 2;
    collect_until_inventory(&mut game, 1, "ranch:egg.fine");
    assert_eq!(game.state().inventory.get("ranch:egg.fine"), Some(&1));
    let coins = game.state().coins;
    game.dispatch(
        Action::Sell {
            key: "ranch:egg.fine".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    // 鸡蛋 45 × 优良 115% = 51
    assert_eq!(game.state().coins, coins + 51);

    animal_mut(&mut game, 1).quality = 3;
    collect_until_inventory(&mut game, 1, "ranch:egg.prime");
    assert!(game.state().inventory.get("ranch:egg.prime").is_some());
    let coins = game.state().coins;
    game.dispatch(
        Action::Sell {
            key: "ranch:egg.prime".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    // 鸡蛋 45 × 精品 130% = 58（精品不再一份变两份）
    assert_eq!(game.state().coins, coins + 58);

    game.state_mut().inventory.insert("ranch:feed".into(), 1);
    assert!(matches!(
        game.dispatch(
            Action::Sell {
                key: "ranch:feed".into(),
                qty: 1,
            },
            0,
        ),
        Err(GameError::OnlyProduceSellable)
    ));
}

#[test]
fn workshop_requires_level_then_builds() {
    let mut game = Game::new(0);
    game.state_mut().level = 7;
    game.state_mut().coins = 100_000;
    assert!(matches!(
        game.dispatch(Action::BuildWorkshop, 0),
        Err(GameError::LevelTooLow { need: 8 })
    ));
    game.state_mut().level = 8;
    game.state_mut().coins = 100;
    assert!(matches!(
        game.dispatch(Action::BuildWorkshop, 0),
        Err(GameError::NotEnoughCoins { .. })
    ));
    game.state_mut().coins = 50_000;
    // 先把「升到 8 级」这类成就的奖励结掉，免得混进建坊的账里
    game.dispatch(Action::ShowGuide, 0).unwrap();
    let coins = game.state().coins;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    assert!(game.state().workshop.built);
    assert_eq!(game.state().coins, coins - 6_000);
    assert!(matches!(
        game.dispatch(Action::BuildWorkshop, 0),
        Err(GameError::RanchWorkshopOwned)
    ));
}

#[test]
fn craft_consumes_lowest_quality_first_and_keeps_min_quality() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    assert!(matches!(
        game.dispatch(
            Action::StartRanchCraft {
                recipe: RanchRecipe::Cheese,
            },
            0,
        ),
        Err(GameError::RanchWorkshopMissing)
    ));
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    assert!(matches!(
        game.dispatch(
            Action::StartRanchCraft {
                recipe: RanchRecipe::Cheese,
            },
            0,
        ),
        Err(GameError::RanchIngredientsShort)
    ));

    game.state_mut().inventory.insert("ranch:milk".into(), 1);
    game.state_mut()
        .inventory
        .insert("ranch:milk.fine".into(), 1);
    game.dispatch(
        Action::StartRanchCraft {
            recipe: RanchRecipe::Cheese,
        },
        0,
    )
    .unwrap();
    // 先扣普通，再扣优良，品质取最低的普通
    assert_eq!(game.state().inventory.get("ranch:milk"), None);
    assert_eq!(game.state().inventory.get("ranch:milk.fine"), None);
    let craft = game.state().workshop.job(0).cloned().unwrap();
    assert_eq!(craft.quality(), 1);
    assert_eq!(craft.ready_at(), 4 * 60 * 60 * 1000);
    assert!(matches!(
        game.dispatch(
            Action::StartRanchCraft {
                recipe: RanchRecipe::Butter,
            },
            0,
        ),
        Err(GameError::RanchCraftBusy)
    ));
    assert!(matches!(
        game.dispatch(Action::CollectCraft { slot: 0 }, 0),
        Err(GameError::RanchCraftNotReady)
    ));
    finish_job(&mut game, 0);
    game.dispatch(Action::CollectCraft { slot: 0 }, 0).unwrap();
    assert_eq!(game.state().inventory.get("ranch:cheese"), Some(&1));
    assert!(game.state().workshop.job(0).is_none());
    assert!(matches!(
        game.dispatch(Action::CollectCraft { slot: 0 }, 0),
        Err(GameError::RanchCraftMissing)
    ));
}

#[test]
fn craft_with_all_fine_inputs_makes_fine_output() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.state_mut()
        .inventory
        .insert("ranch:wool.fine".into(), 2);
    game.dispatch(
        Action::StartRanchCraft {
            recipe: RanchRecipe::Yarn,
        },
        0,
    )
    .unwrap();
    assert_eq!(job_quality(&game, 0), 2);
    finish_job(&mut game, 0);
    game.dispatch(Action::CollectCraft { slot: 0 }, 0).unwrap();
    assert_eq!(game.state().inventory.get("ranch:yarn.fine"), Some(&1));
    let coins = game.state().coins;
    game.dispatch(
        Action::Sell {
            key: "ranch:yarn.fine".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    // 毛线 900 × 115% = 1035；出售可能同时触发成就奖励，按最低价断言
    assert!(game.state().coins >= coins + 1035);
}

#[test]
fn pet_ranch_job_settles_once_per_day_and_levels_up() {
    let mut game = Game::new(0);
    game.state_mut().coins = 10_000;
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 1,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
    let mut pet = PetState::new_breed("豆豆".into(), PetBreed::Corgi, 0).with_id("pet_1".into());
    pet.location = PetLocation::Ranch;
    pet.job.exp = 32;
    game.state_mut().pets.push(pet);

    let coins_before = game.state().coins;
    game.sync(1_000_000, "2026-10-19");
    let pet = &game.state().pets[0];
    assert_eq!(pet.last_job_day, "2026-10-19");
    // 32 + 8 = 40，正好升到 2 级并清零
    assert_eq!(pet.job.level, 2);
    assert_eq!(pet.job.exp, 0);
    let got_product = ["ranch:egg", "ranch:duck_egg", "ranch:milk", "ranch:wool"]
        .iter()
        .any(|key| game.state().inventory.get(*key).copied().unwrap_or(0) > 0);
    assert!(got_product || game.state().coins > coins_before);

    let coins_after = game.state().coins;
    game.sync(1_000_000, "2026-10-19");
    assert_eq!(game.state().pets[0].job.exp, 0);
    assert_eq!(game.state().coins, coins_after);
    assert_eq!(game.state().pets[0].job.level, 2);
}

#[test]
fn crop_box_crafts_and_sells() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    // 先把等级成就结掉，避免混进卖礼盒的账
    game.dispatch(Action::ShowGuide, 0).unwrap();
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.state_mut().inventory.insert("crop:yumi".into(), 6);

    game.dispatch(
        Action::StartCropCraft {
            crop_id: "yumi".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().inventory.get("crop:yumi"), Some(&1));
    let order = game.state().workshop.job(0).cloned().unwrap();
    match &order {
        WorkshopJob::CropBox { crop_id, .. } => assert_eq!(crop_id, "yumi"),
        _ => panic!("should be crop box"),
    }
    assert_eq!(order.ready_at(), 3 * 60 * 60 * 1000);
    assert!(matches!(
        game.dispatch(Action::CollectCraft { slot: 0 }, 0),
        Err(GameError::RanchCraftNotReady)
    ));

    finish_job(&mut game, 0);
    game.dispatch(Action::CollectCraft { slot: 0 }, 0).unwrap();
    assert_eq!(game.state().inventory.get("craft:box:yumi"), Some(&1));

    let coins = game.state().coins;
    game.dispatch(
        Action::Sell {
            key: "craft:box:yumi".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    // 5 份玉米 100 + 60 工时边距 = 160
    assert_eq!(game.state().coins, coins + 160);
}

#[test]
fn crop_box_needs_workshop_and_five_crops() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.state_mut().inventory.insert("crop:yumi".into(), 5);
    assert!(matches!(
        game.dispatch(
            Action::StartCropCraft {
                crop_id: "yumi".into(),
            },
            0,
        ),
        Err(GameError::RanchWorkshopMissing)
    ));

    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    assert!(matches!(
        game.dispatch(
            Action::StartCropCraft {
                crop_id: "nope".into(),
            },
            0,
        ),
        Err(GameError::UnknownCrop)
    ));
    game.state_mut().inventory.insert("crop:yumi".into(), 4);
    assert!(matches!(
        game.dispatch(
            Action::StartCropCraft {
                crop_id: "yumi".into(),
            },
            0,
        ),
        Err(GameError::NotEnoughItems)
    ));
}

#[test]
fn workshop_cannot_run_two_orders() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.state_mut().inventory.insert("ranch:milk".into(), 2);
    game.state_mut().inventory.insert("crop:yumi".into(), 5);
    game.dispatch(
        Action::StartRanchCraft {
            recipe: RanchRecipe::Cheese,
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(
            Action::StartCropCraft {
                crop_id: "yumi".into(),
            },
            0,
        ),
        Err(GameError::RanchCraftBusy)
    ));
}

#[test]
fn equipment_slots_install_and_craft() {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().coins = 100_000;
    game.dispatch(Action::ShowGuide, 0).unwrap();
    game.dispatch(Action::BuildWorkshop, 0).unwrap();

    let coins = game.state().coins;
    game.dispatch(
        Action::InstallEquipment {
            slot: 1,
            kind: EquipmentKind::StoneMill,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().coins, coins - 4_000);
    assert_eq!(
        game.state().workshop.equipment(1),
        Some(EquipmentKind::StoneMill)
    );

    game.state_mut().inventory.insert("crop:yumi".into(), 3);
    game.dispatch(
        Action::StartEquipmentCraft {
            recipe: EquipmentRecipe::CornStarch,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().inventory.get("crop:yumi"), None);
    assert_eq!(job_ready_at(&game, 1), 2 * 60 * 60 * 1000);
    assert!(matches!(
        game.dispatch(Action::CollectCraft { slot: 1 }, 0),
        Err(GameError::RanchCraftNotReady)
    ));

    finish_job(&mut game, 1);
    game.dispatch(Action::CollectCraft { slot: 1 }, 0).unwrap();
    assert_eq!(game.state().inventory.get("craft:corn_starch"), Some(&1));

    let coins = game.state().coins;
    game.dispatch(
        Action::Sell {
            key: "craft:corn_starch".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    // 玉米淀粉 260；出售可能同时触发成就奖励，按最低价断言
    assert!(game.state().coins >= coins + 260);
}

#[test]
fn p2_equipment_install_and_craft() {
    let cases: [(EquipmentKind, EquipmentRecipe, &[(&str, u32)]); 5] = [
        (
            EquipmentKind::Oven,
            EquipmentRecipe::CornPudding,
            &[
                ("craft:corn_starch", 1),
                ("craft:sugar", 1),
                ("ranch:egg", 2),
                ("ranch:milk", 1),
            ],
        ),
        (
            EquipmentKind::Mixer,
            EquipmentRecipe::Latte,
            &[("craft:coffee_powder", 1), ("ranch:milk", 1)],
        ),
        (
            EquipmentKind::OilPress,
            EquipmentRecipe::SunflowerOil,
            &[("crop:xiangrikui", 5)],
        ),
        (
            EquipmentKind::Loom,
            EquipmentRecipe::Sweater,
            &[("ranch:yarn", 3)],
        ),
        (
            EquipmentKind::Still,
            EquipmentRecipe::EssentialOil,
            &[("crop:yuejiancao", 2)],
        ),
    ];
    for (kind, recipe, inputs) in cases {
        let mut game = Game::new(0);
        game.state_mut().level = 30;
        game.state_mut().coins = 300_000;
        game.dispatch(Action::BuildWorkshop, 0).unwrap();
        game.dispatch(Action::ExpandWorkshop, 0).unwrap();
        game.dispatch(Action::ExpandWorkshop, 0).unwrap();
        game.dispatch(Action::InstallEquipment { slot: 1, kind }, 0)
            .unwrap();
        for (key, qty) in inputs {
            game.state_mut().inventory.insert((*key).into(), *qty);
        }
        game.dispatch(Action::StartEquipmentCraft { recipe }, 0)
            .unwrap();
        finish_job(&mut game, 1);
        game.dispatch(Action::CollectCraft { slot: 1 }, 0).unwrap();
        assert_eq!(
            game.state().inventory.get(recipe.output_key()),
            Some(&1),
            "{kind:?}"
        );
    }
}

#[test]
fn equipment_upgrade_speeds_job_and_costs_coins() {
    let mut game = Game::new(0);
    game.state_mut().level = 30;
    game.state_mut().coins = 100_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.dispatch(
        Action::InstallEquipment {
            slot: 1,
            kind: EquipmentKind::StoneMill,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().workshop.equipment_level(1), 1);
    let coins = game.state().coins;
    game.dispatch(Action::UpgradeEquipment { slot: 1 }, 0)
        .unwrap();
    assert_eq!(game.state().workshop.equipment_level(1), 2);
    assert_eq!(game.state().coins, coins - 2_000);
    assert_eq!(game.state().workshop.equipment_speed_pct(1), 4);
    assert_eq!(game.state().workshop.equipment_yield_pct(1), 3);

    game.state_mut().inventory.insert("crop:yumi".into(), 3);
    game.dispatch(
        Action::StartEquipmentCraft {
            recipe: EquipmentRecipe::CornStarch,
        },
        0,
    )
    .unwrap();
    // 2 小时 × (100 - 4)% = 1h55m12s
    assert_eq!(job_ready_at(&game, 1), 2 * 60 * 60 * 1000 * 96 / 100);
    assert!(matches!(
        game.dispatch(Action::UpgradeEquipment { slot: 1 }, 0),
        Err(GameError::RanchCraftBusy)
    ));
    finish_job(&mut game, 1);
    game.dispatch(Action::CollectCraft { slot: 1 }, 0).unwrap();

    for _ in 0..3 {
        game.dispatch(Action::UpgradeEquipment { slot: 1 }, 0)
            .unwrap();
    }
    assert_eq!(game.state().workshop.equipment_level(1), 5);
    assert_eq!(game.state().workshop.equipment_speed_pct(1), 16);
    assert_eq!(game.state().workshop.equipment_yield_pct(1), 12);
    assert!(matches!(
        game.dispatch(Action::UpgradeEquipment { slot: 1 }, 0),
        Err(GameError::WorkshopUpgradeMax)
    ));
    assert!(matches!(
        game.dispatch(Action::UpgradeEquipment { slot: 0 }, 0),
        Err(GameError::WorkshopUpgradeManual)
    ));
}

#[test]
fn equipment_requires_install_and_checks_level() {
    let mut game = Game::new(0);
    game.state_mut().level = 9;
    game.state_mut().coins = 100_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    assert!(matches!(
        game.dispatch(
            Action::InstallEquipment {
                slot: 1,
                kind: EquipmentKind::StoneMill,
            },
            0,
        ),
        Err(GameError::LevelTooLow { need: 10 })
    ));

    game.state_mut().level = 12;
    game.state_mut().inventory.insert("crop:yumi".into(), 3);
    assert!(matches!(
        game.dispatch(
            Action::StartEquipmentCraft {
                recipe: EquipmentRecipe::CornStarch,
            },
            0,
        ),
        Err(GameError::WorkshopEquipmentMissing)
    ));

    game.dispatch(
        Action::InstallEquipment {
            slot: 1,
            kind: EquipmentKind::Juicer,
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(
            Action::StartEquipmentCraft {
                recipe: EquipmentRecipe::CornStarch,
            },
            0,
        ),
        Err(GameError::WorkshopEquipmentMissing)
    ));
}

#[test]
fn workshop_expands_to_five_slots() {
    let mut game = Game::new(0);
    game.state_mut().level = 17;
    game.state_mut().coins = 200_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    assert_eq!(game.state().workshop.slot_count(), 3);
    assert!(matches!(
        game.dispatch(Action::ExpandWorkshop, 0),
        Err(GameError::LevelTooLow { need: 18 })
    ));

    game.state_mut().level = 18;
    let coins = game.state().coins;
    game.dispatch(Action::ExpandWorkshop, 0).unwrap();
    assert_eq!(game.state().workshop.slot_count(), 4);
    assert_eq!(game.state().coins, coins - 30_000);
    assert!(matches!(
        game.dispatch(Action::ExpandWorkshop, 0),
        Err(GameError::LevelTooLow { need: 24 })
    ));
    game.state_mut().level = 24;
    game.dispatch(Action::ExpandWorkshop, 0).unwrap();
    assert_eq!(game.state().workshop.slot_count(), 5);
    assert!(matches!(
        game.dispatch(Action::ExpandWorkshop, 0),
        Err(GameError::WorkshopMaxSlots)
    ));
}

#[test]
fn sugar_chain_uses_cane_juice() {
    let mut game = Game::new(0);
    game.state_mut().level = 14;
    game.state_mut().coins = 100_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.dispatch(
        Action::InstallEquipment {
            slot: 1,
            kind: EquipmentKind::BoilingPot,
        },
        0,
    )
    .unwrap();
    game.state_mut()
        .inventory
        .insert("craft:cane_juice".into(), 2);
    game.dispatch(
        Action::StartEquipmentCraft {
            recipe: EquipmentRecipe::Sugar,
        },
        0,
    )
    .unwrap();
    finish_job(&mut game, 1);
    game.dispatch(Action::CollectCraft { slot: 1 }, 0).unwrap();
    assert_eq!(game.state().inventory.get("craft:sugar"), Some(&1));
}

#[test]
fn workshop_job_deserializes_both_variants() {
    let job: WorkshopJob = serde_json::from_value(serde_json::json!({
        "kind": "ranch",
        "recipe": "cheese",
        "startedAt": 0,
        "readyAt": 100,
        "quality": 2
    }))
    .unwrap();
    match job {
        WorkshopJob::Ranch {
            recipe, quality, ..
        } => {
            assert_eq!(recipe, RanchRecipe::Cheese);
            assert_eq!(quality, 2);
        }
        _ => panic!("should be ranch"),
    }

    let job: WorkshopJob = serde_json::from_value(serde_json::json!({
        "kind": "cropBox",
        "cropId": "yumi",
        "startedAt": 0,
        "readyAt": 100
    }))
    .unwrap();
    assert!(matches!(job, WorkshopJob::CropBox { .. }));
}

fn add_chicken(game: &mut Game, pen: u8) {
    game.dispatch(
        Action::BuyRanchAnimal {
            pen,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
}

fn animal_mut(game: &mut Game, pen: u8) -> &mut RanchAnimal {
    game.state_mut()
        .ranch
        .pen_mut(pen)
        .unwrap()
        .animal
        .as_mut()
        .unwrap()
}

/// 收产物直到仓库里出现指定 key（精品/优良都有概率直接出特殊产物，测试要避开那一发）。
fn collect_until_inventory(game: &mut Game, pen: u8, key: &str) {
    let mut now = 0i64;
    while game.state().inventory.get(key).copied().unwrap_or(0) == 0 {
        animal_mut(game, pen).product_ready_at = 0;
        game.dispatch(Action::CollectRanchProduct { pen }, now)
            .unwrap();
        now += 1_000;
        assert!(now < 200_000, "等不到 {key}");
    }
}

#[test]
fn pen_cleaning_once_a_day() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    game.state_mut().daily.day = "2026-10-24".into();
    add_chicken(&mut game, 1);
    game.state_mut().ranch.pen_mut(1).unwrap().cleanliness = 40;

    game.dispatch(Action::CleanRanchPen { pen: 1 }, 0)
        .unwrap();
    let pen = game.state().ranch.pen(1).unwrap();
    assert_eq!(pen.cleanliness, 90);
    assert_eq!(pen.last_clean_day, "2026-10-24");
    assert!(matches!(
        game.dispatch(Action::CleanRanchPen { pen: 1 }, 0),
        Err(GameError::RanchPenCleanDone)
    ));

    game.state_mut().daily.day = "2026-10-25".into();
    game.dispatch(Action::CleanRanchPen { pen: 1 }, 0)
        .unwrap();
    assert_eq!(game.state().ranch.pen(1).unwrap().cleanliness, 100);
}

#[test]
fn production_traits_change_interval_and_caps() {
    // 高产：产物间隔 −10%。
    let mut fast = RanchAnimal::new(RanchAnimalKind::Cow, 0);
    fast.temper = RanchTrait::HighYield;
    assert_eq!(fast.product_interval(), 16 * 60 * 60 * 1000 * 9 / 10);
    fast.product_ready_at = 0;
    assert!(fast.collect(1_000));
    assert_eq!(
        fast.product_ready_at,
        1_000 + 16 * 60 * 60 * 1000 * 9 / 10
    );

    // 其余特性都是生产线上的数值，不带陪伴语义。
    assert_eq!(RanchTrait::Efficient.feed_save_bonus(), 12);
    assert_eq!(RanchTrait::TwinYield.yield_bonus(), 15);
    assert_eq!(RanchTrait::Hardy.sickness_delta(), -8);
    assert_eq!(RanchTrait::Delicate.sickness_delta(), 8);
    assert_eq!(RanchTrait::Delicate.quality_bonus(), 8);

    // 普通喂食/专用饲料不涨“亲密度”，只动饱食、健康、品质。
    let mut animal = RanchAnimal::new(RanchAnimalKind::Chicken, 0);
    animal.satiety = 50;
    animal.feed();
    assert_eq!(animal.satiety, 80);
    animal.feed_premium();
    assert_eq!(animal.quality, 2);
}

#[test]
fn clean_pen_raises_quality_and_dirty_pen_lowers_it() {
    let animal = RanchAnimal::new(RanchAnimalKind::Chicken, 0);
    let clean = animal.quality_chance(100);
    assert_eq!(clean, 33); // 8 + 100/4
    assert!(animal.quality_chance(20) < clean);
    assert_eq!(farm_core::special_chance(1), 0);
    assert_eq!(farm_core::special_chance(2), 8);
    assert_eq!(farm_core::special_chance(3), 25);
}

#[test]
fn prime_quality_collect_can_yield_special_and_dirties_pen() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    add_chicken(&mut game, 1);
    animal_mut(&mut game, 1).quality = 3;
    for step in 0..120i64 {
        animal_mut(&mut game, 1).product_ready_at = 0;
        game.dispatch(
            Action::CollectRanchProduct { pen: 1 },
            step * 86_400_000,
        )
        .unwrap();
    }
    let special = game
        .state()
        .inventory
        .get("ranch:golden_egg")
        .copied()
        .unwrap_or(0);
    assert!(special > 0, "精品产物应有概率出金蛋");
    assert!(
        game.state().ranch.pen(1).unwrap().cleanliness < 100,
        "收产物应该把畜栏弄脏"
    );
}

#[test]
fn tick_accumulates_sub_hour_syncs() {
    let mut animal = RanchAnimal::new(RanchAnimalKind::Chicken, 0);
    // 壳每 30 秒 sync 一次：不满一小时的零头不能把 last_tick_at 推走。
    animal.tick(30 * 60_000, true, 100);
    assert_eq!(animal.satiety, 80);
    assert_eq!(animal.last_tick_at, 0);
    animal.tick(60 * 60_000, true, 100);
    assert_eq!(animal.satiety, 79);
    assert_eq!(animal.last_tick_at, 60 * 60_000);
    animal.tick(90 * 60_000, true, 100);
    assert_eq!(animal.satiety, 79);
    animal.tick(120 * 60_000, true, 100);
    assert_eq!(animal.satiety, 78);
}

#[test]
fn v37_save_moves_cleanliness_to_pen_and_drops_companion_fields() {
    let value = serde_json::json!({
        "schema": 36,
        "savedAt": 0,
        "state": {"ranch": {"pens": [
            {"index": 1, "animal": {
                "kind": "chicken",
                "name": "小母鸡",
                "trait": "gentle",
                "affection": 100,
                "cleanliness": 65,
                "lastGroomDay": "2026-10-24",
                "lastGrazeDay": "2026-10-24",
                "lastPetDay": "2026-10-24"
            }},
            {"index": 2, "animal": null}
        ]}},
        "content": {}
    });
    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], farm_core::SAVE_SCHEMA);
    let pen = &migrated["state"]["ranch"]["pens"][0];
    assert_eq!(pen["cleanliness"], 65);
    assert_eq!(pen["lastCleanDay"], "");
    assert!(pen["animal"].get("affection").is_none());
    assert!(pen["animal"].get("cleanliness").is_none());
    assert!(pen["animal"].get("lastGroomDay").is_none());
    assert_eq!(pen["animal"]["trait"], "efficient");
    assert_eq!(migrated["state"]["ranch"]["pens"][1]["cleanliness"], 80);
    assert_eq!(pen["animal"]["lineId"], 1);
    assert_eq!(migrated["state"]["ranch"]["lineSeq"], 1);

    // 再跑一次（schema 已是 37）不改特性。
    let again = migrate_save(migrated.clone()).unwrap();
    assert_eq!(
        again["state"]["ranch"]["pens"][0]["animal"]["trait"],
        "efficient"
    );

    // 直接反序列化旧动物：缺字段拿到默认生产特性。
    let old: RanchAnimal =
        serde_json::from_value(serde_json::json!({"kind": "chicken", "name": "老母鸡"})).unwrap();
    assert_eq!(old.temper, RanchTrait::Efficient);
}

fn breeding_game() -> Game {
    let mut game = Game::new(0);
    game.state_mut().coins = 50_000;
    for pen in 1..=2 {
        game.dispatch(
            Action::BuyRanchAnimal {
                pen,
                kind: RanchAnimalKind::Chicken,
            },
            0,
        )
        .unwrap();
    }
    for _ in 0..2 {
        game.dispatch(Action::BuyRanchFeed, 0).unwrap();
    }
    game
}

#[test]
fn preview_matches_actual_child() {
    let mut game = breeding_game();
    let preview = game.preview_ranch_breeding(1, 2, 0).unwrap();
    assert!(preview.ok, "{:?}", preview.missing);
    game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0)
        .unwrap();
    let child = game.state().ranch.pen(3).unwrap().animal.as_ref().unwrap();
    assert_eq!(child.kind, preview.kind);
    assert_eq!(child.quality, preview.quality);
    assert_eq!(child.generation, preview.generation);
    assert_eq!(child.line_id, preview.line_id);
    assert_eq!(child.temper.label(), preview.trait_label);
    assert!(child.is_young(0));
    assert_eq!(child.adult_at, RANCH_YOUNG_MS);
    assert!(child.parents.contains("第1代"));
    assert_eq!(game.state().ranch.breed_seq, 1);
}

#[test]
fn breeding_blockers_are_reported_and_enforced() {
    // 幼年
    let mut game = breeding_game();
    animal_mut(&mut game, 1).adult_at = RANCH_YOUNG_MS;
    let preview = game.preview_ranch_breeding(1, 2, 0).unwrap();
    assert!(!preview.ok);
    assert!(preview.missing.iter().any(|item| item.contains("成年")));
    assert!(matches!(
        game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0),
        Err(GameError::RanchAnimalYoung)
    ));

    // 健康不足
    let mut game = breeding_game();
    animal_mut(&mut game, 1).health = 50;
    assert!(matches!(
        game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0),
        Err(GameError::RanchBreedUnhealthy)
    ));

    // 畜栏太脏
    let mut game = breeding_game();
    game.state_mut().ranch.pen_mut(1).unwrap().cleanliness = 20;
    assert!(matches!(
        game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0),
        Err(GameError::RanchPenDirty)
    ));

    // 没有饲料
    let mut game = breeding_game();
    game.state_mut().inventory.remove("ranch:feed");
    assert!(matches!(
        game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0),
        Err(GameError::RanchNoFeed)
    ));

    // 没有空栏
    let mut game = breeding_game();
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 3,
            kind: RanchAnimalKind::Duck,
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0),
        Err(GameError::RanchNoEmptyPen)
    ));

    // 同一只
    let mut game = breeding_game();
    assert!(matches!(
        game.dispatch(Action::BreedRanchPair { a: 1, b: 1 }, 0),
        Err(GameError::RanchBreedSamePen)
    ));
}

#[test]
fn breeding_cooldown_is_72_hours() {
    let mut game = breeding_game();
    game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0)
        .unwrap();
    game.dispatch(Action::BuyRanchFeed, 0).unwrap();
    game.dispatch(Action::ExpandRanch, 0).unwrap();

    let preview = game.preview_ranch_breeding(1, 2, 3_600_000).unwrap();
    assert!(!preview.ok);
    assert!(preview.missing.iter().any(|item| item.contains("冷却")));
    assert!(matches!(
        game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 3_600_000),
        Err(GameError::RanchBreedCooldown)
    ));
    game.dispatch(
        Action::BreedRanchPair { a: 1, b: 2 },
        RANCH_BREED_COOLDOWN_MS + 1_000,
    )
    .unwrap();
    assert_eq!(game.state().ranch.breed_seq, 2);
}

#[test]
fn young_cannot_collect_until_adult() {
    let mut game = breeding_game();
    game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0)
        .unwrap();
    assert!(matches!(
        game.dispatch(Action::CollectRanchProduct { pen: 3 }, 0),
        Err(GameError::RanchProductNotReady)
    ));
    assert!(matches!(
        game.dispatch(
            Action::CollectRanchProduct { pen: 3 },
            RANCH_YOUNG_MS
        ),
        Err(GameError::RanchProductNotReady)
    ));
    game.dispatch(
        Action::CollectRanchProduct { pen: 3 },
        RANCH_YOUNG_MS + 8 * 3_600_000,
    )
    .unwrap();
    assert!(
        game.state()
            .inventory
            .keys()
            .any(|key| key.starts_with("ranch:egg"))
    );
}

#[test]
fn inbreeding_caps_quality_and_marks_child() {
    let mut game = breeding_game();
    for pen in [1, 2] {
        let animal = animal_mut(&mut game, pen);
        animal.line_id = 7;
        animal.quality = 3;
    }
    let preview = game.preview_ranch_breeding(1, 2, 0).unwrap();
    assert!(preview.inbred);
    assert_eq!(preview.quality, 2);
    assert!(preview.warnings.iter().any(|item| item.contains("近亲")));

    game.dispatch(Action::BreedRanchPair { a: 1, b: 2 }, 0)
        .unwrap();
    let child = game.state().ranch.pen(3).unwrap().animal.as_ref().unwrap();
    assert!(child.inbred);
    assert_eq!(child.quality, 2);
    assert_eq!(child.line_id, 7);
}

#[test]
fn rename_ranch_animal_validates_length() {
    let mut game = breeding_game();
    game.dispatch(
        Action::RenameRanchAnimal {
            pen: 1,
            name: "小花".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.state()
            .ranch
            .pen(1)
            .unwrap()
            .animal
            .as_ref()
            .unwrap()
            .name,
        "小花"
    );
    assert!(matches!(
        game.dispatch(
            Action::RenameRanchAnimal {
                pen: 1,
                name: "  ".into(),
            },
            0,
        ),
        Err(GameError::InvalidRanchName)
    ));
    assert!(matches!(
        game.dispatch(
            Action::RenameRanchAnimal {
                pen: 1,
                name: "一二三四五六七八九十一二三".into(),
            },
            0,
        ),
        Err(GameError::InvalidRanchName)
    ));
}

#[test]
fn v38_save_migrates_breeding_fields() {
    let value = serde_json::json!({
        "schema": 37,
        "savedAt": 12_345,
        "state": {"ranch": {"pens": [
            {"index": 1, "cleanliness": 90, "lastCleanDay": "", "animal": {
                "kind": "chicken", "name": "甲", "bred": true
            }},
            {"index": 2, "cleanliness": 90, "lastCleanDay": "", "animal": {
                "kind": "chicken", "name": "乙", "bred": false
            }}
        ]}},
        "content": {}
    });
    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], farm_core::SAVE_SCHEMA);
    let ranch = &migrated["state"]["ranch"];
    assert_eq!(ranch["breedSeq"], 0);
    assert_eq!(ranch["lineSeq"], 2);
    let first = &ranch["pens"][0]["animal"];
    assert_eq!(first["lineId"], 1);
    assert_eq!(first["lastBreedAt"], 12_345);
    assert_eq!(first["adultAt"], 0);
    assert_eq!(first["inbred"], false);
    assert_eq!(first["parents"], "");
    assert!(first.get("bred").is_none());
    assert_eq!(ranch["pens"][1]["animal"]["lineId"], 2);
    assert_eq!(ranch["pens"][1]["animal"]["lastBreedAt"], 0);
}

#[test]
fn ranch_orders_accept_deliver_and_reward() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-24".into();
    game.state_mut()
        .ranch_orders
        .ensure_day("2026-10-24", 10);
    let offer = game.state().ranch_orders.offers[0].clone();
    let key = offer.item_key();
    game.state_mut().inventory.insert(key.clone(), offer.qty);
    game.dispatch(
        Action::AcceptRanchOrder {
            id: offer.id.clone(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().ranch_orders.active.len(), 1);
    let coins = game.state().coins;
    let feed = game.state().inventory.get("ranch:feed").copied().unwrap_or(0);
    game.dispatch(
        Action::DeliverRanchOrder {
            id: offer.id.clone(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().coins, coins + offer.reward_coins);
    assert_eq!(
        game.state().inventory.get("ranch:feed").copied().unwrap_or(0),
        feed + offer.reward_feed
    );
    assert!(game.state().inventory.get(&key).is_none());
    assert!(game.state().ranch_orders.active.is_empty());
}

#[test]
fn ranch_orders_reject_short_and_limit() {
    let mut game = Game::new(0);
    game.state_mut()
        .ranch_orders
        .ensure_day("2026-10-24", 10);
    let offer = game.state().ranch_orders.offers[0].clone();
    game.dispatch(
        Action::AcceptRanchOrder {
            id: offer.id.clone(),
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(
            Action::DeliverRanchOrder {
                id: offer.id.clone()
            },
            0
        ),
        Err(GameError::RanchOrderShort)
    ));

    // 接单上限：手上已有两条，再点报价就拒绝。
    let offers = game.state().ranch_orders.offers.clone();
    let mut active = game.state().ranch_orders.active.clone();
    active.push(offers[0].clone());
    game.state_mut().ranch_orders.active = active;
    assert!(matches!(
        game.dispatch(
            Action::AcceptRanchOrder {
                id: offers[0].id.clone()
            },
            0
        ),
        Err(GameError::RanchOrderLimit)
    ));
}

#[test]
fn special_recipes_craft_into_new_dishes() {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().coins = 20_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.state_mut()
        .inventory
        .insert("ranch:golden_egg".into(), 1);
    game.state_mut().inventory.insert("ranch:milk".into(), 1);
    game.state_mut().inventory.insert("ranch:egg".into(), 1);
    game.dispatch(
        Action::StartRanchCraft {
            recipe: RanchRecipe::GoldenTart,
        },
        0,
    )
    .unwrap();
    finish_job(&mut game, 0);
    game.dispatch(Action::CollectCraft { slot: 0 }, 0)
        .unwrap();
    assert!(
        game.state()
            .inventory
            .keys()
            .any(|key| key.starts_with("ranch:golden_tart"))
    );
    assert_eq!(RanchRecipe::GoldenTart.output_price(), 1_125);
    assert_eq!(RanchRecipe::CreamCake.output_price(), 1_670);
    assert_eq!(RanchRecipe::FineWoolScarf.output_price(), 2_110);
}

#[test]
fn ranch_codex_tracks_owned_combos_and_special_products() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    add_chicken(&mut game, 1);
    let codex = game.snapshot(0).ranch.codex;
    assert_eq!(codex.animal_total, 20);
    assert_eq!(codex.special_total, 4);
    assert!(codex.animal_done >= 1);

    animal_mut(&mut game, 1).quality = 3;
    for step in 0..80i64 {
        animal_mut(&mut game, 1).product_ready_at = 0;
        game.dispatch(
            Action::CollectRanchProduct { pen: 1 },
            step * 86_400_000,
        )
        .unwrap();
    }
    let codex = game.snapshot(0).ranch.codex;
    assert!(codex.special_done >= 1);
}

#[test]
fn v39_save_adds_ranch_orders_and_codex() {
    let value = serde_json::json!({
        "schema": 38,
        "savedAt": 0,
        "state": {"ranch": {"pens": []}},
        "content": {}
    });
    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], farm_core::SAVE_SCHEMA);
    assert_eq!(migrated["state"]["ranchOrders"]["day"], "");
    assert_eq!(migrated["state"]["ranchOrders"]["offers"], serde_json::json!([]));
    assert_eq!(migrated["state"]["ranch"]["codexAnimals"], serde_json::json!([]));
    assert_eq!(migrated["state"]["ranch"]["codexSpecials"], serde_json::json!([]));
}

#[test]
fn v40_save_adds_ranch_contest() {
    let value = serde_json::json!({
        "schema": 39,
        "savedAt": 0,
        "state": {"ranch": {"pens": []}},
        "content": {}
    });
    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], farm_core::SAVE_SCHEMA);
    assert_eq!(migrated["state"]["ranchContest"]["week"], "");
    assert_eq!(migrated["state"]["ranchContest"]["entered"], false);
    assert_eq!(migrated["state"]["ranchContest"]["medals"], 0);
    assert_eq!(migrated["state"]["ranchContest"]["bestRank"], 0);
}

#[test]
fn ranch_contest_enter_once_and_reset_next_week() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    game.state_mut().daily.day = "2026-10-19".into();
    add_chicken(&mut game, 1);
    {
        let animal = animal_mut(&mut game, 1);
        animal.quality = 3;
        animal.generation = 3;
        animal.health = 100;
    }
    let expected_score = ranch_contest_score(
        game.state()
            .ranch
            .pen(1)
            .unwrap()
            .animal
            .as_ref()
            .unwrap(),
    );
    let before = game.state().coins;
    game.dispatch(Action::EnterRanchContest { pen: 1 }, 0)
        .unwrap();
    let contest = game.state().ranch_contest.clone();
    assert!(contest.entered);
    assert_eq!(contest.medals, contest.result.as_ref().unwrap().medal);
    assert!(game.state().coins > before);
    assert_eq!(
        contest.result.as_ref().unwrap().rank,
        ranch_contest_rank_for("2026-10-19", expected_score)
    );
    assert!(matches!(
        game.dispatch(Action::EnterRanchContest { pen: 1 }, 0),
        Err(GameError::RanchContestEntered)
    ));

    // 下一周重置报名，奖章保留。
    let medals = game.state().ranch_contest.medals;
    game.sync(0, "2026-10-26");
    let contest = game.state().ranch_contest.clone();
    assert!(!contest.entered);
    assert!(contest.result.is_none());
    assert_eq!(contest.medals, medals);
}

#[test]
fn ranch_contest_rejects_young_and_sick() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-19".into();
    game.state_mut().coins = 5_000;
    add_chicken(&mut game, 1);
    animal_mut(&mut game, 1).adult_at = RANCH_YOUNG_MS;
    assert!(matches!(
        game.dispatch(Action::EnterRanchContest { pen: 1 }, 0),
        Err(GameError::RanchAnimalYoung)
    ));
    animal_mut(&mut game, 1).adult_at = 0;
    animal_mut(&mut game, 1).sick = true;
    assert!(matches!(
        game.dispatch(Action::EnterRanchContest { pen: 1 }, 0),
        Err(GameError::RanchAnimalSick)
    ));
}
