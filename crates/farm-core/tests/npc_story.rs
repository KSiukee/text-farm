use farm_core::{Action, Effect, Game, NpcMemory};

fn setup() -> Game {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-25".into();
    game.state_mut().npcs[0].friendship = 30;
    game
}

fn talk(game: &mut Game) -> (String, i32, u64) {
    let before = game.state().coins;
    let outcome = game
        .dispatch(
            Action::ChatWithNpc {
                npc_id: "ahe".into(),
            },
            0,
        )
        .unwrap();
    let Effect::NpcTalk { title, favor, .. } = outcome.effects[0].clone() else {
        panic!("expected npc talk effect");
    };
    (title, favor, game.state().coins - before)
}

fn choose(game: &mut Game, event_id: &str, choice_id: &str) -> (String, i32, u64) {
    let before = game.state().coins;
    let outcome = game
        .dispatch(
            Action::NpcStoryChoice {
                npc_id: "ahe".into(),
                event_id: event_id.into(),
                choice_id: choice_id.into(),
            },
            0,
        )
        .unwrap();
    let Effect::NpcTalk {
        text, favor, tag, ..
    } = outcome.effects[0].clone()
    else {
        panic!("expected npc talk effect");
    };
    assert_eq!(tag, "选择");
    (text, favor, game.state().coins - before)
}

#[test]
fn chat_once_per_day_and_unlocks_first_story() {
    let mut game = setup();
    let (title, favor, coins) = talk(&mut game);
    assert_eq!(title, "老锄头");
    assert_eq!(favor, 0);
    assert_eq!(coins, 0);
    assert_eq!(game.state().npcs[0].friendship, 31);
    assert!(game.state().npcs[0].seen_events.contains(&"ahe_hoe".into()));
    assert_eq!(game.state().npcs[0].pending_event, "ahe_hoe");

    let (reply, favor, coins) = choose(&mut game, "ahe_hoe", "steady");
    assert!(reply.contains("手比嘴稳"));
    assert_eq!(favor, 4);
    assert_eq!(coins, 300);
    assert_eq!(game.state().npcs[0].friendship, 35);
    assert_eq!(game.state().npcs[0].pending_event, "");
    assert_eq!(
        game.state().npcs[0].choices.get("ahe_hoe").map(String::as_str),
        Some("steady")
    );

    let (title, favor, coins) = talk(&mut game);
    assert_eq!(title, "闲聊");
    assert_eq!(favor, 0);
    assert_eq!(coins, 0);
    assert_eq!(game.state().npcs[0].friendship, 35);

    game.state_mut().daily.day = "2026-10-26".into();
    let (title, favor, coins) = talk(&mut game);
    assert_eq!(title, "闲聊");
    assert_eq!(favor, 1);
    assert_eq!(coins, 0);
    assert_eq!(game.state().npcs[0].friendship, 36);
}

#[test]
fn second_story_unlocks_at_sixty_and_never_repeats() {
    let mut game = setup();
    game.state_mut().npcs[0].friendship = 60;
    game.state_mut().npcs[0].seen_events = vec!["ahe_hoe".into()];
    let (title, favor, coins) = talk(&mut game);
    assert_eq!(title, "田埂上的话");
    assert_eq!(favor, 0);
    assert_eq!(coins, 0);
    assert_eq!(game.state().npcs[0].friendship, 61);

    let (reply, favor, coins) = choose(&mut game, "ahe_field", "stay");
    assert!(reply.contains("影子吹得很近"));
    assert_eq!(favor, 5);
    assert_eq!(coins, 600);
    assert_eq!(game.state().npcs[0].friendship, 66);

    game.state_mut().daily.day = "2026-10-27".into();
    let (title, favor, coins) = talk(&mut game);
    assert_eq!(title, "闲聊");
    assert_eq!(favor, 1);
    assert_eq!(coins, 0);
}

#[test]
fn anniversary_chat_uses_special_line() {
    let mut game = setup();
    let now = 1 + 29 * 86_400_000;
    game.state_mut().created_at = 1;
    game.state_mut().npcs[0].friendship = 35;
    game.state_mut().npcs[0].seen_events = vec!["ahe_hoe".into()];
    let outcome = game
        .dispatch(
            Action::ChatWithNpc {
                npc_id: "ahe".into(),
            },
            now,
        )
        .unwrap();
    let Effect::NpcTalk { text, tag, .. } = outcome.effects[0].clone() else {
        panic!("expected npc talk effect");
    };
    assert_eq!(tag, "来农场一个月");
    assert!(text.contains("一个月"));
}

