use farm_core::{
    migrate_save, Action, DisciplineKind, FishingSpot, Game, GameError, RodKind, SaveFile,
    SAVE_SCHEMA,
};
use serde_json::json;

fn new_game() -> Game {
    Game::new(0)
}

#[test]
fn new_game_has_four_disciplines_at_level_one() {
    let game = new_game();
    let skills = &game.state().skills;
    for kind in DisciplineKind::ALL {
        let d = skills.get(kind);
        assert_eq!(d.level, 1, "{kind:?}");
        assert_eq!(d.exp, 0, "{kind:?}");
        assert_eq!(d.points, 0, "{kind:?}");
        assert!(d.perks.is_empty(), "{kind:?}");
    }
    assert_eq!(SAVE_SCHEMA, 33);
}

#[test]
fn upgrade_skill_action_spends_points_and_caps() {
    let mut game = new_game();
    game.state_mut().skills.fishing.points = 10;
    for expected in 1..=4 {
        game.dispatch(
            Action::UpgradeSkill {
                discipline: DisciplineKind::Fishing,
                skill: "reel".into(),
            },
            0,
        )
        .unwrap();
        assert_eq!(
            game.state().skills.fishing.perks.get("reel").copied(),
            Some(expected)
        );
    }
    assert_eq!(game.state().skills.fishing.points, 0);
    assert!(matches!(
        game.dispatch(
            Action::UpgradeSkill {
                discipline: DisciplineKind::Fishing,
                skill: "reel".into(),
            },
            0
        ),
        Err(GameError::SkillMax)
    ));
    assert!(matches!(
        game.dispatch(
            Action::UpgradeSkill {
                discipline: DisciplineKind::Fishing,
                skill: "nope".into(),
            },
            0
        ),
        Err(GameError::BadSkill)
    ));

    assert!(matches!(
        game.dispatch(
            Action::UpgradeSkill {
                discipline: DisciplineKind::Fishing,
                skill: "bait_saver".into(),
            },
            0
        ),
        Err(GameError::NotEnoughSkillPoints { need: 1 })
    ));
}

#[test]
fn v31_save_migrates_fishing_progress_and_legacy_spots() {
    let mut game = new_game();
    game.state_mut().level = 20;
    game.state_mut().skills.fishing.level = 12;
    game.state_mut().skills.fishing.exp = 345;
    game.state_mut().skills.fishing.points = 7;
    game.state_mut()
        .skills
        .fishing
        .perks
        .insert("reel".into(), 2);

    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(31);
    let state = value["state"].as_object_mut().unwrap();
    state.insert("schema".into(), json!(31));

    // 伪造 v31 的形状：四项在 fishing 里，顶层没有 skills。
    let skills = state.remove("skills").unwrap();
    let fishing = state
        .get_mut("fishing")
        .and_then(|fishing| fishing.as_object_mut())
        .unwrap();
    fishing.insert("level".into(), skills["fishing"]["level"].clone());
    fishing.insert("exp".into(), skills["fishing"]["exp"].clone());
    fishing.insert("skillPoints".into(), skills["fishing"]["points"].clone());
    fishing.insert("skills".into(), skills["fishing"]["perks"].clone());
    fishing.remove("legacyUnlocks");

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(33));
    assert_eq!(migrated["state"]["skills"]["fishing"]["level"], json!(12));
    assert_eq!(migrated["state"]["skills"]["fishing"]["exp"], json!(345));
    assert_eq!(migrated["state"]["skills"]["fishing"]["points"], json!(7));
    assert_eq!(
        migrated["state"]["skills"]["fishing"]["perks"]["reel"],
        json!(2)
    );
    assert!(migrated["state"]["fishing"].get("level").is_none());
    assert!(migrated["state"]["fishing"].get("exp").is_none());
    assert!(migrated["state"]["fishing"].get("skillPoints").is_none());
    assert!(migrated["state"]["fishing"].get("skills").is_none());
    assert_eq!(
        migrated["state"]["fishing"]["legacyUnlocks"],
        json!(["creek", "river", "jiang", "sea"])
    );

    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state().skills.fishing.level, 12);
    assert_eq!(restored.state().skills.fishing.points, 7);
    assert_eq!(
        restored.state().skills.fishing.perks.get("reel").copied(),
        Some(2)
    );
    assert!(restored.state().fishing.legacy_unlocks.contains("sea"));
    assert_eq!(restored.state().skills.planting.level, 1);
    assert_eq!(restored.state().skills.ranching.level, 1);
    assert_eq!(restored.state().skills.processing.level, 1);
}

