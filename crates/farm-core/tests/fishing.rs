use farm_core::{
    fish, fish_list, params, pick_personality, price_for, quality_for, species_tuning, spot_fish,
    weight_tenths_for, Action, FishCodexEntry, FishPersonality, FishRarity, FishingSpot, Game,
    GameError, RodKind, FISH, SAVE_SCHEMA,
};

fn game(now: i64) -> Game {
    Game::new(now)
}

#[test]
fn outlier_fish_are_tuned_down() {
    let bronze = params(
        RodKind::Wood,
        fish("bronze_carp").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    let yellow = params(
        RodKind::Wood,
        fish("yellowcheek").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    let king = params(
        RodKind::Wood,
        fish("jiang_king").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    // 鳡鱼难度更高，但速度/转向已被单独压到不比普通江鱼更疯。
    assert!(yellow.fish_speed_pct < bronze.fish_speed_pct);
    assert!(yellow.turn_pct < bronze.turn_pct);
    assert!(yellow.fish_escape_pct <= 170);
    // 江王仍最快，但不再无限叠乘。
    assert!(king.fish_speed_pct > yellow.fish_speed_pct);
    assert!(king.fish_speed_pct < 45);
    assert_eq!(species_tuning("bronze_carp"), (100, 100, 100, 100));
}

#[test]
fn fish_table_is_complete() {
    assert_eq!(fish_list().len(), 35);
    let mut ids = std::collections::BTreeSet::new();
    for def in fish_list() {
        assert!(ids.insert(def.id), "duplicate fish id {}", def.id);
        assert!(def.price > 0);
        assert!((1..=5).contains(&def.difficulty));
    }
    for spot in FishingSpot::ALL {
        let total: u32 = spot_fish(spot).map(|def| def.weight).sum();
        assert_eq!(total, 1000, "{} weights", spot.label());
        assert_eq!(
            spot_fish(spot).count(),
            if spot == FishingSpot::Sea { 8 } else { 9 }
        );
    }
}

#[test]
fn cast_and_resolve_adds_fish() {
    let mut game = game(1_000);
    let before = game.state().fishing.bait;
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        1_000,
    )
    .unwrap();
    assert_eq!(game.state().fishing.bait, before - 1);
    assert!(game.state().fishing.session.is_some());
    game.dispatch(Action::ResolveFishing { success: true }, 2_000)
        .unwrap();
    assert!(game.state().fishing.session.is_none());
    assert_eq!(game.state().fishing.stats.catches, 1);
    assert_eq!(game.state().fishing.stats.fails, 0);
    assert_eq!(game.state().fishing.codex.len(), 1);
    let total: u32 = game
        .state()
        .inventory
        .iter()
        .filter(|(key, _)| key.starts_with("fish:"))
        .map(|(_, qty)| *qty)
        .sum();
    assert_eq!(total, 1);
}

#[test]
fn failed_cast_does_not_add_fish() {
    let mut game = game(1_000);
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        1_000,
    )
    .unwrap();
    game.dispatch(Action::ResolveFishing { success: false }, 2_000)
        .unwrap();
    assert_eq!(game.state().fishing.stats.catches, 0);
    assert_eq!(game.state().fishing.stats.fails, 1);
    assert!(game.state().fishing.codex.is_empty());
    assert!(!game
        .state()
        .inventory
        .keys()
        .any(|key| key.starts_with("fish:")));
}

#[test]
fn spot_and_rod_levels_are_enforced() {
    let mut game = game(1_000);
    assert!(game
        .dispatch(
            Action::CastLine {
                spot: FishingSpot::River,
                quality: None,
            },
            1_000
        )
        .is_err());
    game.state_mut().skills.fishing.level = 5;
    assert!(game
        .dispatch(
            Action::BuyRod {
                rod: RodKind::Bamboo
            },
            1_000
        )
        .is_err()); // 金币不够
    game.state_mut().coins = 10_000;
    game.dispatch(
        Action::BuyRod {
            rod: RodKind::Bamboo,
        },
        1_000,
    )
    .unwrap();
    assert_eq!(game.state().fishing.rod, RodKind::Bamboo);
}

#[test]
fn rod_and_sea_unlock_levels_are_lowered() {
    assert_eq!(RodKind::Wood.unlock_level(), 1);
    assert_eq!(RodKind::Bamboo.unlock_level(), 4);
    assert_eq!(RodKind::Fiberglass.unlock_level(), 7);
    assert_eq!(RodKind::Carbon.unlock_level(), 9);
    assert_eq!(FishingSpot::Sea.unlock_level(), 15);
    assert!(!farm_core::rod_unlocked(3, RodKind::Bamboo));
    assert!(farm_core::rod_unlocked(4, RodKind::Bamboo));
    assert!(!farm_core::rod_unlocked(6, RodKind::Fiberglass));
    assert!(farm_core::rod_unlocked(7, RodKind::Fiberglass));
    assert!(!farm_core::rod_unlocked(8, RodKind::Carbon));
    assert!(farm_core::rod_unlocked(9, RodKind::Carbon));
    assert!(!farm_core::spot_unlocked(14, FishingSpot::Sea));
    assert!(farm_core::spot_unlocked(15, FishingSpot::Sea));
}

#[test]
fn auto_fishing_catches_one_per_interval() {
    let mut game = game(1_000);
    game.dispatch(
        Action::SetAutoFishing {
            spot: FishingSpot::Creek,
            enabled: true,
            rod: None,
            bait_quality: None,
            bait_type: None,
        },
        1_000,
    )
    .unwrap();
    assert!(game.state().fishing.auto.enabled);
    game.sync(1_000 + 15 * 60 * 1000, "2026-10-20");
    assert_eq!(game.state().fishing.stats.auto_catches, 1);
    assert_eq!(game.state().fishing.bait, 9);
    assert!(game.state().fishing.last_catch.is_none());
    assert_eq!(game.state().fishing.fish_batches.len(), 1);
    let total: u32 = game
        .state()
        .inventory
        .iter()
        .filter(|(key, _)| key.starts_with("fish:"))
        .map(|(_, qty)| *qty)
        .sum();
    assert_eq!(total, 1);
}

#[test]
fn fish_can_be_sold() {
    let mut game = game(1_000);
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        1_000,
    )
    .unwrap();
    game.dispatch(Action::ResolveFishing { success: true }, 2_000)
        .unwrap();
    let key = game
        .state()
        .inventory
        .keys()
        .find(|key| key.starts_with("fish:"))
        .cloned()
        .unwrap();
    let coins = game.state().coins;
    game.dispatch(Action::Sell { key, qty: 1 }, 3_000).unwrap();
    assert!(game.state().coins > coins);
}

