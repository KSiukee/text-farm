use farm_core::{Action, Game};

#[test]
fn market_prepares_then_opens_and_takes_supplies() {
    let mut game = Game::new(0);
    game.sync(0, "2026-09-21");
    let market = game.snapshot(0).festival_market.expect("集市应出现");
    assert_eq!(market.phase, "prepare");
    assert_eq!(market.days_until, 2);
    assert_eq!(market.requests.len(), 3);

    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 10);
    game.dispatch(
        Action::SupplyFestivalMarket {
            request_id: "r0".into(),
        },
        0,
    )
    .unwrap();
    let market = game.snapshot(0).festival_market.unwrap();
    assert_eq!(market.contribution, 2);
    assert!(market.requests[0].supplied);

    game.sync(0, "2026-09-23");
    let market = game.snapshot(0).festival_market.unwrap();
    assert_eq!(market.phase, "open");
    assert_eq!(market.days_until, 0);
}

#[test]
fn market_offers_and_gifts_are_limited() {
    let mut game = Game::new(0);
    game.sync(0, "2026-09-21");
    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 10);
    game.state_mut().inventory.insert("crop:luobo".into(), 10);
    game.state_mut()
        .inventory
        .insert("seed:xiaobaicai".into(), 10);
    for request in ["r0", "r1", "r2"] {
        game.dispatch(
            Action::SupplyFestivalMarket {
                request_id: request.into(),
            },
            0,
        )
        .unwrap();
    }
    assert_eq!(
        game.snapshot(0).festival_market.unwrap().contribution,
        9
    );
    game.sync(0, "2026-09-23");

    let offer_id = game.snapshot(0).festival_market.unwrap().offers[0]
        .id
        .clone();
    let before = game.state().coins;
    game.dispatch(
        Action::BuyFestivalOffer {
            offer_id: offer_id.clone(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().coins < before);
    assert!(matches!(
        game.dispatch(
            Action::BuyFestivalOffer {
                offer_id: offer_id.into(),
            },
            0
        ),
        Err(farm_core::GameError::FestivalMarketOfferDone)
    ));

    let coins = game.state().coins;
    game.state_mut().inventory.insert("crop:luobo".into(), 1);
    game.dispatch(
        Action::SellFestivalGoods {
            key: "crop:luobo".into(),
        },
        0,
    )
    .unwrap();
    assert!(game.state().coins > coins);

    for threshold in [3, 6, 9] {
        game.dispatch(Action::ClaimFestivalGift { threshold }, 0)
            .unwrap();
        assert!(matches!(
            game.dispatch(Action::ClaimFestivalGift { threshold }, 0),
            Err(farm_core::GameError::FestivalMarketGiftClaimed)
        ));
    }
}

#[test]
fn national_day_is_a_major_festival_with_five_day_prep() {
    assert_eq!(
        farm_core::festival_of("2026-10-01"),
        Some("国庆节")
    );
    let mut game = Game::new(0);
    game.sync(0, "2026-09-27");
    let market = game.snapshot(0).festival_market.expect("国庆大集应出现");
    assert!(market.major);
    assert_eq!(market.days_until, 4);
    assert_eq!(market.requests.len(), 5);
    assert_eq!(market.max_sells, 5);
    assert!(market.crowd.len() >= 10);
    assert!(!market.offers[0].vendor.is_empty());
    assert!(!market.offers[0].stall.is_empty());
}

#[test]
fn spring_festival_and_lantern_festival_follow_lunar_dates() {
    assert_eq!(
        farm_core::festival_of("2026-02-17"),
        Some("春节")
    );
    assert_eq!(
        farm_core::festival_of("2026-03-03"),
        Some("元宵节")
    );
    assert_eq!(
        farm_core::festival_of("2027-02-06"),
        Some("春节")
    );
}

#[test]
fn solar_festivals_follow_the_year_table() {
    assert_eq!(
        farm_core::festival_of("2024-09-22"),
        Some("丰收节")
    );
    assert_eq!(farm_core::festival_of("2024-09-23"), None);
    assert_eq!(
        farm_core::festival_of("2027-03-21"),
        Some("播种节")
    );
    assert_eq!(
        farm_core::festival_of("2026-12-22"),
        Some("围炉日")
    );
}