#[test]
fn v31_save_with_low_farm_level_keeps_only_unlocked_spots() {
    let mut game = new_game();
    game.state_mut().level = 5;
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(31);
    let state = value["state"].as_object_mut().unwrap();
    state.insert("schema".into(), json!(31));
    let skills = state.remove("skills").unwrap();
    let fishing = state
        .get_mut("fishing")
        .and_then(|fishing| fishing.as_object_mut())
        .unwrap();
    fishing.insert("level".into(), skills["fishing"]["level"].clone());
    fishing.insert("exp".into(), skills["fishing"]["exp"].clone());
    fishing.insert("skillPoints".into(), skills["fishing"]["points"].clone());
    fishing.insert("skills".into(), skills["fishing"]["perks"].clone());
    fishing.remove("legacyUnlocks");

    let migrated = migrate_save(value).unwrap();
    assert_eq!(
        migrated["state"]["fishing"]["legacyUnlocks"],
        json!(["creek", "river"])
    );
}

#[test]
fn fishing_level_gates_spots_and_rods_legacy_still_works() {
    let mut game = new_game();
    assert!(game.state().fishing.spot_unlocked(1, FishingSpot::Creek));
    assert!(!game.state().fishing.spot_unlocked(1, FishingSpot::River));
    assert!(game
        .dispatch(
            Action::CastLine {
                spot: FishingSpot::River,
                quality: None,
            },
            0
        )
        .is_err());

    game.state_mut().skills.fishing.level = 5;
    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::River,
            quality: None,
        },
        0,
    )
    .unwrap();

    // 鱼竿购买只看钓鱼等级；等级不够、或等级够但金币不够，都会失败。
    let mut rod_game = new_game();
    assert!(rod_game
        .dispatch(
            Action::BuyRod {
                rod: RodKind::Bamboo
            },
            0
        )
        .is_err());
    rod_game.state_mut().skills.fishing.level = 5;
    rod_game.state_mut().coins = 10_000;
    rod_game
        .dispatch(
            Action::BuyRod {
                rod: RodKind::Bamboo,
            },
            0,
        )
        .unwrap();

    // legacy：迁移前解锁的大海，钓鱼 1 级也能继续抛。
    let mut legacy_game = new_game();
    legacy_game
        .state_mut()
        .fishing
        .legacy_unlocks
        .insert("sea".into());
    assert!(legacy_game
        .state()
        .fishing
        .spot_unlocked(1, FishingSpot::Sea));
    legacy_game
        .dispatch(
            Action::CastLine {
                spot: FishingSpot::Sea,
                quality: None,
            },
            0,
        )
        .unwrap();
}

#[test]
fn new_save_has_no_legacy_spots_and_roundtrips() {
    let game = new_game();
    let save = game.to_save(0);
    assert_eq!(save.schema, SAVE_SCHEMA);
    assert!(save.state.fishing.legacy_unlocks.is_empty());
    assert_eq!(save.state.skills.fishing.level, 1);

    let value = serde_json::to_value(save).unwrap();
    let restored = Game::from_save(serde_json::from_value(value).unwrap()).unwrap();
    assert!(restored.state().fishing.legacy_unlocks.is_empty());
    assert_eq!(restored.state().skills.fishing.level, 1);
}

/// 手动验一份真实旧档（不随日常测试跑）：
/// `TEXT_FARM_REAL_SAVE=<save.json> cargo test -p farm-core --test skills real_v31_save -- --ignored`
#[test]
#[ignore]
fn real_v31_save_migrates() {
    let path = std::env::var("TEXT_FARM_REAL_SAVE").expect("要给 TEXT_FARM_REAL_SAVE");
    let raw = std::fs::read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let old_level = value["state"]["fishing"]["level"].clone();
    let old_exp = value["state"]["fishing"]["exp"].clone();
    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(33));
    assert_eq!(migrated["state"]["skills"]["fishing"]["level"], old_level);
    assert_eq!(migrated["state"]["skills"]["fishing"]["exp"], old_exp);
    assert!(migrated["state"]["fishing"].get("level").is_none());
    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(
        restored.state().skills.fishing.level,
        old_level.as_u64().unwrap() as u32
    );
}
