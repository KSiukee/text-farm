use farm_core::{Action, Effect, Game};

fn next_day(month: &mut u32, day: &mut u32) -> String {
    let lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    *day += 1;
    if *day > lengths[(*month - 1) as usize] {
        *day = 1;
        *month += 1;
        if *month > 12 {
            *month = 1;
        }
    }
    format!("2026-{:02}-{:02}", month, day)
}

#[test]
fn ninety_days_of_npc_life_stays_healthy() {
    let mut game = Game::new(0);
    let npc_ids: Vec<String> = game.state().npcs.iter().map(|npc| npc.id.clone()).collect();
    let (mut month, mut day) = (1, 1);
    for offset in 0..90i64 {
        let today = next_day(&mut month, &mut day);
        let now = offset * 86_400_000;
        game.sync(now, &today);
        for npc_id in &npc_ids {
            let outcome = game
                .dispatch(
                    Action::ChatWithNpc {
                        npc_id: npc_id.clone(),
                    },
                    now,
                )
                .expect("chat should not fail");
            let first = outcome.effects.first().cloned();
            let choice = match first {
                Some(Effect::NpcTalk { choices, .. }) => {
                    choices.first().map(|choice| choice.id.clone())
                }
                _ => None,
            };
            if let Some(choice_id) = choice {
                let event_id = game
                    .state()
                    .npcs
                    .iter()
                    .find(|npc| npc.id == *npc_id)
                    .map(|npc| npc.pending_event.clone())
                    .unwrap_or_default();
                game.dispatch(
                    Action::NpcStoryChoice {
                        npc_id: npc_id.clone(),
                        event_id,
                        choice_id,
                    },
                    now,
                )
                .expect("choice should not fail");
            }
        }
    }
    assert!(game
        .state()
        .npcs
        .iter()
        .all(|npc| npc.pending_event.is_empty()));
    assert!(game.state().npcs.iter().all(|npc| npc.memories.len() >= 2));
    assert!(game
        .state()
        .npcs
        .iter()
        .all(|npc| npc.friendship >= 60));
}
