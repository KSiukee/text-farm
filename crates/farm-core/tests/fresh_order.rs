use farm_core::{Action, FarmOrder, Game, GameError};

fn fresh_order() -> FarmOrder {
    FarmOrder {
        id: "fresh-1".into(),
        npc_id: "ahe".into(),
        npc_name: "阿禾".into(),
        crop_id: "xiaobaicai".into(),
        crop_name: "小青菜".into(),
        qty: 2,
        expires_day: "2026-10-27".into(),
        reward_coins: 200,
        reward_exp: 20,
        item_key: "crop:xiaobaicai".into(),
        item_name: "小青菜".into(),
        recipe: None,
        auto: false,
        favorite: true,
        fresh: true,
    }
}

#[test]
fn fresh_order_needs_todays_harvest() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-25".into();
    game.state_mut().inventory.insert("crop:xiaobaicai".into(), 5);
    game.state_mut().orders.active.push(fresh_order());

    assert!(matches!(
        game.dispatch(
            Action::DeliverFarmOrder {
                id: "fresh-1".into()
            },
            0
        ),
        Err(GameError::FarmOrderNotFresh)
    ));

    game.state_mut()
        .crop_fresh_day
        .insert("xiaobaicai".into(), "2026-10-25".into());
    game.dispatch(
        Action::DeliverFarmOrder {
            id: "fresh-1".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.state().inventory.get("crop:xiaobaicai").copied(),
        Some(3)
    );
}

#[test]
fn harvest_marks_the_crop_as_fresh_today() {
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
    game.state_mut().plots[0].crop.as_mut().unwrap().ready_at = 0;
    game.dispatch(Action::Harvest { plot: 1 }, 0).unwrap();
    assert_eq!(
        game.state().crop_fresh_day.get("xiaobaicai").map(String::as_str),
        Some("2026-10-25")
    );
}