#[test]
fn birthday_gift_can_only_be_given_once_a_year() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-04-12".into();
    game.state_mut().inventory.insert("crop:xiaobaicai".into(), 3);
    let before = game.state().coins;
    game.dispatch(
        Action::GiveNpcBirthdayGift {
            npc_id: "ahe".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().coins, before + 200);
    assert_eq!(game.state().npcs[0].friendship, 18);
    assert_eq!(game.state().npcs[0].birthday_gift_year, 2026);
    assert!(!game.state().inventory.contains_key("crop:xiaobaicai"));
    assert!(matches!(
        game.dispatch(
            Action::GiveNpcBirthdayGift {
                npc_id: "ahe".into(),
            },
            0
        ),
        Err(farm_core::GameError::NpcBirthdayGiftUnavailable)
    ));

    game.state_mut().daily.day = "2027-04-12".into();
    game.state_mut().inventory.insert("crop:xiaobaicai".into(), 3);
    game.dispatch(
        Action::GiveNpcBirthdayGift {
            npc_id: "ahe".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().npcs[0].birthday_gift_year, 2027);
}

#[test]
fn free_gift_is_once_per_day() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-25".into();
    game.state_mut().inventory.insert("crop:xiaobaicai".into(), 1);
    game.dispatch(
        Action::GiveNpcGift {
            npc_id: "ahe".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().npcs[0].friendship, 12);
    assert_eq!(game.state().npcs[0].last_gift_day, "2026-10-25");
    assert!(!game.state().inventory.contains_key("crop:xiaobaicai"));
    assert!(matches!(
        game.dispatch(
            Action::GiveNpcGift {
                npc_id: "ahe".into(),
            },
            0
        ),
        Err(farm_core::GameError::NpcGiftUnavailable)
    ));
}

#[test]
fn festival_can_be_celebrated_once_a_year() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-09-23".into();
    let before = game.state().coins;
    game.dispatch(Action::CelebrateFestival, 0).unwrap();
    assert_eq!(game.state().coins, before + 300);
    assert_eq!(
        game.state().festival_claims.get("丰收节").copied(),
        Some(2026)
    );
    assert!(matches!(
        game.dispatch(Action::CelebrateFestival, 0),
        Err(farm_core::GameError::FestivalUnavailable)
    ));

    game.state_mut().daily.day = "2027-09-23".into();
    game.dispatch(Action::CelebrateFestival, 0).unwrap();
    assert_eq!(
        game.state().festival_claims.get("丰收节").copied(),
        Some(2027)
    );
}

#[test]
fn weekly_request_consumes_items_and_leaves_souvenir() {
    let mut game = Game::new(0);
    game.state_mut().inventory.insert("crop:xiaobaicai".into(), 5);
    let before = game.state().coins;
    let outcome = game
        .dispatch(
            Action::HelpNpcRequest {
                npc_id: "ahe".into(),
            },
            0,
        )
        .unwrap();
    assert!(matches!(outcome.effects[0], Effect::Log { .. }));
    assert_eq!(game.state().coins, before + 200);
    assert!(!game.state().inventory.contains_key("crop:xiaobaicai"));
    assert_eq!(game.state().npcs[0].friendship, 13);
    assert!(game.state().npcs[0]
        .souvenirs
        .contains(&"一把扎好的小青菜".to_string()));
    assert!(matches!(
        game.dispatch(
            Action::HelpNpcRequest {
                npc_id: "ahe".into(),
            },
            0
        ),
        Err(farm_core::GameError::NpcRequestDone)
    ));

    game.state_mut().inventory.insert("crop:xiaobaicai".into(), 5);
    game.dispatch(
        Action::HelpNpcRequest {
            npc_id: "ahe".into(),
        },
        7 * 86_400_000,
    )
    .unwrap();
    assert_eq!(game.state().npcs[0].souvenirs.len(), 2);
}

#[test]
fn old_memory_is_recalled_with_tag_once_per_cooldown() {
    let mut game = setup();
    game.state_mut().npcs[0].friendship = 35;
    game.state_mut().npcs[0].seen_events = vec!["ahe_hoe".into()];
    game.state_mut().npcs[0].memories = vec![NpcMemory {
        event_id: "ahe_hoe".into(),
        day: -5,
        season: "秋".into(),
        last_recall_day: -10,
    }];
    let before = game.state().coins;
    let outcome = game
        .dispatch(
            Action::ChatWithNpc {
                npc_id: "ahe".into(),
            },
            0,
        )
        .unwrap();
    let Effect::NpcTalk {
        title,
        tag,
        favor,
        ..
    } = outcome.effects[0].clone()
    else {
        panic!("expected npc talk effect");
    };
    assert_eq!(title, "旧事");
    assert_eq!(tag, "记得");
    assert_eq!(favor, 1);
    assert_eq!(game.state().coins, before);
    assert_eq!(game.state().npcs[0].memories[0].last_recall_day, 0);

    let (title, _, _) = talk(&mut game);
    assert_eq!(title, "闲聊");
}
