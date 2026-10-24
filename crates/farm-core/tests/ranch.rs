use farm_core::{
    migrate_save, Action, EquipmentKind, EquipmentRecipe, Game, GameError, PetBreed, PetLocation,
    PetState, RanchAnimal, RanchAnimalKind, RanchRecipe, RanchTrait, Season, WorkshopJob,
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
    game.dispatch(Action::BreedRanch, 0).unwrap();
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
    {
        let animal = game
            .state_mut()
            .ranch
            .pen_mut(1)
            .unwrap()
            .animal
            .as_mut()
            .unwrap();
        animal.quality = 2;
        animal.product_ready_at = 0;
    }
    game.dispatch(Action::CollectRanchProduct { pen: 1 }, 0)
        .unwrap();
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

    {
        let animal = game
            .state_mut()
            .ranch
            .pen_mut(1)
            .unwrap()
            .animal
            .as_mut()
            .unwrap();
        animal.quality = 3;
        animal.product_ready_at = 0;
    }
    game.dispatch(Action::CollectRanchProduct { pen: 1 }, 0)
        .unwrap();
    assert_eq!(game.state().inventory.get("ranch:egg.prime"), Some(&1));
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

#[test]
fn daily_care_once_a_day_and_gentle_gains_more() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    game.state_mut().daily.day = "2026-10-24".into();
    add_chicken(&mut game, 1);
    {
        let animal = animal_mut(&mut game, 1);
        animal.temper = RanchTrait::Gentle;
        animal.affection = 30;
        animal.cleanliness = 60;
        animal.satiety = 60;
    }
    game.dispatch(Action::GroomRanchAnimal { pen: 1 }, 0)
        .unwrap();
    {
        let animal = game.state().ranch.pen(1).unwrap().animal.as_ref().unwrap();
        assert_eq!(animal.affection, 33); // 温顺：2 × 1.5
        assert_eq!(animal.cleanliness, 100);
    }
    assert!(matches!(
        game.dispatch(Action::GroomRanchAnimal { pen: 1 }, 0),
        Err(GameError::RanchCareDone)
    ));
    game.state_mut().daily.day = "2026-10-25".into();
    game.dispatch(Action::GroomRanchAnimal { pen: 1 }, 0)
        .unwrap();
    assert_eq!(
        game.state()
            .ranch
            .pen(1)
            .unwrap()
            .animal
            .as_ref()
            .unwrap()
            .affection,
        36
    );

    // 放牧：冬天收益减半；温顺的亲密 +50%。
    game.state_mut().season = Season::Winter;
    let before = game
        .state()
        .ranch
        .pen(1)
        .unwrap()
        .animal
        .as_ref()
        .unwrap()
        .satiety;
    game.dispatch(Action::GrazeRanchAnimal { pen: 1 }, 0)
        .unwrap();
    let animal = game.state().ranch.pen(1).unwrap().animal.as_ref().unwrap();
    assert_eq!(animal.satiety, before + 7);
    assert_eq!(animal.affection, 39); // 冬天基础 +2，温顺 → +3

    // 互动：基础 +5，温顺 → +7。
    game.dispatch(Action::PetRanchAnimal { pen: 1 }, 0)
        .unwrap();
    assert_eq!(
        game.state()
            .ranch
            .pen(1)
            .unwrap()
            .animal
            .as_ref()
            .unwrap()
            .affection,
        46
    );
}

#[test]
fn traits_change_feed_and_product_interval() {
    let mut animal = RanchAnimal::new(RanchAnimalKind::Chicken, 0);
    animal.temper = RanchTrait::Picky;
    animal.satiety = 50;
    animal.feed();
    assert_eq!(animal.satiety, 74); // 普通喂食 30 × 0.8

    animal.affection = 30;
    animal.feed_premium();
    assert_eq!(animal.quality, 2);
    assert_eq!(animal.affection, 38); // 挑食更喜欢专用饲料：+8

    let mut fast = RanchAnimal::new(RanchAnimalKind::Cow, 0);
    fast.temper = RanchTrait::HighYield;
    assert_eq!(fast.product_interval(), 16 * 60 * 60 * 1000 * 9 / 10);
    fast.product_ready_at = 0;
    assert!(fast.collect(1_000));
    assert_eq!(
        fast.product_ready_at,
        1_000 + 16 * 60 * 60 * 1000 * 9 / 10
    );
}

