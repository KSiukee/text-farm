use farm_core::{Action, Game};

const MIN: i64 = 60_000;

#[test]
fn shift_time_fast_forwards_crop_growth() {
    let mut game = Game::new(0);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    let before = game.snapshot(0).plots[0].crop.as_ref().unwrap().ready_at;
    game.shift_time(10 * MIN);
    let after = game.snapshot(0).plots[0].crop.as_ref().unwrap().ready_at;
    assert_eq!(after, before - 10 * MIN);
    assert!(game.snapshot(0).plots[0].crop.as_ref().unwrap().can_harvest);
}

#[test]
fn shift_time_by_a_day_resets_daily_progress() {
    let mut game = Game::new(0);
    game.dispatch(Action::ClaimSignIn, 0).unwrap();
    assert!(game.state().daily.sign_in);
    game.shift_time(24 * 60 * MIN);
    assert!(!game.state().daily.sign_in);
    assert_eq!(game.state().daily.harvests, 0);
}

#[test]
fn shift_time_does_not_create_pests_by_itself() {
    let mut game = Game::new(0);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    let before = game.state().stats.pests_spawned;
    game.shift_time(60 * MIN);
    assert_eq!(game.state().stats.pests_spawned, before);
}