#[test]
fn old_save_migrates_to_fishing() {
    let mut value = serde_json::to_value(game(1_000).to_save(1_000)).unwrap();
    value["schema"] = serde_json::json!(25);
    value["state"]["schema"] = serde_json::json!(25);
    value["state"].as_object_mut().unwrap().remove("fishing");
    let migrated = farm_core::migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], serde_json::json!(SAVE_SCHEMA));
    let save: farm_core::SaveFile = serde_json::from_value(migrated).unwrap();
    let game = Game::from_save(save).unwrap();
    assert_eq!(game.state().fishing.bait, 10);
}

#[test]
fn king_rarity_exists_in_each_spot() {
    for spot in FishingSpot::ALL {
        let kings = spot_fish(spot)
            .filter(|def| def.rarity == FishRarity::King)
            .count();
        assert_eq!(kings, 1, "{}", spot.label());
    }
}

#[test]
fn rod_changes_weight_and_quality() {
    let def = fish("crucian").unwrap();
    let wood_q = quality_for(RodKind::Wood, 0);
    let carbon_q = quality_for(RodKind::Carbon, 80);
    assert_eq!(wood_q, 1);
    assert_eq!(carbon_q, 3);
    let wood_w = weight_tenths_for(RodKind::Wood, def, 0, wood_q);
    let carbon_w = weight_tenths_for(RodKind::Carbon, def, 0, carbon_q);
    assert!(carbon_w > wood_w);
    assert!(price_for(def, 10, 3, 110) > price_for(def, 10, 1, 90));
}

#[test]
fn catch_records_weight_quality_and_batch() {
    let mut game = game(1_000);
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        1_000,
    )
    .unwrap();
    game.dispatch(Action::ResolveFishing { success: true }, 2_000)
        .unwrap();
    let catch = game.state().fishing.last_catch.as_ref().unwrap();
    assert!((8..=20).contains(&catch.weight_tenths));
    assert!((1..=3).contains(&catch.quality));
    assert!(catch.price > 0);
    assert!((90..=110).contains(&catch.market_pct));
    let batch = game
        .state()
        .fishing
        .fish_batches
        .get(&catch.fish_id)
        .unwrap();
    assert_eq!(batch.count, 1);
    assert_eq!(batch.unit_price(), catch.price);
}

#[test]
fn v26_fish_inventory_backfills_batches() {
    let mut game = game(1_000);
    game.state_mut()
        .inventory
        .insert("fish:white_bait".into(), 2);
    let mut value = serde_json::to_value(game.to_save(1_000)).unwrap();
    value["schema"] = serde_json::json!(26);
    value["state"]["schema"] = serde_json::json!(26);
    value["state"]["fishing"]
        .as_object_mut()
        .unwrap()
        .remove("fishBatches");
    let migrated = farm_core::migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], serde_json::json!(35));
    let save: farm_core::SaveFile = serde_json::from_value(migrated).unwrap();
    let game = Game::from_save(save).unwrap();
    let batch = game.state().fishing.fish_batches.get("white_bait").unwrap();
    assert_eq!(batch.count, 2);
    assert!(batch.total_value > 0);
}

