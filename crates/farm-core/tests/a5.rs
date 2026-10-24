use farm_core::{
    Action, ContentPack, CropInstance, DailyState, FishingSpot, Game, GameError, Pest, PestKind,
    RanchAnimalKind,
};

const MIN: i64 = 60_000;

#[test]
fn clearing_a_bug_does_not_stop_growth() {
    let mut state = Game::new(0).state().clone();
    state.plots[0].crop = Some(CropInstance {
        crop_id: "xiaobaicai".into(),
        planted_at: 0,
        ready_at: 100 * MIN,
        season: 1,
        seasons: 1,
        remaining: 2,
        watered: true,
        pest: Some(Pest {
            kind: PestKind::Bug,
            since: 90 * MIN,
        }),
        disaster: None,
        off_season: false,
        growth_factor: 1000,
        greenhouse: false,
        withered: false,
    });
    let mut game = Game::from_parts(state, ContentPack::starter());

    game.dispatch(Action::ClearPest { plot: 1 }, 100 * MIN)
        .unwrap();
    let crop = game.snapshot(100 * MIN).plots[0].crop.clone().unwrap();
    assert!(crop.pest.is_none());
    assert_eq!(crop.ready_at, 100 * MIN);
    assert_eq!(game.state().stats.pests_cleared, 1);
}

#[test]
fn clearing_grass_does_not_stop_growth() {
    let mut state = Game::new(0).state().clone();
    state.plots[0].crop = Some(CropInstance {
        crop_id: "xiaobaicai".into(),
        planted_at: 0,
        ready_at: 100 * MIN,
        season: 1,
        seasons: 1,
        remaining: 2,
        watered: true,
        pest: Some(Pest {
            kind: PestKind::Grass,
            since: 90 * MIN,
        }),
        disaster: None,
        off_season: false,
        growth_factor: 1000,
        greenhouse: false,
        withered: false,
    });
    let mut game = Game::from_parts(state, ContentPack::starter());
    game.dispatch(Action::ClearPest { plot: 1 }, 100 * MIN)
        .unwrap();
    assert_eq!(
        game.snapshot(100 * MIN).plots[0]
            .crop
            .as_ref()
            .unwrap()
            .ready_at,
        100 * MIN
    );
}

#[test]
fn harvest_is_blocked_until_the_pest_is_cleared() {
    let mut state = Game::new(0).state().clone();
    state.plots[0].crop = Some(CropInstance {
        crop_id: "xiaobaicai".into(),
        planted_at: 0,
        ready_at: 3 * MIN,
        season: 1,
        seasons: 1,
        remaining: 2,
        watered: false,
        pest: Some(Pest {
            kind: PestKind::Bug,
            since: 0,
        }),
        disaster: None,
        off_season: false,
        growth_factor: 1000,
        greenhouse: false,
        withered: false,
    });
    let mut game = Game::from_parts(state, ContentPack::starter());
    assert!(matches!(
        game.dispatch(Action::Harvest { plot: 1 }, 10 * MIN),
        Err(GameError::PestBlocking)
    ));
}

#[test]
fn sync_resets_daily_and_sign_in_works_once() {
    let mut game = Game::new(0);
    game.sync(0, "2026-10-18");
    assert_eq!(game.state().daily.day, "2026-10-18");

    game.dispatch(Action::ClaimSignIn, 0).unwrap();
    assert!(game.state().daily.sign_in);
    assert_eq!(game.state().stats.sign_ins, 1);
    assert!(matches!(
        game.dispatch(Action::ClaimSignIn, 0),
        Err(GameError::AlreadySignedIn)
    ));

    game.sync(24 * 60 * MIN, "2026-10-19");
    assert_eq!(game.state().daily.day, "2026-10-19");
    assert!(!game.state().daily.sign_in);
    assert_eq!(game.state().daily.harvests, 0);
}

#[test]
fn daily_rewards_require_progress_and_only_pay_once() {
    let mut state = Game::new(0).state().clone();
    state.daily = DailyState {
        day: "2026-10-18".into(),
        harvests: 10,
        waterings: 5,
        pests_cleared: 3,
        ..DailyState::default()
    };
    let mut game = Game::from_parts(state, ContentPack::starter());

    game.dispatch(Action::ClaimDaily { id: 1 }, 0).unwrap();
    assert!(game.state().daily.claimed_harvest);
    assert!(matches!(
        game.dispatch(Action::ClaimDaily { id: 1 }, 0),
        Err(GameError::AlreadyClaimed)
    ));

    assert!(matches!(
        game.dispatch(Action::ClaimDaily { id: 9 }, 0),
        Err(GameError::BadDailyTask)
    ));
}

