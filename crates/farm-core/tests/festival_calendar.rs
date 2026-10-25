use farm_core::{Action, Game};

#[test]
fn calendar_shows_seven_festivals_and_next_one() {
    let mut game = Game::new(0);
    game.sync(0, "2026-10-25");
    let calendar = game.snapshot(0).festival_calendar;
    assert_eq!(calendar.festivals.len(), 7);
    let next = calendar.next.expect("应有下一个节日");
    assert_eq!(next.name, "围炉日");
    assert_eq!(next.date, "2026-12-22");
    assert_eq!(next.days_until, Some(58));
}

#[test]
fn calendar_records_market_and_story_visits() {
    let mut game = Game::new(0);
    game.sync(0, "2026-10-01");
    game.dispatch(
        Action::ChooseFestivalStory {
            choice_id: "help_flag".into(),
        },
        0,
    )
    .unwrap();
    let calendar = game.snapshot(0).festival_calendar;
    assert_eq!(calendar.market_visits, 1);
    assert_eq!(calendar.memories, 1);
    let national = calendar
        .festivals
        .iter()
        .find(|day| day.name == "国庆节")
        .unwrap();
    assert!(national.story_done);
    assert!(national.market_seen);
}
