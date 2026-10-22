use farm_core::{
    economy_preview, migrate_save, Action, ContentPack, FacilityKind, Game, GameError, SaveFile,
};
use serde_json::json;

const MIN: i64 = 60_000;

fn game_with(level: u32, coins: u64, seeds: &[(&str, u32)]) -> Game {
    let mut state = Game::new(0).state().clone();
    state.level = level;
    state.coins = coins;
    state.inventory.clear();
    for (crop, qty) in seeds {
        state.inventory.insert(format!("seed:{crop}"), *qty);
    }
    Game::from_parts(state, ContentPack::starter())
}

#[test]
fn v1_save_migrates_to_v25_with_defaults() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(1);
    let state = value["state"].as_object_mut().unwrap();
    for key in [
        "facilities",
        "settings",
        "stats",
        "lastTickAt",
        "daily",
        "codex",
        "achievements",
        "npcs",
        "npcMessages",
        "lastNpcTickAt",
    ] {
        state.remove(key);
    }
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(crop) = plot.get_mut("crop").and_then(|c| c.as_object_mut()) {
                crop.remove("pest");
            }
        }
    }
    state.insert("schema".into(), json!(1));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["schema"], json!(34));
    assert_eq!(migrated["state"]["settings"]["expRate"], json!(1));
    assert_eq!(migrated["state"]["daily"]["day"], json!(""));
    assert_eq!(migrated["state"]["daily"]["steals"], json!(0));

    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state().level, 1);
    assert!(!restored.state().facilities.harvester);
    assert!(!restored.state().settings.onboarded);
    assert_eq!(restored.state().stats.harvests, 0);
    assert!(restored.state().codex.is_empty());
    assert!(restored.state().npcs.is_empty());
}

#[test]
fn v2_save_migrates_to_v25_with_pest_null_and_daily() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(2);
    let state = value["state"].as_object_mut().unwrap();
    for key in [
        "lastTickAt",
        "daily",
        "codex",
        "achievements",
        "npcs",
        "npcMessages",
        "lastNpcTickAt",
    ] {
        state.remove(key);
    }
    if let Some(stats) = state.get_mut("stats").and_then(|s| s.as_object_mut()) {
        for key in ["pestsCleared", "itemsSold", "signIns", "plotsUnlocked"] {
            stats.remove(key);
        }
    }
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(crop) = plot.get_mut("crop").and_then(|c| c.as_object_mut()) {
                crop.remove("pest");
            }
        }
    }
    state.insert("schema".into(), json!(2));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["stats"]["pestsCleared"], json!(0));
    assert_eq!(migrated["state"]["daily"]["harvests"], json!(0));

    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state().stats.plots_unlocked, 4);
}

#[test]
fn v4_save_migrates_to_v25_with_plot_tiers() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(4);
    let state = value["state"].as_object_mut().unwrap();
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(plot) = plot.as_object_mut() {
                plot.remove("tier");
            }
        }
    }
    state.insert("schema".into(), json!(4));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["plots"][0]["tier"], json!(0));
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    assert_eq!(Game::from_save(save).unwrap().state().plots[0].tier, 0);
}

#[test]
fn v5_save_migrates_to_v25_with_disaster_null() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(5);
    let state = value["state"].as_object_mut().unwrap();
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(crop) = plot.get_mut("crop").and_then(|c| c.as_object_mut()) {
                crop.remove("disaster");
            }
        }
    }
    state.insert("schema".into(), json!(5));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert!(restored.state().plots.iter().all(|plot| plot
        .crop
        .as_ref()
        .map(|crop| crop.disaster.is_none())
        .unwrap_or(true)));
}

#[test]
fn v6_save_migrates_to_v25_with_season_and_off_season() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(6);
    let state = value["state"].as_object_mut().unwrap();
    state.remove("season");
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(crop) = plot.get_mut("crop").and_then(|c| c.as_object_mut()) {
                crop.remove("offSeason");
            }
        }
    }
    state.insert("schema".into(), json!(6));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["season"], json!("spring"));
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    assert_eq!(
        Game::from_save(save).unwrap().state().season,
        farm_core::Season::Spring
    );
}

