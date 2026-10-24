use farm_core::{Action, Game, GameError};

#[test]
fn mystery_shop_buy_and_plant_rare_seed() {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().coins = 1_000_000;
    game.sync(0, "2026-10-24");
    let offers = game.state().mystery.offers.clone();
    assert_eq!(offers.len(), 8);

    let seed = offers
        .iter()
        .find(|offer| offer.kind == farm_core::MysteryKind::Seed)
        .unwrap()
        .clone();
    let coins = game.state().coins;
    game.dispatch(
        Action::BuyMysteryOffer {
            id: seed.id.clone(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().coins < coins);
    assert!(game.state().mystery.offer(&seed.id).unwrap().sold);
    let key = format!("rare_seed:{}", seed.crop_id);
    assert_eq!(game.state().inventory.get(&key), Some(&1));
    assert!(matches!(
        game.dispatch(
            Action::BuyMysteryOffer {
                id: seed.id.clone(),
            },
            0,
        ),
        Err(GameError::MysteryOfferSold)
    ));

    game.dispatch(
        Action::PlantRareSeed {
            plot: 1,
            crop_id: seed.crop_id.clone(),
        },
        0,
    )
    .unwrap();
    let plot = &game.state().plots[0];
    assert_eq!(plot.crop.as_ref().unwrap().crop_id, seed.crop_id);
    assert_eq!(game.state().inventory.get(&key), None);
}

#[test]
fn coin_elixir_boosts_sell_price() {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().coins = 1_000_000;
    game.sync(0, "2026-10-24");
    let offer = game
        .state()
        .mystery
        .offers
        .iter()
        .find(|offer| offer.item_key == "craft:coin_elixir")
        .unwrap()
        .clone();
    game.dispatch(
        Action::BuyMysteryOffer {
            id: offer.id.clone(),
        },
        0,
    )
    .unwrap();
    game.dispatch(
        Action::UseBuffItem {
            key: "craft:coin_elixir".into(),
        },
        0,
    )
    .unwrap();
    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 1);
    let coins = game.state().coins;
    game.dispatch(
        Action::Sell {
            key: "crop:xiaobaicai".into(),
            qty: 1,
        },
        0,
    )
    .unwrap();
    // 小青菜 6 金币 × 1.25 = 7（向下取整）。
    assert_eq!(game.state().coins - coins, 7);
}

#[test]
fn mystery_buff_can_be_used() {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().coins = 1_000_000;
    game.sync(0, "2026-10-24");
    let buff = game
        .state()
        .mystery
        .offers
        .iter()
        .find(|offer| offer.kind == farm_core::MysteryKind::Buff)
        .unwrap()
        .clone();
    game.dispatch(
        Action::BuyMysteryOffer {
            id: buff.id.clone(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().inventory.get(&buff.item_key), Some(&1));
    game.dispatch(Action::UseBuffItem { key: buff.item_key }, 0)
        .unwrap();
    assert!(!game.snapshot(0).skills.buffs.is_empty());
}
