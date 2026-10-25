use farm_core::{Action, Game, GameError};

fn goal_done(game: &Game, id: &str) -> bool {
    game.snapshot(0)
        .handbook
        .goals
        .iter()
        .find(|goal| goal.id == id)
        .map(|goal| goal.done)
        .unwrap_or(false)
}

#[test]
fn handbook_checks_existing_progress_and_claims_once() {
    let mut game = Game::new(0);
    let handbook = &game.snapshot(0).handbook;
    assert_eq!(handbook.goals.len(), 8);
    assert_eq!(handbook.done, 0);

    game.state_mut().stats.harvests = 1;
    assert!(goal_done(&game, "first_harvest"));
    let before = game.state().coins;
    game.dispatch(
        Action::ClaimHandbookGoal {
            id: "first_harvest".into(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().coins >= before + 50);
    assert!(matches!(
        game.dispatch(
            Action::ClaimHandbookGoal {
                id: "first_harvest".into(),
            },
            0
        ),
        Err(GameError::HandbookGoalUnavailable)
    ));

    game.state_mut().stats.planted = 4;
    let seeds = game
        .state()
        .inventory
        .get("seed:xiaobaicai")
        .copied()
        .unwrap_or(0);
    game.dispatch(
        Action::ClaimHandbookGoal {
            id: "four_plots".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.state().inventory.get("seed:xiaobaicai").copied(),
        Some(seeds + 1)
    );
}

#[test]
fn handbook_can_see_neighbors_orders_and_plots() {
    let mut game = Game::new(0);
    game.state_mut().stats.orders_delivered = 1;
    assert!(goal_done(&game, "first_order"));

    game.state_mut()
        .plots
        .iter_mut()
        .find(|plot| plot.index == 5)
        .unwrap()
        .unlocked = true;
    assert!(goal_done(&game, "fifth_plot"));

    for npc in &mut game.state_mut().npcs {
        npc.last_chat_day = "2026-10-25".into();
    }
    assert!(goal_done(&game, "five_neighbors"));
}
