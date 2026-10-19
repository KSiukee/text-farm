use farm_core::{Action, ContentPack, CropInstance, Game, GameError};

fn game_with(level: u32, coins: u64, tier: u8) -> Game {
    let mut state = Game::new(0).state().clone();
    state.level = level;
    state.coins = coins;
    state.plots[0].tier = tier;
    state.inventory.clear();
    for threshold in [5, 10, 20, 30] {
        if level >= threshold {
            state
                .achievements
                .insert(format!("level_{threshold}"), true);
        }
    }
    Game::from_parts(state, ContentPack::starter())
}

#[test]
fn upgrading_a_plot_costs_coins_and_checks_level() {
    let mut game = game_with(5, 2_000, 0);
    assert_eq!(game.state().coins, 2_000);
    game.dispatch(Action::UpgradePlot { plot: 1 }, 0).unwrap();
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.plots[0].tier, 1);
    assert_eq!(snapshot.plots[0].tier_name, "肥沃");
    assert_eq!(snapshot.coins, 0);
    assert_eq!(game.state().stats.plot_upgrades, 1);

    // 肥沃 → 红土需要 12 级。
    assert!(matches!(
        game.dispatch(Action::UpgradePlot { plot: 1 }, 0),
        Err(GameError::LevelTooLow { need: 12 })
    ));
}

#[test]
fn upgrading_a_locked_or_maxed_plot_is_rejected() {
    let mut game = game_with(30, 1_000_000, 3);
    assert!(matches!(
        game.dispatch(Action::UpgradePlot { plot: 1 }, 0),
        Err(GameError::PlotMaxTier)
    ));
    assert!(matches!(
        game.dispatch(Action::UpgradePlot { plot: 5 }, 0),
        Err(GameError::PlotLocked)
    ));
}

#[test]
fn harvest_applies_land_yield_coin_and_exp_bonuses() {
    let mut state = Game::new(0).state().clone();
    state.level = 10;
    state.coins = 0;
    state.inventory.clear();
    state.plots[0].tier = 3;
    state.plots[0].crop = Some(CropInstance {
        crop_id: "yumi".into(),
        planted_at: 0,
        ready_at: 0,
        season: 1,
        seasons: 1,
        remaining: 10,
        watered: false,
        pest: None,
        disaster: None,
        off_season: false,
        growth_factor: 1000,
        greenhouse: false,
        withered: false,
    });
    let mut game = Game::from_parts(state, ContentPack::starter());

    let outcome = game.dispatch(Action::Harvest { plot: 1 }, 0).unwrap();
    let snapshot = game.snapshot(0);
    let qty = snapshot
        .inventory
        .iter()
        .find(|item| item.key == "crop:yumi")
        .map(|item| item.qty)
        .unwrap();
    // 基础 10 + 黑土收成 20%（2）+ 可能的幸运丰收
    assert!(qty >= 12, "qty={qty}");
    assert!(snapshot.coins >= 24, "coins={}", snapshot.coins);
    assert!(snapshot.exp >= 22, "exp={}", snapshot.exp);
    assert!(outcome.effects.iter().any(
        |effect| matches!(effect, farm_core::Effect::Log { text } if text.contains("土地收成+2"))
    ));
}

#[test]
fn plot_view_exposes_upgrade_affordability() {
    let game = game_with(5, 2_000, 0);
    let plot = &game.snapshot(0).plots[0];
    assert!(plot.can_upgrade);
    assert_eq!(plot.upgrade_cost, 2_000);
    assert_eq!(plot.upgrade_level, 5);
    assert_eq!(plot.effects.yield_pct, 0);

    let game = game_with(5, 1_999, 0);
    assert!(!game.snapshot(0).plots[0].can_upgrade);

    let game = game_with(12, 30_000, 1);
    let plot = &game.snapshot(0).plots[0];
    assert_eq!(plot.effects.exp_pct, 5.0);
    assert_eq!(plot.effects.yield_pct, 10);

    let game = game_with(21, 200_000, 2);
    let plot = &game.snapshot(0).plots[0];
    assert_eq!(plot.effects.exp_pct, 8.5);
    assert_eq!(plot.effects.coin_pct, 8.5);
    assert_eq!(plot.effects.yield_pct, 15);
}