#[test]
fn care_raises_quality_and_unlocks_special_product() {
    let mut game = Game::new(0);
    game.state_mut().coins = 5_000;
    add_chicken(&mut game, 1);
    {
        let animal = animal_mut(&mut game, 1);
        animal.quality = 1;
        animal.affection = 100;
        animal.cleanliness = 100;
    }
    for step in 0..300i64 {
        animal_mut(&mut game, 1).product_ready_at = 0;
        game.dispatch(
            Action::CollectRanchProduct { pen: 1 },
            step * 86_400_000,
        )
        .unwrap();
    }
    let mut fine = 0u32;
    let mut special = 0u32;
    for (key, qty) in &game.state().inventory {
        if key.ends_with(".fine") || key.ends_with(".prime") {
            fine += *qty;
        }
        if key == "ranch:golden_egg" {
            special += *qty;
        }
    }
    assert!(fine > 0, "照料到位时应该能出优良产物");
    assert!(special > 0, "亲密度 100 时应有机会出金蛋");
}

#[test]
fn cleanliness_decays_and_dirty_lowers_quality_chance() {
    let mut animal = RanchAnimal::new(RanchAnimalKind::Chicken, 0);
    animal.affection = 100;
    let clean = animal.care_quality_chance(false);
    animal.tick(24 * 3_600_000, true);
    assert_eq!(animal.cleanliness, 76);
    animal.cleanliness = 20;
    assert!(animal.care_quality_chance(false) < clean);
    assert_eq!(animal.mood_label(), "有点脏");
}

#[test]
fn tick_accumulates_sub_hour_syncs() {
    let mut animal = RanchAnimal::new(RanchAnimalKind::Chicken, 0);
    // 壳每 30 秒 sync 一次：不满一小时的零头不能把 last_tick_at 推走。
    animal.tick(30 * 60_000, true);
    assert_eq!(animal.satiety, 80);
    assert_eq!(animal.last_tick_at, 0);
    animal.tick(60 * 60_000, true);
    assert_eq!(animal.satiety, 79);
    assert_eq!(animal.last_tick_at, 60 * 60_000);
    animal.tick(90 * 60_000, true);
    assert_eq!(animal.satiety, 79);
    animal.tick(120 * 60_000, true);
    assert_eq!(animal.satiety, 78);
}

#[test]
fn old_save_animals_get_traits_and_defaults() {
    let value = serde_json::json!({
        "schema": 35,
        "savedAt": 0,
        "state": {"ranch": {"pens": [
            {"index": 1, "animal": {"kind": "chicken", "name": "小母鸡"}},
            {"index": 2, "animal": null}
        ]}},
        "content": {}
    });
    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], 36);
    let animal = &migrated["state"]["ranch"]["pens"][0]["animal"];
    assert_eq!(animal["affection"], 40);
    assert_eq!(animal["cleanliness"], 80);
    let trait_name = animal["trait"].as_str().unwrap().to_string();
    assert!(
        ["high_yield", "gentle", "picky", "delicate", "homebody"].contains(&trait_name.as_str()),
        "未知性格：{trait_name}"
    );
    // 再跑一次（schema 已是 36）不该改性格。
    let again = migrate_save(migrated.clone()).unwrap();
    assert_eq!(
        again["state"]["ranch"]["pens"][0]["animal"]["trait"],
        trait_name
    );

    // 直接反序列化旧动物：缺字段也能拿到默认性格/亲密度。
    let old: RanchAnimal =
        serde_json::from_value(serde_json::json!({"kind": "chicken", "name": "老母鸡"})).unwrap();
    assert_eq!(old.temper, RanchTrait::Gentle);
    assert_eq!(old.affection, 30);
    assert_eq!(old.cleanliness, 100);
}
