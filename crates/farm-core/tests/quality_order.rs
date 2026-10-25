use farm_core::{Action, FarmOrder, Game, GameError};

fn quality_order() -> FarmOrder {
    FarmOrder {
        id: "quality-1".into(),
        npc_id: "xiaoman".into(),
        npc_name: "小满".into(),
        crop_id: "caomei".into(),
        crop_name: "草莓".into(),
        qty: 2,
        expires_day: "2026-10-27".into(),
        reward_coins: 300,
        reward_exp: 30,
        item_key: "crop:caomei".into(),
        item_name: "草莓".into(),
        recipe: None,
        auto: false,
        favorite: true,
        fresh: false,
        min_quality: 2,
    }
}

#[test]
fn quality_order_needs_todays_fine_harvest() {
    let mut game = Game::new(0);
    game.state_mut().daily.day = "2026-10-25".into();
    game.state_mut().inventory.insert("crop:caomei".into(), 5);
    game.state_mut().orders.active.push(quality_order());

    assert!(matches!(
        game.dispatch(
            Action::DeliverFarmOrder {
                id: "quality-1".into()
            },
            0
        ),
        Err(GameError::FarmOrderNotQuality)
    ));

    game.state_mut().crop_quality.insert(
        "caomei".into(),
        farm_core::CropQualityMark {
            day: "2026-10-25".into(),
            quality: 2,
        },
    );
    game.dispatch(
        Action::DeliverFarmOrder {
            id: "quality-1".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().inventory.get("crop:caomei").copied(), Some(3));
}
