use farm_core::{
    level_need, plot_unlock_cost, plot_unlock_level, ContentPack, Game, SaveError, SAVE_SCHEMA,
};

#[test]
fn level_curve_is_monotonic_and_capped() {
    assert_eq!(level_need(1), 30);
    assert_eq!(level_need(2), 84);
    assert_eq!(level_need(30), 0);
    let mut prev = 0;
    for level in 1..30 {
        let need = level_need(level);
        assert!(need > prev, "等级 {} 的需求没有上升：{}", level, need);
        prev = need;
    }
}

#[test]
fn plot_unlocks_match_the_reference_screenshot() {
    assert_eq!(plot_unlock_level(1), 1);
    assert_eq!(plot_unlock_level(4), 1);
    assert_eq!(plot_unlock_level(5), 3);
    assert_eq!(plot_unlock_level(6), 6);
    assert_eq!(plot_unlock_level(7), 12);
    assert_eq!(plot_unlock_level(8), 21);
    assert_eq!(plot_unlock_cost(5), 300);
    assert_eq!(plot_unlock_cost(8), 30_000);
}

#[test]
fn starter_content_is_valid_and_has_thirteen_crops() {
    let content = ContentPack::starter();
    assert_eq!(content.crops.len(), 13);
    assert!(content.validate().is_empty(), "{:?}", content.validate());
    assert!(content.crop("xiaobaicai").is_some());
    let cane = content.crop("ganzhe").expect("甘蔗");
    assert_eq!(cane.sell_price, 22);
    assert!(cane.sow_seasons.contains(&farm_core::Season::Summer));
    assert!(content.crop("nope").is_none());
}

#[test]
fn old_content_pack_gains_sugarcane() {
    let mut content = ContentPack::starter();
    content.crops.retain(|crop| crop.id != "ganzhe");
    content.schema = 4;
    content.migrate_balance();
    assert!(content.crop("ganzhe").is_some());
}

#[test]
fn content_validation_catches_duplicate_ids() {
    let mut content = ContentPack::starter();
    content.crops[1].id = content.crops[0].id.clone();
    let issues = content.validate();
    assert!(
        issues.iter().any(|issue| issue.contains("重复")),
        "{issues:?}"
    );
}

#[test]
fn new_game_snapshot_has_four_plots_and_starter_seeds() {
    let game = Game::new(1_000);
    let snapshot = game.snapshot(1_000);
    assert_eq!(snapshot.level, 1);
    assert_eq!(snapshot.exp, 0);
    assert_eq!(snapshot.exp_need, 30);
    assert_eq!(snapshot.coins, 200);
    assert_eq!(snapshot.plots.len(), 8);
    assert_eq!(snapshot.plots.iter().filter(|p| p.unlocked).count(), 4);
    assert_eq!(snapshot.inventory.len(), 2);
    assert!(snapshot
        .inventory
        .iter()
        .any(|item| item.name == "小青菜种子" && item.qty == 3));
    assert_eq!(snapshot.content_crops, 13);
}

#[test]
fn old_default_balance_migrates_but_custom_values_stay() {
    let mut pack = ContentPack::starter();
    pack.schema = 1;
    let putao = pack.crops.iter_mut().find(|c| c.id == "putao").unwrap();
    putao.seed_price = 350;
    putao.sell_price = 45;
    let xigua = pack.crops.iter_mut().find(|c| c.id == "xigua").unwrap();
    xigua.seed_price = 123;
    xigua.sell_price = 45;

    pack.migrate_balance();
    assert_eq!(pack.schema, farm_core::CONTENT_SCHEMA);
    let putao = pack.crop("putao").unwrap();
    assert_eq!(putao.seed_price, 450);
    assert_eq!(putao.sell_price, 40);
    assert!(!putao.sow_seasons.is_empty());
    let xigua = pack.crop("xigua").unwrap();
    assert_eq!(xigua.seed_price, 123);
    assert_eq!(xigua.sell_price, 45);
}

#[test]
fn save_roundtrip_keeps_the_same_snapshot() {
    let game = Game::new(5_000);
    let save = game.to_save(6_000);
    assert_eq!(save.schema, SAVE_SCHEMA);
    assert_eq!(save.saved_at, 6_000);

    let bytes = serde_json::to_vec(&save).unwrap();
    let decoded: farm_core::SaveFile = serde_json::from_slice(&bytes).unwrap();
    let restored = Game::from_save(decoded).unwrap();

    assert_eq!(
        serde_json::to_value(game.snapshot(7_000)).unwrap(),
        serde_json::to_value(restored.snapshot(7_000)).unwrap()
    );
}

#[test]
fn save_from_the_future_is_rejected() {
    let game = Game::new(0);
    let mut save = game.to_save(0);
    save.schema = 99;
    assert!(matches!(Game::from_save(save), Err(SaveError::TooNew(99))));
}

#[test]
fn invalid_content_in_save_is_rejected() {
    let game = Game::new(0);
    let mut save = game.to_save(0);
    save.content.crops.clear();
    assert!(matches!(
        Game::from_save(save),
        Err(SaveError::InvalidContent(_))
    ));
}