#[test]
fn daily_ranch_task_shows_only_when_ranch_has_animal() {
    let game = Game::new(0);
    let tasks = game.snapshot(0).daily.tasks;
    assert_eq!(tasks.len(), 5);
    assert!(tasks.iter().all(|task| task.id != 4));
    assert!(tasks.iter().any(|task| task.id == 5));

    let mut game = Game::new(0);
    game.state_mut().coins = 1_000;
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 1,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
    let tasks = game.snapshot(0).daily.tasks;
    assert_eq!(tasks.len(), 6);
    assert!(tasks.iter().any(|task| task.id == 4));
}

#[test]
fn ranch_and_fishing_daily_tasks_count_progress() {
    let mut game = Game::new(0);
    game.state_mut().coins = 1_000;
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 1,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
    game.state_mut()
        .ranch
        .pen_mut(1)
        .unwrap()
        .animal
        .as_mut()
        .unwrap()
        .product_ready_at = 0;
    game.dispatch(Action::CollectRanchProduct { pen: 1 }, 0)
        .unwrap();
    assert_eq!(game.state().daily.ranch_collects, 1);

    game.dispatch(
        Action::CastLine {
            spot: FishingSpot::Creek,
            quality: None,
        },
        0,
    )
    .unwrap();
    game.dispatch(Action::ResolveFishing { success: true }, 10_000)
        .unwrap();
    assert_eq!(game.state().daily.fish_catches, 1);

    let tasks = game.snapshot(10_000).daily.tasks;
    let ranch = tasks.iter().find(|task| task.id == 4).unwrap();
    assert_eq!(ranch.progress, 1);
    let fish = tasks.iter().find(|task| task.id == 5).unwrap();
    assert_eq!(fish.progress, 1);
}

#[test]
fn daily_ranch_and_fishing_rewards_pay_once() {
    let mut state = Game::new(0).state().clone();
    state.daily = DailyState {
        day: "2026-10-18".into(),
        ranch_collects: 3,
        fish_catches: 3,
        ..DailyState::default()
    };
    let mut game = Game::from_parts(state, ContentPack::starter());

    let before = game.state().coins;
    game.dispatch(Action::ClaimDaily { id: 4 }, 0).unwrap();
    assert!(game.state().daily.claimed_ranch);
    assert_eq!(game.state().coins, before + 60);
    assert!(matches!(
        game.dispatch(Action::ClaimDaily { id: 4 }, 0),
        Err(GameError::AlreadyClaimed)
    ));

    game.dispatch(Action::ClaimDaily { id: 5 }, 0).unwrap();
    assert!(game.state().daily.claimed_fish);
}

#[test]
fn first_plant_and_harvest_award_achievements() {
    let mut game = Game::new(0);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.snapshot(0).achievements_earned, 1);
    assert_eq!(game.snapshot(0).coins, 220);

    game.dispatch(Action::Harvest { plot: 1 }, 3 * MIN).unwrap();
    let snapshot = game.snapshot(3 * MIN);
    assert_eq!(snapshot.achievements_earned, 2);
    assert!(snapshot
        .achievements
        .iter()
        .any(|a| a.id == "first_harvest" && a.earned));
}

#[test]
fn harvest_fills_the_codex() {
    let mut game = Game::new(0);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    game.dispatch(Action::Harvest { plot: 1 }, 3 * MIN).unwrap();
    let snapshot = game.snapshot(3 * MIN);
    let entry = snapshot
        .codex
        .iter()
        .find(|c| c.crop_id == "xiaobaicai")
        .unwrap();
    assert!(entry.discovered);
    assert_eq!(entry.total, 2);
    assert_eq!(snapshot.codex_discovered, 1);
}

#[test]
fn exp_rate_multiplies_gains() {
    let mut game = Game::new(0);
    game.dispatch(Action::UpdateSettings { exp_rate: 2 }, 0)
        .unwrap();
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.snapshot(0).exp, 4); // 初种成就 2 × 2
    assert!(matches!(
        game.dispatch(Action::UpdateSettings { exp_rate: 3 }, 0),
        Err(GameError::InvalidExpRate)
    ));
}

#[test]
fn content_rename_updates_names_without_breaking_the_crop() {
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
    pack.nouns.farm = "青禾农场".into();
    pack.crops[0].name = "青青菜".into();
    game.apply_content(pack, 0).unwrap();

    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.farm_name, "青禾农场");
    assert_eq!(snapshot.plots[0].crop.as_ref().unwrap().name, "青青菜");
}