#[test]
fn v7_save_migrates_to_v25_with_region_and_greenhouse() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(7);
    let state = value["state"].as_object_mut().unwrap();
    if let Some(settings) = state.get_mut("settings").and_then(|s| s.as_object_mut()) {
        settings.remove("region");
    }
    if let Some(facilities) = state.get_mut("facilities").and_then(|f| f.as_object_mut()) {
        facilities.remove("greenhouse");
    }
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(crop) = plot.get_mut("crop").and_then(|c| c.as_object_mut()) {
                crop.remove("growthFactor");
                crop.remove("greenhouse");
            }
        }
    }
    state.insert("schema".into(), json!(7));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["settings"]["region"], json!("north"));
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state().settings.region, farm_core::Region::North);
    assert!(!restored.state().facilities.greenhouse);
}

#[test]
fn v22_save_migrates_to_v25_with_ranch() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(22);
    let state = value["state"].as_object_mut().unwrap();
    state.remove("ranch");
    state.insert("schema".into(), json!(22));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert!(migrated["state"]["ranch"]["pens"].is_array());
}

#[test]
fn v24_save_migrates_to_v25_with_workshop() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(24);
    let state = value["state"].as_object_mut().unwrap();
    state.remove("workshop");
    if let Some(ranch) = state.get_mut("ranch").and_then(|r| r.as_object_mut()) {
        ranch.insert("workshop".into(), json!(true));
        ranch.insert(
            "craft".into(),
            json!({
                "recipe": "cheese",
                "startedAt": 0,
                "readyAt": 100,
                "quality": 2
            }),
        );
    }
    state.insert("schema".into(), json!(24));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["workshop"]["built"], json!(true));
    assert_eq!(
        migrated["state"]["workshop"]["equipment"][0],
        json!("manual")
    );
    assert_eq!(
        migrated["state"]["workshop"]["jobs"][0]["kind"],
        json!("ranch")
    );
    assert_eq!(
        migrated["state"]["workshop"]["jobs"][0]["recipe"],
        json!("cheese")
    );
    assert!(migrated["state"]["ranch"].get("workshop").is_none());
    assert!(migrated["state"]["ranch"].get("craft").is_none());
}

#[test]
fn v21_save_migrates_to_v25_with_activity_streak() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(21);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pets".into(),
        json!([{"id":"pet_1","name":"豆豆","species":"dog","breed":"corgi","boarded":false}]),
    );
    state.insert("schema".into(), json!(21));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["pets"][0]["activityStreak"], json!(0));
    assert!(migrated["state"]["pets"][0]["lastActivity"].is_null());
}

#[test]
fn v20_save_migrates_to_v25_with_activity_day() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(20);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pets".into(),
        json!([{"id":"pet_1","name":"豆豆","species":"dog","breed":"corgi","boarded":false}]),
    );
    state.insert("schema".into(), json!(20));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["pets"][0]["lastActivityDay"], json!(""));
}

#[test]
fn v19_save_migrates_to_v25_with_skill_clock() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(19);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pets".into(),
        json!([{"id":"pet_1","name":"豆豆","species":"dog","breed":"corgi","boarded":false}]),
    );
    state.insert("schema".into(), json!(19));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["pets"][0]["lastSkillAt"], json!(0));
}

#[test]
fn v18_save_migrates_to_v25_with_pet_personality() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(18);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pets".into(),
        json!([{"id":"pet_1","name":"豆豆","species":"dog","breed":"corgi","boarded":false}]),
    );
    state.insert("schema".into(), json!(18));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert!(migrated["state"]["pets"][0]["personality"]["primary"].is_string());
    assert!(migrated["state"]["pets"][0]["personality"]["quirk"].is_string());
}

#[test]
fn v16_save_migrates_to_v25_with_split_care() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(16);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "petCare".into(),
        json!({"bed":true,"bowl":true,"toy":true,"medicine":2}),
    );
    state.insert("schema".into(), json!(16));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["petCare"]["dogBed"], json!(true));
    assert_eq!(migrated["state"]["petCare"]["catToy"], json!(true));
    assert_eq!(migrated["state"]["petCare"]["bowls"], json!(3));
}

#[test]
fn v15_save_migrates_to_v25_with_pet_care() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(15);
    let state = value["state"].as_object_mut().unwrap();
    state.remove("petCare");
    state.insert("schema".into(), json!(15));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["petCare"]["dogBed"], json!(false));
    assert_eq!(migrated["state"]["petCare"]["medicine"], json!(0));
}

