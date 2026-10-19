use farm_core::{Action, ContentPack, Game, GameError};

#[test]
fn new_game_has_five_neighbors_with_one_ready_plot() {
    let game = Game::new(0);
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.npcs.len(), 5);
    assert_eq!(snapshot.npcs[0].id, "ahe");
    assert_eq!(snapshot.npcs[0].plots[0].status, "ready");
    assert!(snapshot.npcs[0].plots[0].can_steal);
    assert_eq!(snapshot.npcs[0].plots[0].label, "土坑1");
    assert_eq!(snapshot.npcs[1].plots[0].label, "土坑1");
    assert!(snapshot.npcs[1].plots[0].label != snapshot.npcs[1].plots[1].label);
    assert_eq!(snapshot.steals_left, 5);
}

#[test]
fn helping_a_neighbor_raises_friendship_and_uses_daily_quota() {
    let mut game = Game::new(0);
    let before = game.state().npcs[0].friendship;
    game.dispatch(
        Action::HelpNpc {
            npc_id: "ahe".into(),
            plot: 2,
        },
        0,
    )
    .unwrap();

    assert_eq!(game.state().npcs[0].friendship, before + 2);
    assert_eq!(game.state().daily.helps, 1);
    assert_eq!(game.snapshot(0).helps_left, 9);
    assert!(matches!(
        game.dispatch(
            Action::HelpNpc {
                npc_id: "ahe".into(),
                plot: 2
            },
            0
        ),
        Err(GameError::NpcAlreadyHelped)
    ));
}

#[test]
fn stealing_takes_fruit_but_leaves_some_and_lowers_friendship() {
    let mut game = Game::new(0);
    let before = game.state().npcs[0].friendship;
    game.dispatch(
        Action::StealNpc {
            npc_id: "ahe".into(),
            plot: 1,
        },
        0,
    )
    .unwrap();

    let snapshot = game.snapshot(0);
    assert!(snapshot
        .inventory
        .iter()
        .any(|item| item.key == "crop:xiaobaicai" && item.qty == 1));
    assert_eq!(game.state().npcs[0].friendship, before - 2);
    assert_eq!(snapshot.steals_left, 4);
    assert!(!snapshot.npcs[0].plots[0].can_steal);
}

#[test]
fn stealing_stops_at_the_daily_limit() {
    let mut state = Game::new(0).state().clone();
    state.daily.steals = 5;
    let mut game = Game::from_parts(state, ContentPack::starter());
    assert!(matches!(
        game.dispatch(
            Action::StealNpc {
                npc_id: "ahe".into(),
                plot: 1
            },
            0
        ),
        Err(GameError::NpcStealLimit)
    ));
}

#[test]
fn greeting_leaves_player_and_npc_messages() {
    let mut game = Game::new(0);
    game.dispatch(
        Action::NpcGreeting {
            npc_id: "ahe".into(),
        },
        0,
    )
    .unwrap();
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.npc_messages.len(), 2);
    assert!(snapshot.npc_messages.iter().any(|m| m.from_player));
    assert!(snapshot.npc_messages.iter().any(|m| !m.from_player));
}

#[test]
fn sync_brings_at_most_three_visits_after_a_long_gap() {
    let mut state = Game::new(0).state().clone();
    state.last_npc_tick_at = 0;
    let mut game = Game::from_parts(state, ContentPack::starter());
    game.sync(5 * 3_600_000, "2026-10-18");
    let visits = game
        .snapshot(5 * 3_600_000)
        .npc_messages
        .iter()
        .filter(|m| !m.from_player)
        .count();
    assert!(visits <= 3);
    assert!(visits >= 1);
}

#[test]
fn ensure_npcs_normalizes_plot_indices() {
    let mut state = Game::new(0).state().clone();
    state.npcs[1].plots[0].index = 8;
    let mut game = Game::from_parts(state, ContentPack::starter());
    game.ensure_npcs(0);
    assert_eq!(game.state().npcs[1].plots[0].index, 1);
    assert_eq!(game.state().npcs[1].plots[4].index, 5);
}
