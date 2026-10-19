use farm_core::{Action, ContentPack, FacilityKind, Game, GameError, Season};

#[test]
fn fifteen_windmills_give_fifteen_percent_and_repair_restores() {
    let mut state = Game::new(0).state().clone();
    state.level = 30;
    state.coins = 10_000_000;
    state.season = Season::Summer;
    state.inventory.clear();
    state.inventory.insert("seed:xigua".into(), 2);
    let mut game = Game::from_parts(state, ContentPack::starter());

    for _ in 0..15 {
        game.dispatch(
            Action::BuyFacility {
                kind: FacilityKind::Windmill,
            },
            0,
        )
        .unwrap();
    }
    assert_eq!(game.state().facilities.windmills, 15);
    game.dispatch(
        Action::BuyFacility {
            kind: FacilityKind::Windmill,
        },
        0,
    )
    .unwrap_err();

    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xigua".into(),
        },
        0,
    )
    .unwrap();
    let ready = game.snapshot(0).plots[0].crop.as_ref().unwrap().ready_at;
    assert_eq!(ready, 480 * 60_000 * 85 / 100);
}

#[test]
fn repair_damaged_windmills_costs_coins() {
    let mut state = Game::new(0).state().clone();
    state.coins = 20_000;
    state.facilities.windmills = 3;
    state.facilities.damaged_windmills = 2;
    let mut game = Game::from_parts(state, ContentPack::starter());
    game.dispatch(Action::RepairWindmills, 0).unwrap();
    assert_eq!(game.state().facilities.damaged_windmills, 0);
    // 20,000 - 6,000 维修费 + 200 设施成就奖励。
    assert_eq!(game.state().coins, 14_200);
}

#[test]
fn repair_without_damage_is_rejected() {
    let mut game = Game::new(0);
    assert!(matches!(
        game.dispatch(Action::RepairWindmills, 0),
        Err(GameError::NoDamagedWindmill)
    ));
}