#[test]
fn auto_bootstrap_only_unlocks_starter() {
    let mut game = game(1_000);
    game.dispatch(
        Action::SetAutoFishing {
            spot: FishingSpot::Creek,
            enabled: true,
            rod: None,
            bait_quality: None,
            bait_type: None,
        },
        1_000,
    )
    .unwrap();
    game.sync(1_000 + 15 * 60 * 1000, "2026-10-20");
    assert_eq!(game.state().fishing.stats.auto_catches, 1);
    assert_eq!(game.state().fishing.codex.len(), 1);
    assert_eq!(
        game.state().fishing.codex.get("white_bait").unwrap().count,
        1
    );
}

#[test]
fn auto_does_not_unlock_new_species() {
    let mut game = game(1_000);
    game.state_mut().skills.fishing.level = 5;
    game.state_mut().fishing.codex.insert(
        "white_bait".into(),
        FishCodexEntry {
            first_at: 0,
            count: 1,
            best_weight_tenths: 10,
            best_quality: 1,
            best_price: 10,
            ..Default::default()
        },
    );
    game.dispatch(
        Action::SetAutoFishing {
            spot: FishingSpot::River,
            enabled: true,
            rod: None,
            bait_quality: None,
            bait_type: None,
        },
        1_000,
    )
    .unwrap();
    game.sync(1_000 + 15 * 60 * 1000, "2026-10-20");
    assert_eq!(game.state().fishing.stats.auto_catches, 0);
    assert_eq!(game.state().fishing.codex.len(), 1);
    assert!(!game.state().fishing.auto.enabled);
}

#[test]
fn auto_buys_ten_bait_when_empty() {
    let mut game = game(1_000);
    game.state_mut().fishing.codex.insert(
        "white_bait".into(),
        FishCodexEntry {
            first_at: 0,
            count: 1,
            best_weight_tenths: 10,
            best_quality: 1,
            best_price: 10,
            ..Default::default()
        },
    );
    game.state_mut().fishing.bait = 0;
    game.dispatch(
        Action::SetAutoFishing {
            spot: FishingSpot::Creek,
            enabled: true,
            rod: None,
            bait_quality: None,
            bait_type: None,
        },
        1_000,
    )
    .unwrap();
    game.sync(1_000 + 15 * 60 * 1000, "2026-10-20");
    assert_eq!(game.state().fishing.stats.auto_catches, 1);
    assert_eq!(game.state().fishing.bait, 9);
    assert_eq!(game.state().coins, 100);
    let report = game.state().fishing.last_auto_report.as_ref().unwrap();
    assert_eq!(report.bait_bought, 10);
    assert_eq!(report.bait_used, 1);
    assert_eq!(report.coins_spent, 100);
    assert_eq!(report.catches.get("white_bait"), Some(&1));
}

#[test]
fn auto_can_use_special_bait() {
    let mut game = game(1_000);
    game.state_mut().fishing.codex.insert(
        "white_bait".into(),
        FishCodexEntry {
            first_at: 0,
            count: 1,
            best_weight_tenths: 10,
            best_quality: 1,
            best_price: 10,
            ..Default::default()
        },
    );
    game.state_mut().fishing.bait_master = 1;
    game.dispatch(
        Action::SetAutoFishing {
            spot: FishingSpot::Creek,
            enabled: true,
            rod: Some(RodKind::Wood),
            bait_quality: Some(farm_core::BaitQuality::Master),
            bait_type: None,
        },
        1_000,
    )
    .unwrap();
    game.sync(1_000 + 15 * 60 * 1000, "2026-10-20");
    assert_eq!(game.state().fishing.stats.auto_catches, 1);
    assert_eq!(game.state().fishing.bait_master, 0);
    assert!(game.state().fishing.last_auto_report.is_some());
}

#[test]
fn auto_action_deserializes_bait_quality() {
    let action: Action = serde_json::from_str(
        r#"{"type":"setAutoFishing","spot":"creek","enabled":true,"rod":"wood","baitQuality":"master"}"#,
    )
    .unwrap();
    match action {
        Action::SetAutoFishing { bait_quality, .. } => {
            assert_eq!(bait_quality, Some(farm_core::BaitQuality::Master));
        }
        _ => panic!("wrong action"),
    }
}

