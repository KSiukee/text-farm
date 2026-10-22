use farm_core::{migrate_save, Action, ContentPack, Game, GameError, SaveFile, SAVE_SCHEMA};
use serde_json::json;

fn game_at(level: u32, coins: u64) -> Game {
    let mut state = Game::new(0).state().clone();
    state.level = level;
    state.coins = coins;
    state.inventory.clear();
    Game::from_parts(state, ContentPack::starter())
}

#[test]
fn level_rewards_cover_two_to_thirty() {
    let game = Game::new(0);
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.level_rewards.len(), 29);
    assert_eq!(snapshot.level_rewards.first().unwrap().level, 2);
    assert_eq!(snapshot.level_rewards.last().unwrap().level, 30);
    assert_eq!(snapshot.level_rewards_claimable, 0);
    assert!(snapshot
        .level_rewards
        .iter()
        .all(|reward| !reward.claimed && !reward.claimable));

    let game = game_at(3, 0);
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.level_rewards_claimable, 2);
    let level_two = &snapshot.level_rewards[0];
    assert!(level_two.claimable);
    assert_eq!(level_two.coins, 90);
    let level_three = &snapshot.level_rewards[1];
    assert!(level_three.claimable);
    assert!(level_three.text.contains("草莓种子×2"));
}

#[test]
fn claim_level_three_adds_coins_and_new_crop_seeds() {
    let mut game = game_at(3, 1_000);
    let before = game.state().coins;
    game.dispatch(Action::ClaimLevelReward { level: 3 }, 0)
        .unwrap();
    assert_eq!(game.state().coins, before + 110);
    assert_eq!(game.state().inventory.get("seed:caomei"), Some(&2));
    assert!(game.state().level_rewards_claimed.contains(&3));
    assert_eq!(game.snapshot(0).level_rewards_claimable, 1);
    assert!(matches!(
        game.dispatch(Action::ClaimLevelReward { level: 3 }, 0),
        Err(GameError::AlreadyClaimed)
    ));
}

#[test]
fn level_five_and_thirty_milestones_give_ranch_feed() {
    let mut game = game_at(5, 0);
    game.dispatch(Action::ClaimLevelReward { level: 5 }, 0)
        .unwrap();
    assert_eq!(game.state().inventory.get("ranch:feed"), Some(&1));

    let mut game = game_at(30, 0);
    game.dispatch(Action::ClaimLevelReward { level: 30 }, 0)
        .unwrap();
    assert_eq!(game.state().inventory.get("ranch:feed"), Some(&3));
    assert_eq!(game.state().inventory.get("seed:yuejiancao"), Some(&2));
}

#[test]
fn claim_level_reward_rejects_bad_levels() {
    let mut game = game_at(1, 0);
    assert!(matches!(
        game.dispatch(Action::ClaimLevelReward { level: 1 }, 0),
        Err(GameError::BadLevelReward)
    ));
    assert!(matches!(
        game.dispatch(Action::ClaimLevelReward { level: 31 }, 0),
        Err(GameError::BadLevelReward)
    ));
    assert!(matches!(
        game.dispatch(Action::ClaimLevelReward { level: 2 }, 0),
        Err(GameError::LevelTooLow { need: 2 })
    ));
}

#[test]
fn claimed_level_rewards_survive_save_roundtrip() {
    let mut game = game_at(5, 0);
    game.dispatch(Action::ClaimLevelReward { level: 2 }, 0)
        .unwrap();
    game.dispatch(Action::ClaimLevelReward { level: 5 }, 0)
        .unwrap();
    let save = game.to_save(0);
    assert_eq!(save.schema, SAVE_SCHEMA);
    let restored = Game::from_save(save).unwrap();
    assert!(restored.state().level_rewards_claimed.contains(&2));
    assert!(restored.state().level_rewards_claimed.contains(&5));
    assert!(!restored.state().level_rewards_claimed.contains(&3));
}

#[test]
fn v29_save_migrates_level_rewards_and_daily_counters() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(29);
    let state = value["state"].as_object_mut().unwrap();
    state.insert("schema".into(), json!(29));
    state.remove("levelRewardsClaimed");
    if let Some(daily) = state
        .get_mut("daily")
        .and_then(|daily| daily.as_object_mut())
    {
        for key in [
            "ranchCollects",
            "fishCatches",
            "claimedRanch",
            "claimedFish",
        ] {
            daily.remove(key);
        }
    }

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["levelRewardsClaimed"], json!([]));
    assert_eq!(migrated["state"]["daily"]["ranchCollects"], json!(0));
    assert_eq!(migrated["state"]["daily"]["claimedFish"], json!(false));

    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert!(restored.state().level_rewards_claimed.is_empty());
    assert_eq!(restored.state().daily.ranch_collects, 0);
}

#[test]
fn v30_save_migrates_fishing_level_and_skills() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    value["schema"] = json!(30);
    let state = value["state"].as_object_mut().unwrap();
    state.insert("schema".into(), json!(30));
    let fishing = state
        .get_mut("fishing")
        .and_then(|fishing| fishing.as_object_mut())
        .unwrap();
    for key in ["level", "exp", "skillPoints", "skills"] {
        fishing.remove(key);
    }

    let migrated = migrate_save(value).unwrap();
    assert_eq!(migrated["schema"], json!(34));
    assert_eq!(migrated["state"]["skills"]["fishing"]["level"], json!(1));
    assert_eq!(migrated["state"]["skills"]["fishing"]["exp"], json!(0));
    assert_eq!(migrated["state"]["skills"]["fishing"]["points"], json!(0));
    assert_eq!(migrated["state"]["skills"]["fishing"]["perks"], json!({}));
    assert!(migrated["state"]["fishing"].get("level").is_none());

    let save: SaveFile = serde_json::from_value(migrated).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state().skills.fishing.level, 1);
    assert!(restored.state().skills.fishing.perks.is_empty());
}
