use farm_core::{Action, ContentPack, Game, Region, RegionAffinity, Season};

const MIN: i64 = 60_000;

fn game_in(season: Season) -> Game {
    let mut state = Game::new(0).state().clone();
    state.level = 14;
    state.season = season;
    state.inventory.clear();
    state.inventory.insert("seed:xigua".into(), 3);
    Game::from_parts(state, ContentPack::starter())
}

#[test]
fn seasons_map_to_months() {
    assert_eq!(Season::from_month(3), Season::Spring);
    assert_eq!(Season::from_month(6), Season::Summer);
    assert_eq!(Season::from_month(9), Season::Autumn);
    assert_eq!(Season::from_month(12), Season::Winter);
    assert_eq!(Season::from_today("2026-07-01"), Season::Summer);
}

#[test]
fn sync_updates_the_current_season() {
    let mut game = Game::new(0);
    game.sync(0, "2026-07-01");
    assert_eq!(game.snapshot(0).season, Season::Summer);
    assert_eq!(game.snapshot(0).season_label, "夏季");
}

#[test]
fn cross_season_recalculates_growing_time() {
    let mut game = game_in(Season::Summer);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xigua".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.snapshot(0).plots[0].crop.as_ref().unwrap().ready_at,
        480 * MIN
    );
    game.sync(100 * MIN, "2026-09-01");
    let crop = game.snapshot(100 * MIN).plots[0].crop.clone().unwrap();
    assert!(crop.ready_at > 480 * MIN);
    assert_eq!(
        game.state().plots[0].crop.as_ref().unwrap().growth_factor,
        750
    );
}

#[test]
fn greenhouse_ignores_season_penalty() {
    let mut state = Game::new(0).state().clone();
    state.level = 14;
    state.season = Season::Winter;
    state.facilities.greenhouse = true;
    state.inventory.clear();
    state.inventory.insert("seed:xigua".into(), 1);
    let mut game = Game::from_parts(state, ContentPack::starter());
    game.ensure_greenhouse();
    game.dispatch(
        Action::Plant {
            plot: 9,
            crop_id: "xigua".into(),
        },
        0,
    )
    .unwrap();
    let crop = game.snapshot(0).plots[8].crop.clone().unwrap();
    assert!(crop.off_season);
    assert!(crop.greenhouse);
    assert!(crop.ready_at > 480 * MIN);
    assert_eq!(
        game.state().plots[8].crop.as_ref().unwrap().growth_factor,
        950
    );
}

#[test]
fn south_region_has_a_softer_penalty() {
    let mut state = Game::new(0).state().clone();
    state.level = 14;
    state.season = Season::Winter;
    state.settings.region = Region::South;
    state.inventory.clear();
    state.inventory.insert("seed:xigua".into(), 1);
    let mut game = Game::from_parts(state, ContentPack::starter());
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xigua".into(),
        },
        0,
    )
    .unwrap();
    let crop = game.snapshot(0).plots[0].crop.clone().unwrap();
    assert!(crop.off_season);
    assert_eq!(
        game.state().plots[0].crop.as_ref().unwrap().growth_factor,
        850
    );
}

#[test]
fn changing_region_recalculates_growth() {
    let mut game = game_in(Season::Winter);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xigua".into(),
        },
        0,
    )
    .unwrap();
    let before = game.snapshot(0).plots[0].crop.as_ref().unwrap().ready_at;
    game.dispatch(
        Action::SetRegion {
            region: Region::South,
        },
        60 * MIN,
    )
    .unwrap();
    let after = game.snapshot(60 * MIN).plots[0]
        .crop
        .as_ref()
        .unwrap()
        .ready_at;
    assert!(after < before);
}

#[test]
fn wrong_region_seed_can_wither_and_be_cleared() {
    let mut state = Game::new(0).state().clone();
    state.level = 14;
    state.settings.region = Region::South;
    state.inventory.clear();
    state.inventory.insert("seed:xigua".into(), 100);
    let mut content = ContentPack::starter();
    content
        .crops
        .iter_mut()
        .find(|crop| crop.id == "xigua")
        .unwrap()
        .region = RegionAffinity::North;
    let mut game = Game::from_parts(state, content);

    let mut withered = false;
    for attempt in 0..30 {
        let now = i64::from(attempt) * MIN;
        game.dispatch(
            Action::Plant {
                plot: 1,
                crop_id: "xigua".into(),
            },
            now,
        )
        .unwrap();
        if game.snapshot(now).plots[0]
            .crop
            .as_ref()
            .is_some_and(|crop| crop.withered)
        {
            withered = true;
        }
        game.dispatch(Action::Clear { plot: 1 }, now).unwrap();
        if withered {
            break;
        }
    }
    assert!(withered);
    assert!(game.snapshot(0).plots[0].crop.is_none());
}
