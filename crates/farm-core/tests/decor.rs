use farm_core::{
    default_layout, validate_ownership, Action, Game, RoomFurniture, RoomLayout, RoomState,
    LIVING_ROOM,
};

fn room_game() -> Game {
    let mut game = Game::new(0);
    game.state_mut().coins = 100_000;
    game.sync(0, "2026-10-24");
    game
}

#[test]
fn furniture_is_bought_not_free() {
    let mut game = room_game();
    let before = game.state().coins;
    let before_owned = game.state().room.owned_furniture.get("chair").copied().unwrap_or(0);
    game.dispatch(
        Action::BuyFurniture {
            def: "chair".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().coins, before - 800);
    assert_eq!(
        game.state().room.owned_furniture.get("chair"),
        Some(&(before_owned + 1))
    );

    let mut layout = default_layout();
    layout.furniture.push(RoomFurniture {
        id: "test-chair".into(),
        def: "chair".into(),
        x: 8,
        y: 8,
        rot: 0,
        fish: vec![],
    });
    validate_ownership(LIVING_ROOM, &layout, &game.state().room).unwrap();
}

#[test]
fn decor_styles_are_bought_and_unlock_room() {
    let mut game = room_game();
    let offer = game
        .state()
        .room
        .decor_shop
        .offers
        .iter()
        .find(|offer| !offer.sold)
        .unwrap()
        .clone();
    let before = game.state().coins;
    game.dispatch(Action::BuyDecor { id: offer.id.clone() }, 0)
        .unwrap();
    assert_eq!(game.state().coins, before - offer.price);
    let owned = match offer.kind {
        farm_core::DecorKind::Floor => game.state().room.owned_floors.contains(&offer.item_id),
        farm_core::DecorKind::Wall => game.state().room.owned_walls.contains(&offer.item_id),
    };
    assert!(owned);

    let mut layout = default_layout();
    match offer.kind {
        farm_core::DecorKind::Floor => layout.floor = offer.item_id.clone(),
        farm_core::DecorKind::Wall => layout.wall = offer.item_id.clone(),
    }
    validate_ownership(LIVING_ROOM, &layout, &game.state().room).unwrap();
}

#[test]
fn mystery_furniture_goes_to_owned_not_inventory() {
    let mut game = room_game();
    let offer = game
        .state()
        .mystery
        .offers
        .iter()
        .find(|offer| offer.kind == farm_core::MysteryKind::Furniture)
        .unwrap()
        .clone();
    let before = game
        .state()
        .room
        .owned_furniture
        .get(offer.item_key.strip_prefix("furniture:").unwrap())
        .copied()
        .unwrap_or(0);
    game.dispatch(Action::BuyMysteryOffer { id: offer.id }, 0)
        .unwrap();
    let def = offer.item_key.strip_prefix("furniture:").unwrap();
    assert_eq!(
        game.state().room.owned_furniture.get(def),
        Some(&(before + 1))
    );
    assert!(!game.state().inventory.contains_key(&offer.item_key));
}

#[test]
fn level_gift_adds_bound_furniture() {
    let mut game = Game::new(0);
    game.state_mut().level = 10;
    game.dispatch(Action::ClaimLevelReward { level: 10 }, 0)
        .unwrap();
    assert_eq!(game.state().room.owned_furniture.get("gift_quilt"), Some(&1));
}

#[test]
fn old_claimed_level_gift_is_backfilled() {
    let mut game = Game::new(0);
    game.state_mut().level = 12;
    game.state_mut().level_rewards_claimed.insert(10);
    game.state_mut().room.owned_furniture.remove("gift_quilt");
    game.sync(0, "2026-10-24");
    assert_eq!(game.state().room.owned_furniture.get("gift_quilt"), Some(&1));
}

#[test]
fn achievement_and_anniversary_gifts_are_granted_once() {
    let mut game = Game::new(0);
    game.state_mut().achievements.insert("sign_in_7".into(), true);
    game.state_mut().created_at = -31 * 86_400_000;
    game.sync(0, "2026-10-24");
    assert_eq!(
        game.state().room.owned_furniture.get("gift_signin_lantern"),
        Some(&1)
    );
    assert_eq!(
        game.state().room.owned_furniture.get("gift_anniversary_quilt"),
        Some(&1)
    );
    game.sync(1_000, "2026-10-24");
    assert_eq!(
        game.state().room.owned_furniture.get("gift_signin_lantern"),
        Some(&1)
    );
}

#[test]
fn old_room_ownership_backfills_from_layout() {
    let mut state = RoomState::default();
    state.ownership_initialized = false;
    state.owned_furniture.clear();
    state.owned_floors.clear();
    state.owned_walls.clear();
    state.ensure_ownership();
    assert!(!state.owned_furniture.is_empty());
    assert!(state.owned_floors.contains("wood"));
    assert!(state.owned_walls.contains("cream"));
}

#[test]
fn unbought_furniture_cannot_be_placed() {
    let game = room_game();
    let mut layout: RoomLayout = default_layout();
    layout.furniture.push(RoomFurniture {
        id: "test-tv".into(),
        def: "tv".into(),
        x: 8,
        y: 8,
        rot: 0,
        fish: vec![],
    });
    assert!(validate_ownership(LIVING_ROOM, &layout, &game.state().room).is_err());
}