#[test]
fn v14_save_migrates_to_v25_with_skills() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(14);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pets".into(),
        json!([{"id":"pet_1","name":"豆豆","species":"dog","breed":"corgi","boarded":false}]),
    );
    state.insert("schema".into(), json!(14));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["pets"][0]["digSkill"], json!(1));
    assert_eq!(migrated["state"]["pets"][0]["guardSkill"], json!(1));
}

#[test]
fn v13_save_migrates_to_v25_with_pet_array() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(13);
    let state = value["state"].as_object_mut().unwrap();
    state.remove("pets");
    state.insert(
        "pet".into(),
        json!({"name":"豆豆","species":"dog","breed":"corgi","adoptedAt":0}),
    );
    state.insert("schema".into(), json!(13));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["pets"].as_array().unwrap().len(), 1);
    assert_eq!(migrated["state"]["pets"][0]["id"], json!("pet_1"));
    assert_eq!(migrated["state"]["pets"][0]["boarded"], json!(false));
}

#[test]
fn v12_save_migrates_to_v25_with_breed() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(12);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pet".into(),
        json!({"name":"豆豆","species":"cat","adoptedAt":0}),
    );
    state.insert("schema".into(), json!(12));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["pets"][0]["breed"], json!("domestic_cat"));
}

#[test]
fn v11_save_migrates_to_v25_with_empty_pet() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(11);
    let state = value["state"].as_object_mut().unwrap();
    state.remove("pet");
    state.remove("lastRaidDay");
    state.insert("schema".into(), json!(11));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert!(migrated["state"]["pet"].is_null());
    assert_eq!(migrated["state"]["lastRaidDay"], json!(""));
}

#[test]
fn v10_save_migrates_to_v25_with_facility_counts() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(10);
    let state = value["state"].as_object_mut().unwrap();
    if let Some(facilities) = state.get_mut("facilities").and_then(|f| f.as_object_mut()) {
        facilities.remove("windmills");
        facilities.remove("damaged_windmills");
        facilities.remove("greenhouses");
        facilities.insert("windmill".into(), json!(true));
        facilities.insert("greenhouse".into(), json!(true));
    }
    state.insert("schema".into(), json!(10));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["facilities"]["windmills"], json!(1));
    assert_eq!(migrated["state"]["facilities"]["greenhouses"], json!(1));
}

#[test]
fn v9_save_migrates_to_v25_with_plot_kind() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(9);
    let state = value["state"].as_object_mut().unwrap();
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(object) = plot.as_object_mut() {
                object.remove("kind");
            }
        }
    }
    state.insert("schema".into(), json!(9));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["plots"][0]["kind"], json!("field"));
}

#[test]
fn v8_save_migrates_to_v25_with_withered_false() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(8);
    let state = value["state"].as_object_mut().unwrap();
    if let Some(plots) = state.get_mut("plots").and_then(|p| p.as_array_mut()) {
        for plot in plots {
            if let Some(crop) = plot.get_mut("crop").and_then(|c| c.as_object_mut()) {
                crop.remove("withered");
            }
        }
    }
    state.insert("schema".into(), json!(8));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    assert!(Game::from_save(save)
        .unwrap()
        .state()
        .plots
        .iter()
        .all(|plot| plot
            .crop
            .as_ref()
            .map(|crop| !crop.withered)
            .unwrap_or(true)));
}

#[test]
fn v3_save_migrates_to_v25_with_empty_neighbors() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(3);
    let state = value["state"].as_object_mut().unwrap();
    for key in ["npcs", "npcMessages", "lastNpcTickAt"] {
        state.remove(key);
    }
    if let Some(daily) = state.get_mut("daily").and_then(|d| d.as_object_mut()) {
        daily.remove("steals");
        daily.remove("helps");
    }
    state.insert("schema".into(), json!(3));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["npcs"], json!([]));
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    assert!(Game::from_save(save).unwrap().state().npcs.is_empty());
}

