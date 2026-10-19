use farm_core::{Action, Game};

#[test]
fn npc_ranch_help_and_steal_use_independent_counts() {
    let mut game = Game::new(0);
    let npc_id = game.state().npcs[0].id.clone();
    game.dispatch(
        Action::HelpNpcRanch {
            npc_id: npc_id.clone(),
            index: 1,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().daily.ranch_helps, 1);
    assert_eq!(game.state().daily.helps, 0);
    assert!(game.state().npcs[0].ranch[0].helped);

    game.state_mut().npcs[0].ranch[1].ready_at = 0;
    game.dispatch(Action::StealNpcRanch { npc_id, index: 2 }, 0)
        .unwrap();
    assert_eq!(game.state().daily.ranch_steals, 1);
    assert_eq!(game.state().daily.steals, 0);
    assert_eq!(game.state().inventory.get("ranch:duck_egg"), Some(&1));
}