#[test]
fn manual_and_auto_codex_are_separate() {
    let mut game = game(1_000);
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        1_000,
    )
    .unwrap();
    game.dispatch(Action::ResolveFishing { success: true }, 2_000)
        .unwrap();
    let id = game.state().fishing.codex.keys().next().cloned().unwrap();
    let manual = game.state().fishing.codex.get(&id).unwrap().manual_count;
    assert_eq!(manual, 1);
    game.dispatch(
        Action::SetAutoFishing {
            spot: FishingSpot::Creek,
            enabled: true,
            rod: None,
            bait_quality: None,
            bait_type: None,
        },
        3_000,
    )
    .unwrap();
    game.sync(3_000 + 15 * 60 * 1000, "2026-10-20");
    let entry = game.state().fishing.codex.get(&id).unwrap();
    assert_eq!(entry.manual_count, 1);
    assert!(entry.auto_count >= 1);
    assert!(entry.auto_best_weight_tenths > 0);
}

#[test]
fn locked_fish_is_skipped_by_sell_all() {
    let mut game = game(1_000);
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        1_000,
    )
    .unwrap();
    game.dispatch(Action::ResolveFishing { success: true }, 2_000)
        .unwrap();
    let id = game.state().fishing.codex.keys().next().cloned().unwrap();
    game.dispatch(
        Action::ToggleFishLock {
            fish_id: id.clone(),
        },
        3_000,
    )
    .unwrap();
    assert!(game.dispatch(Action::SellAllFish, 3_000).is_err());
    assert!(game
        .dispatch(
            Action::Sell {
                key: format!("fish:{id}"),
                qty: 1
            },
            3_000
        )
        .is_err());
    game.dispatch(Action::ToggleFishLock { fish_id: id }, 4_000)
        .unwrap();
    game.dispatch(Action::SellAllFish, 4_000).unwrap();
    assert!(game
        .state()
        .inventory
        .keys()
        .all(|key| !key.starts_with("fish:")));
}

#[test]
fn rod_maintenance_clears_wear_and_restores_frame_speed() {
    let mut game = game(1_000);
    game.state_mut().coins = 1_000;
    game.state_mut().fishing.add_rod_wear(RodKind::Wood);
    for _ in 0..29 {
        game.state_mut().fishing.add_rod_wear(RodKind::Wood);
    }
    assert!(game.state().fishing.needs_maintenance(RodKind::Wood));
    let worn = farm_core::params(
        RodKind::Wood,
        fish("crucian").unwrap(),
        true,
        FishPersonality::Normal,
    );
    assert_eq!(worn.frame_speed_pct, 100);
    assert_eq!(worn.frame_accel_pct, 62);
    game.dispatch(Action::MaintainRod { rod: RodKind::Wood }, 1_000)
        .unwrap();
    assert!(!game.state().fishing.needs_maintenance(RodKind::Wood));
    assert_eq!(game.state().coins, 700);
}

#[test]
fn owned_rods_switch_and_maintenance_waits_for_session_end() {
    let mut game = game(1_000);
    game.state_mut().coins = 10_000;
    game.state_mut().skills.fishing.level = 5;
    game.dispatch(Action::BuyRod { rod: RodKind::Bamboo }, 1_000)
        .unwrap();
    assert_eq!(game.state().fishing.rod, RodKind::Bamboo);
    assert!(game.state().fishing.owns_rod(RodKind::Bamboo));

    game.dispatch(Action::SwitchRod { rod: RodKind::Wood }, 1_000)
        .unwrap();
    assert_eq!(game.state().fishing.rod, RodKind::Wood);
    game.dispatch(Action::SwitchRod { rod: RodKind::Bamboo }, 1_000)
        .unwrap();
    assert_eq!(game.state().fishing.rod, RodKind::Bamboo);
    assert!(game
        .dispatch(Action::SwitchRod { rod: RodKind::Carbon }, 1_000)
        .is_err());

    for _ in 0..30 {
        game.state_mut().fishing.add_rod_wear(RodKind::Bamboo);
    }
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        1_000,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(Action::MaintainRod { rod: RodKind::Bamboo }, 1_000),
        Err(GameError::FishingSessionActive)
    ));
    assert!(matches!(
        game.dispatch(Action::SwitchRod { rod: RodKind::Wood }, 1_000),
        Err(GameError::FishingSessionActive)
    ));
    game.dispatch(Action::ResolveFishing { success: false }, 2_000)
        .unwrap();
    game.dispatch(Action::MaintainRod { rod: RodKind::Bamboo }, 2_000)
        .unwrap();
    assert!(!game.state().fishing.needs_maintenance(RodKind::Bamboo));
}

