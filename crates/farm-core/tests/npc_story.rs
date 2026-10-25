use farm_core::{Action, Effect, Game};

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
