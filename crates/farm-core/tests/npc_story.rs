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

#[test]
fn chat_once_per_day_and_unlocks_first_story() {
    let mut game = setup();
    let (title, favor, coins) = talk(&mut game);
    assert_eq!(title, "老锄头");
    assert_eq!(favor, 4);
    assert_eq!(coins, 300);
    assert_eq!(game.state().npcs[0].friendship, 35);
    assert!(game.state().npcs[0].seen_events.contains(&"ahe_hoe".into()));

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