#[test]
fn golden_session_gets_king_window() {
    let id = (1u64..1_000_000)
        .find(|id| {
            farm_core::golden_session(*id, FishingSpot::Creek, RodKind::Wood)
                && !farm_core::golden_session(*id, FishingSpot::River, RodKind::Wood)
        })
        .expect("golden session seed");
    assert!(farm_core::golden_session(
        id,
        FishingSpot::Creek,
        RodKind::Wood
    ));
    assert!(!farm_core::golden_session(
        id,
        FishingSpot::River,
        RodKind::Wood
    ));
    assert_eq!(farm_core::FISHING_GOLDEN_CATCH_MS, 20_000);
    assert_eq!(farm_core::FISHING_KING_CATCH_MS, 20_000);
}

#[test]
fn rod_economy_tiers_match_m3_a2_g1() {
    let rods = [
        RodKind::Wood,
        RodKind::Bamboo,
        RodKind::Fiberglass,
        RodKind::Carbon,
    ];
    assert_eq!(rods.map(|r| r.frame_pct()), [20, 18, 15, 13]);
    assert_eq!(rods.map(|r| r.durability()), [30, 40, 60, 90]);
    assert_eq!(
        rods.map(|r| r.maintenance_cost()),
        [300, 600, 1_200, 1_800]
    );
    assert_eq!(rods.map(|r| r.sell_bonus_pct()), [0, 10, 25, 40]);
    assert_eq!(rods.map(|r| r.bait_save_pct()), [0, 0, 10, 20]);
    assert_eq!(
        rods.map(|r| r.golden_bonus_permille()),
        [0, 5, 10, 20]
    );
    assert_eq!(rods.map(|r| r.exp_bonus_pct()), [0, 0, 40, 80]);
    assert_eq!(farm_core::rod_exp(10, RodKind::Wood), 10);
    assert_eq!(farm_core::rod_exp(10, RodKind::Fiberglass), 14);
    assert_eq!(farm_core::rod_exp(10, RodKind::Carbon), 18);
    assert_eq!(farm_core::king_base_price(FishingSpot::River), 800);
    assert_eq!(farm_core::golden_floor_price(FishingSpot::River), 400);
    assert_eq!(farm_core::sell_price(100, RodKind::Carbon), 140);

    let mut game = game(1_000);
    for _ in 0..39 {
        game.state_mut().fishing.add_rod_wear(RodKind::Bamboo);
    }
    assert!(!game.state().fishing.needs_maintenance(RodKind::Bamboo));
    game.state_mut().fishing.add_rod_wear(RodKind::Bamboo);
    assert!(game.state().fishing.needs_maintenance(RodKind::Bamboo));
}

#[test]
fn active_cast_uses_selected_bait_quality() {
    let mut game = game(1_000);
    game.state_mut().fishing.bait_fine = 1;
    game.state_mut().coins = 1_000;
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: Some(farm_core::BaitQuality::Fine),
        },
        1_000,
    )
    .unwrap();
    assert_eq!(game.state().fishing.bait_fine, 0);
    assert_eq!(
        game.state().fishing.session.as_ref().unwrap().bait_quality,
        farm_core::BaitQuality::Fine
    );
}