#[test]
fn v32_save_migrates_to_v33_with_fishing_job() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(32);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pets".into(),
        json!([{"id":"pet_1","name":"豆豆","species":"dog","breed":"corgi","boarded":false}]),
    );
    state.insert("schema".into(), json!(32));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(
        migrated["state"]["pets"][0]["fishingJob"],
        json!({"level": 1, "exp": 0})
    );
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state().pets[0].fishing_job.level, 1);
}

#[test]
fn v33_save_migrates_to_v34_with_pet_talents() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(33);
    let state = value["state"].as_object_mut().unwrap();
    state.insert(
        "pets".into(),
        json!([{"id":"pet_1","name":"豆豆","species":"dog","breed":"corgi","boarded":false}]),
    );
    state.insert("schema".into(), json!(33));

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(
        migrated["state"]["pets"][0]["talents"],
        json!({"tier1": null, "tier2": null, "tier3": null})
    );
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert!(restored.state().pets[0].talents.tier1.is_none());
}

#[test]
fn facilities_apply_to_new_plantings_and_enable_harvest_all() {
    let mut game = game_with(15, 100_000, &[("xiaobaicai", 1)]);
    for kind in [
        FacilityKind::Sprinkler,
        FacilityKind::Windmill,
        FacilityKind::Harvester,
    ] {
        game.dispatch(Action::BuyFacility { kind }, 0).unwrap();
    }

    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();

    let crop = game.snapshot(0).plots[0].crop.clone().unwrap();
    assert!(crop.watered);
    assert!(crop.ready_at < 3 * MIN);
    assert_eq!(
        game.snapshot(0)
            .facilities
            .iter()
            .filter(|facility| facility.owned)
            .count(),
        3
    );
    assert!(!game.state().facilities.greenhouse);

    game.dispatch(Action::HarvestAll, 3 * MIN).unwrap();
    assert!(game.snapshot(3 * MIN).plots[0].crop.is_none());
}

#[test]
fn sprinkler_watering_counts_toward_daily_task() {
    let mut game = game_with(15, 100_000, &[("xiaobaicai", 5)]);
    for plot in &mut game.state_mut().plots {
        plot.unlocked = true;
    }
    game.dispatch(
        Action::BuyFacility {
            kind: FacilityKind::Sprinkler,
        },
        0,
    )
    .unwrap();
    for plot in 1..=5 {
        game.dispatch(
            Action::Plant {
                plot,
                crop_id: "xiaobaicai".into(),
            },
            0,
        )
        .unwrap();
    }

    // 灌溉机自动浇的水也记进今日浇水，否则日常「浇水 5 次」永远卡住。
    assert_eq!(game.state().daily.waterings, 5);
    let task = game
        .snapshot(0)
        .daily
        .tasks
        .into_iter()
        .find(|task| task.id == 2)
        .unwrap();
    assert_eq!(task.progress, 5);
    assert!(task.claimable);

    let coins = game.state().coins;
    game.dispatch(Action::ClaimDaily { id: 2 }, 0).unwrap();
    assert!(game.state().coins > coins);
}

#[test]
fn harvest_all_requires_the_harvester() {
    let mut game = game_with(1, 200, &[("xiaobaicai", 1)]);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.dispatch(Action::HarvestAll, 3 * MIN),
        Err(GameError::NeedHarvester)
    ));
}

#[test]
fn content_cannot_remove_a_crop_in_use() {
    let mut game = Game::new(0);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();

    let mut pack = ContentPack::starter();
    pack.crops.retain(|c| c.id != "xiaobaicai");
    assert!(matches!(
        game.apply_content(pack, 0),
        Err(GameError::ContentRemovesUsedCrop(id)) if id == "xiaobaicai"
    ));
}

#[test]
fn economy_preview_covers_every_crop() {
    let pack = ContentPack::starter();
    let preview = economy_preview(&pack);
    assert_eq!(preview.len(), pack.crops.len());
    assert!(preview.iter().all(|p| p.coins_per_hour > 0));
    assert!(preview.iter().all(|p| p.exp_per_hour > 0));
    assert!(preview.iter().all(|p| p.payback_minutes > 0));
}

#[test]
fn dismiss_guide_sets_the_flag() {
    let mut game = Game::new(0);
    assert!(!game.snapshot(0).onboarded);
    game.dispatch(Action::DismissGuide, 0).unwrap();
    assert!(game.snapshot(0).onboarded);
}
