use farm_core::{
    spot_fish, Action, BaitQuality, BaitType, FishCodexEntry, FishPersonality, FishingSession,
    FishingSpot, Game,
};

fn set_session(game: &mut Game, now: i64, fish_id: &str, spot: FishingSpot) {
    game.state_mut().fishing.session = Some(FishingSession {
        id: (now.max(0) as u64) + 1,
        spot,
        fish_id: fish_id.into(),
        started_at: now,
        expires_at: now + 60_000,
        bait_quality: BaitQuality::Normal,
        personality: FishPersonality::default(),
    });
}

fn catch(game: &mut Game, now: i64, fish_id: &str, spot: FishingSpot) {
    set_session(game, now, fish_id, spot);
    game.dispatch(Action::ResolveFishing { success: true }, now)
        .unwrap();
}

fn fill_spot(game: &mut Game, spot: FishingSpot) {
    for def in spot_fish(spot) {
        game.state_mut().fishing.codex.insert(
            def.id.into(),
            FishCodexEntry {
                count: 1,
                ..Default::default()
            },
        );
    }
}

#[test]
fn spot_completion_reward_is_granted_once() {
    let mut game = Game::new(0);
    fill_spot(&mut game, FishingSpot::Creek);
    let before = game.state().coins;
    catch(&mut game, 0, "white_bait", FishingSpot::Creek);
    assert_eq!(game.state().coins, before + 2_000);
    assert_eq!(
        game.state()
            .fishing
            .bait_count_of(BaitType::Worm, BaitQuality::Fine),
        10
    );
    assert!(game
        .state()
        .fishing
        .spot_rewards
        .contains(FishingSpot::Creek.key()));

    let after = game.state().coins;
    catch(&mut game, 1_000, "white_bait", FishingSpot::Creek);
    assert_eq!(game.state().coins, after);
}

#[test]
fn full_codex_reward_is_granted_once() {
    let mut game = Game::new(0);
    for spot in FishingSpot::ALL {
        fill_spot(&mut game, spot);
    }
    let before = game.state().coins;
    catch(&mut game, 0, "white_bait", FishingSpot::Creek);
    // 四个钓点奖励 2,000 + 6,000 + 15,000 + 40,000 + 全图鉴 100,000。
    assert_eq!(game.state().coins, before + 163_000);
    assert!(game.state().fishing.full_codex_claimed);
    assert_eq!(
        game.state()
            .fishing
            .bait_count_of(BaitType::Lure, BaitQuality::Master),
        15
    );

    let after = game.state().coins;
    catch(&mut game, 1_000, "white_bait", FishingSpot::Creek);
    assert_eq!(game.state().coins, after);
}

#[test]
fn golden_variant_is_recorded_without_touching_stock() {
    let mut game = Game::new(0);
    let mut found = false;
    for now in 0..1_000i64 {
        catch(&mut game, now, "white_bait", FishingSpot::Creek);
        if game.state().fishing.last_catch.as_ref().unwrap().golden {
            found = true;
            break;
        }
    }
    assert!(found, "1000 次都没出金色变异（1.5% 概率），实现有问题");
    assert!(game.state().fishing.stats.golden_catches > 0);
    assert!(game
        .state()
        .fishing
        .golden_codex
        .get("white_bait")
        .is_some_and(|count| *count > 0));
    // 金色变异不改库存：只有 fish:white_bait，没有 golden 库存键。
    assert!(game
        .state()
        .inventory
        .get("fish:white_bait")
        .is_some_and(|count| *count > 0));
    assert!(game
        .state()
        .inventory
        .keys()
        .all(|key| !key.contains("golden")));
}

#[test]
fn first_catch_is_a_new_record() {
    let mut game = Game::new(0);
    catch(&mut game, 0, "white_bait", FishingSpot::Creek);
    let catch = game.state().fishing.last_catch.as_ref().unwrap();
    assert!(catch.is_record);
    assert_eq!(
        game.state()
            .fishing
            .codex
            .get("white_bait")
            .unwrap()
            .best_weight_tenths,
        catch.weight_tenths
    );
}

#[test]
fn codex_view_shows_spot_progress_and_golden() {
    let mut game = Game::new(0);
    fill_spot(&mut game, FishingSpot::Creek);
    catch(&mut game, 0, "white_bait", FishingSpot::Creek);
    let snapshot = game.snapshot(0);
    let creek = snapshot
        .fishing
        .spot_progress
        .iter()
        .find(|spot| spot.spot == FishingSpot::Creek)
        .unwrap();
    assert_eq!(creek.caught, 9);
    assert_eq!(creek.total, 9);
    assert!(creek.complete);
    assert!(creek.claimed);
    assert_eq!(snapshot.fishing.codex_total, 35);
    assert_eq!(snapshot.fishing.king_trophies.len(), 4);
    assert!(snapshot.fishing.golden_total == 35);
}

#[test]
fn old_save_defaults_codex_flags() {
    let game = Game::new(0);
    let mut value = serde_json::to_value(game.to_save(0)).unwrap();
    let fishing = value["state"]["fishing"].as_object_mut().unwrap();
    for key in ["spotRewards", "goldenCodex", "fullCodexClaimed"] {
        fishing.remove(key);
    }
    let save: farm_core::SaveFile = serde_json::from_value(value).unwrap();
    let restored = Game::from_save(save).unwrap();
    assert!(restored.state().fishing.spot_rewards.is_empty());
    assert!(restored.state().fishing.golden_codex.is_empty());
    assert!(!restored.state().fishing.full_codex_claimed);
}