#[test]
fn rarer_fish_escape_a_bit_faster() {
    let common = farm_core::params(
        RodKind::Wood,
        fish("crucian").unwrap(),
        false,
        FishPersonality::Normal,
    );
    let king = farm_core::params(
        RodKind::Wood,
        fish("creek_king").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    assert_eq!(common.fish_escape_pct, 130);
    assert_eq!(king.fish_escape_pct, 170);
    assert!(king.fish_escape_pct < 200);
}

#[test]
fn king_is_always_fierce() {
    let king = fish("creek_king").unwrap();
    assert_eq!(
        pick_personality(king, 0),
        farm_core::FishPersonality::Fierce
    );
    assert_eq!(
        pick_personality(fish("crucian").unwrap(), 0),
        farm_core::FishPersonality::Lazy
    );
    assert_eq!(
        pick_personality(fish("crucian").unwrap(), 70),
        farm_core::FishPersonality::Active
    );
}

#[test]
fn personality_scale_rises_from_lazy_to_fierce() {
    let def = fish("crucian").unwrap();
    let personalities = [
        FishPersonality::Lazy,
        FishPersonality::Normal,
        FishPersonality::Active,
        FishPersonality::Fierce,
    ];
    let speeds: Vec<u32> = personalities
        .iter()
        .map(|p| farm_core::params(RodKind::Wood, def, false, *p).fish_speed_pct)
        .collect();
    let escapes: Vec<u32> = personalities
        .iter()
        .map(|p| farm_core::params(RodKind::Wood, def, false, *p).fish_escape_pct)
        .collect();
    assert!(
        speeds.windows(2).all(|pair| pair[0] < pair[1]),
        "{speeds:?}"
    );
    assert!(
        escapes.windows(2).all(|pair| pair[0] < pair[1]),
        "{escapes:?}"
    );
}

#[test]
fn king_is_faster_than_a_same_difficulty_fierce_fish() {
    let king = farm_core::params(
        RodKind::Wood,
        fish("creek_king").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    let fierce = farm_core::params(
        RodKind::Wood,
        fish("yellowcheek").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    assert!(king.fish_speed_pct > fierce.fish_speed_pct);
    assert!(king.fish_escape_pct > fierce.fish_escape_pct);
}

#[test]
fn catch_window_is_generous() {
    let normal = farm_core::params(
        RodKind::Wood,
        fish("crucian").unwrap(),
        false,
        FishPersonality::Normal,
    );
    assert_eq!(normal.catch_ms, 11_900);
    assert_eq!(normal.progress_loss, 12);
    assert_eq!(normal.frame_pct, 20);
    let king = farm_core::params(
        RodKind::Wood,
        fish("creek_king").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    assert_eq!(king.catch_ms, 20_000);
}

#[test]
fn fish_asset_orientation_list_is_fixed() {
    let right: Vec<&str> = FISH
        .iter()
        .filter(|def| farm_core::faces_right(def.id))
        .map(|def| def.id)
        .collect();
    assert_eq!(right.len(), 10, "{right:?}");
    assert!(farm_core::faces_right("hairtail"));
    assert!(farm_core::faces_right("large_yellow_croaker"));
    assert!(!farm_core::faces_right("crucian"));
}

#[test]
fn manual_catch_gets_quality_and_weight_bonus() {
    assert_eq!(farm_core::apply_manual_bonus(1, 10), (2, 11));
    assert_eq!(farm_core::apply_manual_bonus(2, 15), (3, 16));
    assert_eq!(farm_core::apply_manual_bonus(3, 20), (3, 21));
}

#[test]
fn auto_intervals_are_fifteen_thirteen_eleven_nine() {
    assert_eq!(RodKind::Wood.auto_interval_ms(), 15 * 60_000);
    assert_eq!(RodKind::Bamboo.auto_interval_ms(), 13 * 60_000);
    assert_eq!(RodKind::Fiberglass.auto_interval_ms(), 11 * 60_000);
    assert_eq!(RodKind::Carbon.auto_interval_ms(), 9 * 60_000);
}

#[test]
fn fishing_levels_and_grants_skill_points() {
    let mut game = game(1_000);
    let levels = game.state_mut().skills.fishing.add_exp(500);
    assert_eq!(levels, 2);
    assert_eq!(game.state().skills.fishing.level, 3);
    assert_eq!(game.state().skills.fishing.exp, 0);
    assert_eq!(game.state().skills.fishing.points, 2);
}

#[test]
fn fishing_skill_upgrade_costs_and_caps() {
    let mut game = game(1_000);
    game.state_mut().skills.fishing.points = 10;
    for expected in 1..=4 {
        game.dispatch(
            Action::UpgradeFishingSkill {
                skill: "reel".into(),
            },
            1_000,
        )
        .unwrap();
        assert_eq!(
            game.state()
                .skills
                .fishing
                .perks
                .get("reel")
                .copied()
                .unwrap_or(0),
            expected
        );
    }
    assert_eq!(game.state().skills.fishing.points, 0);
    assert!(matches!(
        game.dispatch(
            Action::UpgradeFishingSkill {
                skill: "reel".into()
            },
            1_000
        ),
        Err(GameError::FishingSkillMax)
    ));
    assert!(matches!(
        game.dispatch(
            Action::UpgradeFishingSkill {
                skill: "nope".into()
            },
            1_000
        ),
        Err(GameError::BadFishingSkill)
    ));
    game.state_mut().skills.fishing.points = 1;
    game.dispatch(
        Action::UpgradeFishingSkill {
            skill: "bait_saver".into(),
        },
        1_000,
    )
    .unwrap();
}

#[test]
fn fishing_skill_effects_scale() {
    assert_eq!(farm_core::bait_saver_pct(0), 0);
    assert_eq!(farm_core::bait_saver_pct(4), 32);
    assert_eq!(farm_core::reel_bonus(4), 8);
    assert_eq!(farm_core::big_fish_bonus_tenths(4), 4);
    assert_eq!(farm_core::skill_cost(0), 1);
    assert_eq!(farm_core::skill_cost(3), 4);
    assert_eq!(
        farm_core::auto_interval_ms_with_skill(RodKind::Wood, 4),
        15 * 60_000 * 84 / 100
    );
}

#[test]
fn catch_exp_scales_with_rarity_and_quality() {
    assert!(
        farm_core::catch_exp(FishRarity::King, 1, 10)
            > farm_core::catch_exp(FishRarity::Rare, 3, 20)
    );
    assert!(
        farm_core::catch_exp(FishRarity::Common, 3, 20)
            > farm_core::catch_exp(FishRarity::Common, 1, 10)
    );
    assert_eq!(farm_core::fishing_exp_need(29), 3_000);
    assert_eq!(farm_core::fishing_exp_need(30), 0);
}

#[test]
fn pet_fish_pool_only_returns_lit_unlocked_cheap_fish() {
    let mut fishing = farm_core::FishingState::new();
    for id in ["white_bait", "carp", "creek_king", "eel", "jiang_tuan"] {
        fishing.codex.insert(
            id.into(),
            FishCodexEntry {
                count: 1,
                ..Default::default()
            },
        );
    }

    // 1 级只解锁小溪；河王是鱼王，即使点亮也不进池。
    let pool = farm_core::pet_fish_pool(&fishing, 1, 70);
    assert_eq!(pool.len(), 1);
    assert_eq!(pool[0].id, "white_bait");
    assert_eq!(
        farm_core::pick_from_pool(&pool, 12_345).unwrap().id,
        "white_bait"
    );

    // 5 级解锁河流；江面的鳗鲡仍被钓点门槛挡住。
    let pool = farm_core::pet_fish_pool(&fishing, 5, 70);
    let ids: Vec<&str> = pool.iter().map(|def| def.id).collect();
    assert!(ids.contains(&"carp"));
    assert!(!ids.contains(&"eel"));
    assert!(!ids.contains(&"creek_king"));
    assert!(farm_core::pick_from_pool(&pool, 99).is_some());

    // 12 级解锁江面，但鳗鲡单价 160，超出默认 70 的上限；120 的江团正好能进慧眼池。
    let pool = farm_core::pet_fish_pool(&fishing, 12, 70);
    assert!(!pool.iter().any(|def| def.id == "eel"));
    assert!(!pool.iter().any(|def| def.id == "jiang_tuan"));
    let keen = farm_core::pet_fish_pool(&fishing, 12, 120);
    assert!(keen.iter().any(|def| def.id == "jiang_tuan"));
    assert!(!keen.iter().any(|def| def.id == "eel"));
    assert!(!keen.iter().any(|def| def.id == "creek_king"));
    assert!(farm_core::pick_from_pool(&[], 1).is_none());
}

#[test]
fn daily_commission_accept_progress_claim() {
    let mut game = game(0);
    game.state_mut().skills.fishing.level = 12;
    game.sync(0, "2026-10-23");
    let offers = game.state().fishing.quests.offers.clone();
    assert_eq!(offers.len(), 3);
    let offer = offers[0].clone();
    game.dispatch(
        Action::AcceptFishingCommission {
            id: offer.id.clone(),
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(
            Action::ClaimFishingCommission {
                id: offer.id.clone(),
            },
            0,
        ),
        Err(GameError::FishingCommissionNotReady)
    ));
    let def = offer
        .fish_id
        .as_deref()
        .and_then(fish)
        .or_else(|| offer.spot.and_then(|spot| spot_fish(spot).next()))
        .expect("commission has a fish");
    let weight = offer.min_weight_tenths.max(10);
    for _ in 0..offer.qty {
        game.state_mut().fishing.quests.on_catch(def, weight);
    }
    let coins = game.state().coins;
    let bait = game.state().fishing.bait;
    game.dispatch(
        Action::ClaimFishingCommission {
            id: offer.id.clone(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().coins > coins);
    assert!(game.state().fishing.bait > bait);
    assert!(game.state().fishing.quests.active.is_empty());

    // 同时最多接 2 条；先接满，再换一批拿新委托来试
    let more = game.state().fishing.quests.offers.clone();
    game.dispatch(
        Action::AcceptFishingCommission {
            id: more[0].id.clone(),
        },
        0,
    )
    .unwrap();
    game.dispatch(
        Action::AcceptFishingCommission {
            id: more[1].id.clone(),
        },
        0,
    )
    .unwrap();
    game.dispatch(Action::RefreshFishingCommissions, 0).unwrap();
    let fresh = game.state().fishing.quests.offers.clone();
    assert!(matches!(
        game.dispatch(
            Action::AcceptFishingCommission {
                id: fresh[0].id.clone(),
            },
            0,
        ),
        Err(GameError::FishingCommissionLimit)
    ));
}

#[test]
fn commission_refresh_once_per_day() {
    let mut game = game(0);
    game.state_mut().skills.fishing.level = 12;
    game.sync(0, "2026-10-23");
    game.dispatch(Action::RefreshFishingCommissions, 0).unwrap();
    assert!(matches!(
        game.dispatch(Action::RefreshFishingCommissions, 0),
        Err(GameError::FishingCommissionRefreshUsed)
    ));
    game.sync(1000, "2026-10-24");
    game.dispatch(Action::RefreshFishingCommissions, 1000)
        .unwrap();
}

#[test]
fn tournament_settles_once_and_claims_once() {
    let mut game = game(0);
    game.state_mut().skills.fishing.level = 12;
    game.sync(0, "2026-10-23");
    let def = fish("white_bait").expect("white bait");
    game.state_mut().fishing.quests.on_catch(def, 20);
    game.sync(1000, "2026-10-24");
    let results = game.state().fishing.quests.results.clone();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].best_tenths, 20);
    assert_eq!(results[0].rank, 1);
    let day = results[0].day.clone();
    let coins = game.state().coins;
    game.dispatch(
        Action::ClaimFishingTournament { day: day.clone() },
        1000,
    )
    .unwrap();
    assert!(game.state().coins > coins);
    assert!(matches!(
        game.dispatch(Action::ClaimFishingTournament { day }, 1000),
        Err(GameError::FishingTournamentInvalid)
    ));
    // 第二天再结算也只多一条，不会重复结算同一天
    game.sync(2000, "2026-10-25");
    assert_eq!(game.state().fishing.quests.results.len(), 1);
}

#[test]
fn condition_weights_change_distribution_without_banning() {
    use farm_core::{condition_hint, condition_weight, pick_fish_with_bait_condition, FishingCondition};
    use farm_core::Season;

    let catfish = fish("catfish").unwrap();
    let day = FishingCondition {
        season: Season::Summer,
        night: false,
        rainy: false,
    };
    let night = FishingCondition {
        season: Season::Summer,
        night: true,
        rainy: false,
    };
    assert_eq!(condition_weight(catfish, day), 100);
    assert_eq!(condition_weight(catfish, night), 200);
    assert!(condition_hint("catfish").contains("夜行"));
    assert!(condition_hint("eel").contains("雨天"));

    let mut day_catfish = 0;
    let mut night_catfish = 0;
    for roll in 0..5_000 {
        if pick_fish_with_bait_condition(FishingSpot::River, roll, farm_core::BaitType::Dough, day)
            .id
            == "catfish"
        {
            day_catfish += 1;
        }
        if pick_fish_with_bait_condition(FishingSpot::River, roll, farm_core::BaitType::Dough, night)
            .id
            == "catfish"
        {
            night_catfish += 1;
        }
    }
    assert!(
        night_catfish > day_catfish,
        "night={night_catfish} day={day_catfish}"
    );

    // 中立条件与旧接口完全一致（旧档/旧模拟不受影响）
    for roll in 0..2_000 {
        assert_eq!(
            farm_core::pick_fish_with_bait(FishingSpot::Creek, roll, farm_core::BaitType::Dough).id,
            pick_fish_with_bait_condition(
                FishingSpot::Creek,
                roll,
                farm_core::BaitType::Dough,
                FishingCondition::neutral(),
            )
            .id
        );
    }
}

#[test]
fn snapshot_exposes_current_condition_and_codex_hint() {
    let game = game(0);
    let snapshot = game.snapshot(0);
    assert!(snapshot.fishing.condition.summary.contains("·"));
    let eel = snapshot
        .fishing
        .codex
        .iter()
        .find(|item| item.id == "eel")
        .unwrap();
    assert!(eel.condition_hint.contains("夜行"));
}

#[test]
fn fish_behaviors_are_distinct_and_exposed_in_params() {
    use farm_core::{fish, fish_behavior, params, FishBehavior, FishPersonality, RodKind};

    assert_eq!(fish_behavior(fish("loach").unwrap()), FishBehavior::Bottom);
    assert_eq!(fish_behavior(fish("white_bait").unwrap()), FishBehavior::Surface);
    assert_eq!(fish_behavior(fish("carp").unwrap()), FishBehavior::Migratory);
    assert_eq!(fish_behavior(fish("black_carp").unwrap()), FishBehavior::Burst);

    let bottom = params(
        RodKind::Wood,
        fish("catfish").unwrap(),
        false,
        FishPersonality::Normal,
    );
    assert_eq!(bottom.behavior, FishBehavior::Bottom);
    assert_eq!(bottom.behavior_label, "贴底");
    assert_eq!(bottom.tension_gain_pct, 80);

    let burst = params(
        RodKind::Wood,
        fish("black_carp").unwrap(),
        false,
        FishPersonality::Fierce,
    );
    assert_eq!(burst.behavior, FishBehavior::Burst);
    assert_eq!(burst.behavior_label, "爆发");
    assert_eq!(burst.tension_gain_pct, 130);
}
