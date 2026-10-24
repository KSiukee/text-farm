use farm_core::{Action, Game, GameError};

#[test]
fn farm_order_accept_deliver_and_expire() {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().coins = 0;
    game.sync(0, "2026-10-24");
    let offers = game.state().orders.offers.clone();
    assert_eq!(offers.len(), 3);

    let first = offers[0].clone();
    game.dispatch(
        Action::AcceptFarmOrder {
            id: first.id.clone(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().orders.active.len(), 1);

    // 不够就交不了
    assert!(matches!(
        game.dispatch(
            Action::DeliverFarmOrder {
                id: first.id.clone(),
            },
            0,
        ),
        Err(GameError::FarmOrderNotEnough)
    ));

    game.state_mut()
        .inventory
        .insert(format!("crop:{}", first.crop_id), first.qty);
    let coins = game.state().coins;
    game.dispatch(
        Action::DeliverFarmOrder {
            id: first.id.clone(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().coins > coins);
    assert!(game.state().orders.active.is_empty());
    assert_eq!(
        game.state()
            .inventory
            .get(&format!("crop:{}", first.crop_id)),
        None
    );
    assert!(matches!(
        game.dispatch(
            Action::DeliverFarmOrder {
                id: first.id.clone(),
            },
            0,
        ),
        Err(GameError::FarmOrderInvalid)
    ));

    // 接满 2 条后，第二天换新报价，仍然拒绝第 3 条
    let rest = game.state().orders.offers.clone();
    game.dispatch(
        Action::AcceptFarmOrder {
            id: rest[0].id.clone(),
        },
        0,
    )
    .unwrap();
    game.dispatch(
        Action::AcceptFarmOrder {
            id: rest[1].id.clone(),
        },
        0,
    )
    .unwrap();
    game.sync(24 * 60 * 60 * 1000, "2026-10-25");
    let fresh = game.state().orders.offers.clone();
    assert_eq!(fresh.len(), 3);
    assert!(matches!(
        game.dispatch(
            Action::AcceptFarmOrder {
                id: fresh[0].id.clone(),
            },
            24 * 60 * 60 * 1000,
        ),
        Err(GameError::FarmOrderLimit)
    ));

    // 隔几天不交，订单过期，NPC 友好度下降
    let npc_id = game.state().orders.active[0].npc_id.clone();
    let friendship = game
        .state()
        .npcs
        .iter()
        .find(|npc| npc.id == npc_id)
        .map(|npc| npc.friendship)
        .unwrap_or(0);
    game.sync(3 * 24 * 60 * 60 * 1000, "2026-10-29");
    assert!(game.state().orders.active.is_empty());
    let after = game
        .state()
        .npcs
        .iter()
        .find(|npc| npc.id == npc_id)
        .map(|npc| npc.friendship)
        .unwrap_or(0);
    assert!(after < friendship);
}
