use farm_core::{Action, EquipmentKind, EquipmentRecipe, Game, WorkshopJob};

fn workshop_game() -> Game {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.dispatch(
        Action::InstallEquipment {
            slot: 1,
            kind: EquipmentKind::Juicer,
        },
        0,
    )
    .unwrap();
    game.state_mut().daily.day = "2026-10-24".into();
    *game
        .state_mut()
        .inventory
        .entry("crop:caomei".into())
        .or_insert(0) += 4;
    game
}

fn craft_order() -> farm_core::FarmOrder {
    let recipe = EquipmentRecipe::StrawberryJuice;
    farm_core::FarmOrder {
        id: "test-craft".into(),
        npc_id: "xiaoman".into(),
        npc_name: "小满".into(),
        crop_id: String::new(),
        crop_name: recipe.label().into(),
        qty: 1,
        expires_day: "2026-10-26".into(),
        reward_coins: 500,
        reward_exp: 20,
        item_key: recipe.output_key().into(),
        item_name: recipe.label().into(),
        recipe: Some(recipe),
    }
}

#[test]
fn craft_order_can_be_sent_to_workshop_and_delivered() {
    let mut game = workshop_game();
    game.state_mut().orders.active.push(craft_order());

    let before_start = game.snapshot(0);
    let tracked = before_start
        .orders
        .active
        .iter()
        .find(|order| order.id == "test-craft")
        .expect("tracked order");
    let craft = tracked.craft.as_ref().expect("craft track");
    assert_eq!(craft.status, "可加工");
    assert!(craft.can_start);
    assert_eq!(craft.inputs[0].name, "草莓");
    assert_eq!(craft.inputs[0].have, 4);

    game.dispatch(
        Action::StartOrderCraft {
            id: "test-craft".into(),
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.state().workshop.job(1),
        Some(WorkshopJob::Equipment {
            recipe: EquipmentRecipe::StrawberryJuice,
            ..
        })
    ));
    let after_start = game.snapshot(0);
    let tracked = after_start
        .orders
        .active
        .iter()
        .find(|order| order.id == "test-craft")
        .expect("tracked order");
    assert_eq!(tracked.craft.as_ref().unwrap().status, "加工中");

    if let Some(Some(job)) = game.state_mut().workshop.job_mut(1) {
        if let WorkshopJob::Equipment { ready_at, .. } = job {
            *ready_at = 0;
        }
    }
    game.dispatch(Action::CollectCraft { slot: 1 }, 0)
        .unwrap();
    assert!(game
        .state()
        .inventory
        .contains_key("craft:strawberry_juice"));

    let before = game.state().coins;
    game.dispatch(
        Action::DeliverFarmOrder {
            id: "test-craft".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().coins, before + 500);
    assert!(game.state().orders.active.is_empty());
    assert!(!game
        .state()
        .inventory
        .contains_key("craft:strawberry_juice"));
}
