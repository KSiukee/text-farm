use farm_core::{Action, ContentPack, Game, GameError, PlotStatus};

const MIN: i64 = 60_000;

fn game_with(level: u32, seeds: &[(&str, u32)], coins: u64) -> Game {
    let mut state = Game::new(0).state().clone();
    state.level = level;
    state.coins = coins;
    state.inventory.clear();
    for (crop, qty) in seeds {
        state.inventory.insert(format!("seed:{crop}"), *qty);
    }
    Game::from_parts(state, ContentPack::starter())
}

fn plant(game: &mut Game, plot: u8, crop: &str, now: i64) {
    game.dispatch(
        Action::Plant {
            plot,
            crop_id: crop.to_string(),
        },
        now,
    )
    .unwrap();
}

#[test]
fn plant_grow_water_harvest_loop() {
    let mut game = Game::new(0);
    plant(&mut game, 1, "xiaobaicai", 0);

    let growing = game.snapshot(0).plots[0].crop.clone().unwrap();
    assert_eq!(growing.status, PlotStatus::Growing);
    assert_eq!(growing.ready_at, 3 * MIN);
    assert!(growing.can_water);

    assert!(matches!(
        game.dispatch(Action::Harvest { plot: 1 }, 0),
        Err(GameError::NotReady)
    ));

    let watered = game.dispatch(Action::Water { plot: 1 }, 0).unwrap();
    assert!(watered.rev >= 2);
    let crop = game.snapshot(0).plots[0].crop.clone().unwrap();
    assert!(crop.ready_at < 3 * MIN);
    assert!(crop.watered);
    assert!(!crop.can_water);

    game.dispatch(Action::Harvest { plot: 1 }, 3 * MIN).unwrap();
    let snapshot = game.snapshot(3 * MIN);
    assert!(snapshot.plots[0].crop.is_none());
    assert!(snapshot
        .inventory
        .iter()
        .any(|item| item.key == "crop:xiaobaicai" && item.qty == 2));
    assert_eq!(snapshot.exp, 9); // 浇水 1 + 收获 3 + 初种/初收成就 5
}

#[test]
fn multi_season_crop_regrows_then_finishes() {
    let mut game = game_with(3, &[("caomei", 1)], 200);
    plant(&mut game, 1, "caomei", 0);

    game.dispatch(Action::Harvest { plot: 1 }, 30 * MIN)
        .unwrap();
    let crop = game.snapshot(30 * MIN).plots[0].crop.clone().unwrap();
    assert_eq!(crop.season, 2);
    assert_eq!(crop.seasons, 2);
    assert_eq!(crop.remaining, 5);
    assert_eq!(crop.ready_at, 45 * MIN);
    assert_eq!(crop.status, PlotStatus::Growing);

    assert!(matches!(
        game.dispatch(Action::Harvest { plot: 1 }, 40 * MIN),
        Err(GameError::NotReady)
    ));

    game.dispatch(Action::Harvest { plot: 1 }, 45 * MIN)
        .unwrap();
    assert!(game.snapshot(45 * MIN).plots[0].crop.is_none());
    assert!(game
        .snapshot(45 * MIN)
        .inventory
        .iter()
        .any(|item| item.key == "crop:caomei" && item.qty == 10));
}

#[test]
fn nine_harvests_level_the_farm_to_two() {
    let mut game = game_with(1, &[("xiaobaicai", 10)], 200);
    let mut now = 0;
    for _ in 0..9 {
        plant(&mut game, 1, "xiaobaicai", now);
        now += 3 * MIN;
        game.dispatch(Action::Harvest { plot: 1 }, now).unwrap();
    }
    let snapshot = game.snapshot(now);
    assert_eq!(snapshot.level, 2);
    assert_eq!(snapshot.exp, 2);
    assert_eq!(snapshot.exp_need, 84);
}

#[test]
fn unlock_plot_costs_coins_and_checks_level() {
    let mut game = game_with(3, &[], 1_000);
    game.dispatch(Action::UnlockPlot { plot: 5 }, 0).unwrap();
    let snapshot = game.snapshot(0);
    assert!(snapshot.plots[4].unlocked);
    assert_eq!(snapshot.coins, 1_000); // 1000 - 300 + 五块地成就 300

    assert!(matches!(
        game.dispatch(Action::UnlockPlot { plot: 6 }, 0),
        Err(GameError::LevelTooLow { need: 6 })
    ));
}

#[test]
fn buy_and_sell_are_atomic_on_failure() {
    let mut game = Game::new(0);
    let before = game.snapshot(0);
    assert!(matches!(
        game.dispatch(
            Action::BuySeed {
                crop_id: "renshen".into(),
                qty: 1
            },
            0
        ),
        Err(GameError::LevelTooLow { need: 18 })
    ));
    let after = game.snapshot(0);
    assert_eq!(before.coins, after.coins);

    game.dispatch(
        Action::BuySeed {
            crop_id: "luobo".into(),
            qty: 2,
        },
        0,
    )
    .unwrap();
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.coins, 180);
    assert!(snapshot
        .inventory
        .iter()
        .any(|item| item.key == "seed:luobo" && item.qty == 4));
}

#[test]
fn harvest_all_collects_every_ready_plot() {
    let mut state = Game::new(0).state().clone();
    state.facilities.harvester = true;
    state.inventory.clear();
    state.inventory.insert("seed:xiaobaicai".into(), 4);
    let mut game = Game::from_parts(state, ContentPack::starter());

    for plot in 1..=4 {
        plant(&mut game, plot, "xiaobaicai", 0);
    }
    game.dispatch(Action::HarvestAll, 3 * MIN).unwrap();
    let snapshot = game.snapshot(3 * MIN);
    assert!(snapshot.plots.iter().all(|p| p.crop.is_none()));
    assert!(snapshot
        .inventory
        .iter()
        .any(|item| item.key == "crop:xiaobaicai" && item.qty == 8));
}

#[test]
fn broadcast_is_bounded_and_non_empty() {
    let mut game = Game::new(0);
    assert!(matches!(
        game.dispatch(Action::Broadcast { text: "  ".into() }, 0),
        Err(GameError::EmptyText)
    ));
    assert!(matches!(
        game.dispatch(
            Action::Broadcast {
                text: "好".repeat(41)
            },
            0
        ),
        Err(GameError::BroadcastTooLong)
    ));
    game.dispatch(
        Action::Broadcast {
            text: "好".repeat(40),
        },
        0,
    )
    .unwrap();
}

#[test]
fn action_wire_format_is_camel_case() {
    let json = serde_json::to_value(Action::Plant {
        plot: 1,
        crop_id: "xiaobaicai".into(),
    })
    .unwrap();
    assert_eq!(json["type"], "plant");
    assert_eq!(json["cropId"], "xiaobaicai");

    let json = serde_json::to_value(Action::BuySeed {
        crop_id: "luobo".into(),
        qty: 2,
    })
    .unwrap();
    assert_eq!(json["type"], "buySeed");
    assert_eq!(json["cropId"], "luobo");
}
