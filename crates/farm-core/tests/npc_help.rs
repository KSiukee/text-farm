use farm_core::{Action, Game, GameError};

#[test]
fn asking_xiaoman_for_help_gives_a_buff_once_a_day() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-25".into();
    game.dispatch(
        Action::AskNpcHelp {
            npc_id: "xiaoman".into(),
        },
        0,
    )
    .unwrap();
    assert!(game
        .state()
        .buffs
        .iter()
        .any(|buff| buff.id == "npc_help_xiaoman"));
    assert_eq!(game.state().npcs[1].last_help_day, "2026-10-25");
    assert!(matches!(
        game.dispatch(
            Action::AskNpcHelp {
                npc_id: "xiaoman".into(),
            },
            0
        ),
        Err(GameError::NpcHelpDone)
    ));
}

#[test]
fn ahe_waters_growing_plots() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-25".into();
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    let before = game.state().plots[0].crop.as_ref().unwrap().ready_at;
    game.dispatch(
        Action::AskNpcHelp {
            npc_id: "ahe".into(),
        },
        0,
    )
    .unwrap();
    let crop = game.state().plots[0].crop.as_ref().unwrap();
    assert!(crop.watered);
    assert!(crop.ready_at < before);
}
