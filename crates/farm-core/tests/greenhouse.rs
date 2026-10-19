use farm_core::{Action, ContentPack, FacilityKind, Game, Season};

#[test]
fn greenhouse_is_a_separate_upgradeable_plot() {
    let mut state = Game::new(0).state().clone();
    state.level = 20;
    state.coins = 100_000;
    state.season = Season::Winter;
    state.inventory.clear();
    state.inventory.insert("seed:xigua".into(), 2);
    let mut game = Game::from_parts(state, ContentPack::starter());

    game.dispatch(
        Action::BuyFacility {
            kind: FacilityKind::Greenhouse,
        },
        0,
    )
    .unwrap();
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.plots.len(), 11);
    assert_eq!(snapshot.plots[8].label, "温室1");
    assert_eq!(snapshot.plots[8].kind, farm_core::PlotKind::Greenhouse);
    assert_eq!(snapshot.plots[10].label, "温室3");

    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xigua".into(),
        },
        0,
    )
    .unwrap();
    game.dispatch(
        Action::Plant {
            plot: 9,
            crop_id: "xigua".into(),
        },
        0,
    )
    .unwrap();
    let snapshot = game.snapshot(0);
    assert!(!snapshot.plots[0].crop.as_ref().unwrap().greenhouse);
    assert!(snapshot.plots[8].crop.as_ref().unwrap().greenhouse);

    game.dispatch(Action::UpgradePlot { plot: 9 }, 0).unwrap();
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.plots[8].tier_name, "温室·改良");
    assert_eq!(snapshot.plots[8].effects.exp_pct, 4.0);
    assert_eq!(snapshot.plots[0].tier, 0);
}

#[test]
fn three_greenhouses_give_nine_plots_and_fourth_is_rejected() {
    let mut state = Game::new(0).state().clone();
    state.level = 30;
    state.coins = 1_000_000;
    let mut game = Game::from_parts(state, ContentPack::starter());
    for _ in 0..3 {
        game.dispatch(
            Action::BuyFacility {
                kind: FacilityKind::Greenhouse,
            },
            0,
        )
        .unwrap();
    }
    assert_eq!(game.snapshot(0).plots.len(), 17);
    assert_eq!(game.state().facilities.greenhouses, 3);
    assert!(matches!(
        game.dispatch(
            Action::BuyFacility {
                kind: FacilityKind::Greenhouse
            },
            0
        ),
        Err(farm_core::GameError::FacilityMax)
    ));
}
