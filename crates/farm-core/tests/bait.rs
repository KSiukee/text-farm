use farm_core::{
    bait_weight, pick_fish_with_bait, Action, BaitQuality, BaitRecipe, BaitType, Game, GameError,
    WorkshopJob,
};

fn give(game: &mut Game, key: &str, qty: u32) {
    game.state_mut().inventory.insert(key.into(), qty);
}

fn finish_job(game: &mut Game, slot: u8) {
    if let Some(Some(job)) = game.state_mut().workshop.job_mut(slot) {
        match job {
            WorkshopJob::Ranch { ready_at, .. }
            | WorkshopJob::CropBox { ready_at, .. }
            | WorkshopJob::Equipment { ready_at, .. }
            | WorkshopJob::Bait { ready_at, .. }
            | WorkshopJob::Fish { ready_at, .. } => *ready_at = 0,
        }
    }
}

#[test]
fn bait_stock_is_per_type() {
    let mut game = Game::new(0);
    let fishing = &mut game.state_mut().fishing;
    fishing.add_bait_of(BaitType::Corn, BaitQuality::Normal, 3);
    fishing.add_bait_of(BaitType::Worm, BaitQuality::Fine, 2);
    assert_eq!(
        fishing.bait_count_of(BaitType::Corn, BaitQuality::Normal),
        3
    );
    assert_eq!(fishing.bait_count_of(BaitType::Worm, BaitQuality::Fine), 2);
    assert_eq!(fishing.bait_count(BaitQuality::Normal), 10); // 面团初始 10
    assert!(fishing.take_bait_of(BaitType::Corn, BaitQuality::Normal));
    assert_eq!(
        fishing.bait_count_of(BaitType::Corn, BaitQuality::Normal),
        2
    );
    assert!(!fishing.take_bait_of(BaitType::Lure, BaitQuality::Master));
    assert_eq!(fishing.bait_count(BaitQuality::Normal), 10);
}

#[test]
fn bait_type_changes_fish_distribution() {
    let biased = |bait: BaitType| {
        (0..4_000u64)
            .filter(|roll| {
                let def = pick_fish_with_bait(farm_core::FishingSpot::River, *roll, bait);
                bait_weight(bait, def.id) > 100
            })
            .count()
    };
    let dough = biased(BaitType::Dough);
    let corn = biased(BaitType::Corn);
    let lure = biased(BaitType::Lure);
    assert_eq!(dough, 0, "面团不应该有偏好鱼");
    assert!(corn > 0, "玉米饵应该更容易出鲤鱼/草鱼等");
    assert!(lure > 0, "拟饵应该更容易出翘嘴/鳡鱼等");
    assert_ne!(corn, lure);
}

#[test]
fn craft_bait_consumes_inputs_and_adds_to_stock() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    give(&mut game, "crop:yumi", 2);
    game.dispatch(
        Action::StartBaitCraft {
            recipe: BaitRecipe::Corn,
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.state().workshop.job(0).unwrap().ready_at(),
        10 * 60_000
    );
    assert_eq!(game.state().inventory.get("crop:yumi"), None);
    finish_job(&mut game, 0);
    game.dispatch(Action::CollectCraft { slot: 0 }, 0).unwrap();
    assert_eq!(
        game.state()
            .fishing
            .bait_count_of(BaitType::Corn, BaitQuality::Normal),
        10
    );
    assert_eq!(game.state().skills.processing.exp, 18);
}

#[test]
fn craft_bait_fails_without_inputs() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    assert!(matches!(
        game.dispatch(
            Action::StartBaitCraft {
                recipe: BaitRecipe::Corn
            },
            0
        ),
        Err(GameError::RanchIngredientsShort)
    ));
}

#[test]
fn buy_and_cast_use_selected_type() {
    let mut game = Game::new(0);
    game.state_mut().coins = 10_000;
    game.dispatch(
        Action::SetBaitType {
            bait_type: BaitType::Corn,
        },
        0,
    )
    .unwrap();
    game.dispatch(
        Action::BuyBait {
            qty: 5,
            quality: BaitQuality::Normal,
            bait_type: BaitType::Corn,
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.state()
            .fishing
            .bait_count_of(BaitType::Corn, BaitQuality::Normal),
        5
    );
    assert_eq!(game.state().fishing.bait_count(BaitQuality::Normal), 10);

    game.dispatch(
        Action::CastLine {
            spot: farm_core::FishingSpot::Creek,
            quality: Some(BaitQuality::Normal),
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.state()
            .fishing
            .bait_count_of(BaitType::Corn, BaitQuality::Normal),
        4
    );
    assert_eq!(game.state().fishing.bait_count(BaitQuality::Normal), 10);
}

#[test]
fn auto_uses_configured_type() {
    let mut game = Game::new(0);
    game.state_mut()
        .fishing
        .add_bait_of(BaitType::Worm, BaitQuality::Normal, 5);
    game.state_mut().fishing.codex.insert(
        "white_bait".into(),
        farm_core::FishCodexEntry {
            count: 1,
            ..Default::default()
        },
    );
    game.dispatch(
        Action::SetAutoFishing {
            spot: farm_core::FishingSpot::Creek,
            enabled: true,
            rod: None,
            bait_quality: Some(BaitQuality::Normal),
            bait_type: Some(BaitType::Worm),
        },
        0,
    )
    .unwrap();
    game.sync(15 * 60_000, "2026-10-20");
    assert_eq!(game.state().fishing.stats.auto_catches, 1);
    assert_eq!(
        game.state()
            .fishing
            .bait_count_of(BaitType::Worm, BaitQuality::Normal),
        4
    );
}

#[test]
fn old_save_without_type_fields_defaults_to_dough() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    let state = value["state"].as_object_mut().unwrap();
    let fishing = state
        .get_mut("fishing")
        .and_then(|fishing| fishing.as_object_mut())
        .unwrap();
    for key in ["baitCorn", "baitWorm", "baitLure", "activeBaitType"] {
        fishing.remove(key);
    }
    if let Some(auto) = fishing.get_mut("auto").and_then(|a| a.as_object_mut()) {
        auto.remove("baitType");
    }
    let save: farm_core::SaveFile = serde_json::from_value(value).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state().fishing.active_bait_type, BaitType::Dough);
    assert_eq!(restored.state().fishing.auto.bait_type, BaitType::Dough);
    assert_eq!(restored.state().fishing.bait_corn, [0; 4]);
    assert_eq!(restored.state().fishing.bait_worm, [0; 4]);
    assert_eq!(restored.state().fishing.bait_lure, [0; 4]);
}
