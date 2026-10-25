use farm_core::{Action, Game, GameError};

#[test]
fn spring_festival_story_is_once_per_year() {
    let mut game = Game::new(0);
    game.sync(0, "2026-02-17");
    let story = game.snapshot(0).festival_story.expect("春节剧情应出现");
    assert_eq!(story.title, "年夜饭");
    assert_eq!(story.choices.len(), 2);

    let coins = game.state().coins;
    game.dispatch(
        Action::ChooseFestivalStory {
            choice_id: "ahe_dinner".into(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().coins > coins);
    assert!(game.snapshot(0).festival_story.is_none());
    assert_eq!(game.state().festival_memories.len(), 1);
    assert!(matches!(
        game.dispatch(
            Action::ChooseFestivalStory {
                choice_id: "ahe_dinner".into()
            },
            0
        ),
        Err(GameError::FestivalStoryDone)
    ));
}

#[test]
fn national_day_story_gives_market_contribution() {
    let mut game = Game::new(0);
    game.sync(0, "2026-10-01");
    assert!(game.snapshot(0).festival_story.is_some());
    game.dispatch(
        Action::ChooseFestivalStory {
            choice_id: "help_flag".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().festival_market.contribution, 3);
    assert_eq!(
        game.snapshot(0).festival_history[0].text,
        "帮村里把旗挂上了，风一吹，整面旗都展开。"
    );
}

#[test]
fn qingming_dragon_boat_and_mid_autumn_have_stories() {
    for (day, title) in [
        ("2026-04-05", "田埂上的青团"),
        ("2026-06-19", "河边的粽子"),
        ("2026-09-25", "晒场上的月亮"),
    ] {
        let mut game = Game::new(0);
        game.sync(0, day);
        let story = game.snapshot(0).festival_story.expect("节日剧情应出现");
        assert_eq!(story.title, title);
    }
}

#[test]
fn seasonal_festivals_have_stories() {
    for (day, title) in [
        ("2026-03-20", "第一把种子"),
        ("2026-06-21", "井里的瓜"),
        ("2026-09-23", "最好的一篮"),
        ("2026-12-22", "一锅热汤"),
    ] {
        let mut game = Game::new(0);
        game.sync(0, day);
        let story = game.snapshot(0).festival_story.expect("节气剧情应出现");
        assert_eq!(story.title, title);
    }
}
