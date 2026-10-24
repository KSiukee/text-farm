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
