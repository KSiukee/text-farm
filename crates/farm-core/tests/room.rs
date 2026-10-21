use farm_core::{
    default_layout, validate_layout, Action, Game, GameError, RoomDoor, RoomFurniture, RoomLayout,
    RoomSurfaceItem, RoomWindow, LIVING_ROOM, LIVING_WIDTH, ROOM_GRID,
};

fn layout() -> RoomLayout {
    RoomLayout {
        floor: "wood".into(),
        wall: "cream".into(),
        door: None,
        windows: vec![],
        furniture: vec![RoomFurniture {
            id: "bed-1".into(),
            def: "bed".into(),
            x: 8,
            y: 8,
            rot: 0,
            fish: vec![],
        }],
        surface_items: vec![],
    }
}

#[test]
fn default_room_is_valid_and_visible() {
    let mut game = Game::new(1_000);
    let room = &game.snapshot(1_000).room;
    assert_eq!(room.room_id, LIVING_ROOM);
    assert_eq!(room.width, LIVING_WIDTH);
    assert_eq!(room.grid, ROOM_GRID);
    assert!(room.layout.furniture.len() >= 4);
    assert_eq!(room.catalog.len(), 16);
    assert!(validate_layout(LIVING_ROOM, &room.layout).is_ok());

    let saved = layout();
    game.dispatch(
        Action::SaveRoomLayout {
            room: LIVING_ROOM.into(),
            layout: saved.clone(),
        },
        2_000,
    )
    .unwrap();
    let after = &game.snapshot(2_000).room;
    assert_eq!(after.layout, saved);
}

#[test]
fn room_styles_unlock_from_level_achievement_days_and_craft() {
    let mut game = Game::new(1_000);
    let now = 1_000;
    let before = game.snapshot(now);
    assert!(before.room.floors.iter().find(|style| style.id == "terracotta").unwrap().locked);
    assert!(before.room.floors.iter().find(|style| style.id == "tatami").unwrap().locked);

    game.state_mut().level = 20;
    game.state_mut().achievements.insert("harvest_100".into(), true);
    game.state_mut().inventory.insert("craft:coffee_powder".into(), 1);
    game.state_mut().created_at = now - 60 * 86_400_000;
    let after = game.snapshot(now);
    for (list, id) in [
        (&after.room.floors, "terracotta"),
        (&after.room.floors, "tatami"),
        (&after.room.floors, "cafe"),
        (&after.room.walls, "dusk"),
        (&after.room.walls, "linen"),
    ] {
        let style = list.iter().find(|style| style.id == id).unwrap();
        assert!(!style.locked, "{id} should be unlocked");
    }
}

#[test]
fn table_lamp_must_sit_on_desk_or_nightstand() {
    let mut room = default_layout();
    room.surface_items.push(RoomSurfaceItem {
        id: "lamp-1".into(),
        def: "table_lamp".into(),
        support_id: "starter-desk".into(),
    });
    assert!(validate_layout(LIVING_ROOM, &room).is_ok());

    room.surface_items[0].support_id = "starter-chair".into();
    assert!(validate_layout(LIVING_ROOM, &room).is_err());
}

#[test]
fn npc_rooms_are_read_only_views_with_thresholds() {
    let game = Game::new(1_000);
    let snap = game.snapshot(1_000);
    assert_eq!(snap.npc_rooms.len(), snap.npcs.len());
    let ahe = snap.npc_rooms.iter().find(|room| room.npc_id == "ahe").unwrap();
    assert!(ahe.unlocked);
    assert_eq!(ahe.room.width, 12);
    assert_eq!(ahe.room.height, 9);
    assert!(!ahe.room.layout.furniture.is_empty());
    let xiaoman = snap.npc_rooms.iter().find(|room| room.npc_id == "xiaoman").unwrap();
    assert!(!xiaoman.unlocked);
}


#[test]
fn aquarium_moves_fish_between_tank_and_inventory() {
    let mut game = Game::new(1_000);
    let mut room = layout();
    room.furniture.push(RoomFurniture {
        id: "tank".into(),
        def: "aquarium_small".into(),
        x: 0,
        y: 0,
        rot: 0,
        fish: vec![],
    });
    game.dispatch(
        Action::SaveRoomLayout {
            room: LIVING_ROOM.into(),
            layout: room,
        },
        2_000,
    )
    .unwrap();
    game.state_mut().inventory.insert("fish:carp".into(), 1);

    game.dispatch(
        Action::MoveFishToAquarium {
            room: LIVING_ROOM.into(),
            furniture_id: "tank".into(),
            fish_id: "carp".into(),
        },
        3_000,
    )
    .unwrap();
    let snap = game.snapshot(3_000);
    let tank = snap.room.layout.furniture.iter().find(|item| item.id == "tank").unwrap();
    assert_eq!(tank.fish, vec!["carp".to_string()]);
    assert!(!snap.inventory.iter().any(|item| item.key == "fish:carp"));

    game.dispatch(
        Action::RemoveFishFromAquarium {
            room: LIVING_ROOM.into(),
            furniture_id: "tank".into(),
            fish_id: "carp".into(),
        },
        4_000,
    )
    .unwrap();
    let snap = game.snapshot(4_000);
    let tank = snap.room.layout.furniture.iter().find(|item| item.id == "tank").unwrap();
    assert!(tank.fish.is_empty());
    assert_eq!(
        snap.inventory.iter().find(|item| item.key == "fish:carp").map(|item| item.qty),
        Some(1)
    );
}


#[test]
fn extra_room_unlocks_once_with_door_and_cost() {
    let mut game = Game::new(1_000);
    game.state_mut().coins = 30_000;
    assert!(game.snapshot(1_000).extra_room.is_none());
    game.dispatch(Action::UnlockExtraRoom, 2_000).unwrap();
    let snap = game.snapshot(2_000);
    let extra = snap.extra_room.expect("extra room");
    assert_eq!(extra.width, 12);
    assert_eq!(extra.height, 9);
    assert!(extra.layout.door.is_some());
    assert_eq!(snap.coins, 10_000);
    assert!(matches!(
        game.dispatch(Action::UnlockExtraRoom, 3_000),
        Err(GameError::RoomLayoutInvalid(_))
    ));
}

#[test]
fn door_cannot_overlap_window() {
    let mut layout = default_layout();
    layout.door = Some(RoomDoor {
        id: "door".into(),
        wall: "right".into(),
        x: 64,
        w: 16,
        h: 32,
        style: "wood".into(),
    });
    assert!(validate_layout(LIVING_ROOM, &layout).is_err());
}

#[test]
fn room_windows_are_bounded_and_do_not_overlap() {
    let mut good = layout();
    good.windows.push(RoomWindow {
        id: "win-1".into(),
        wall: "right".into(),
        x: 24,
        y: 24,
        w: 24,
        h: 20,
        style: "large".into(),
    });
    assert!(validate_layout(LIVING_ROOM, &good).is_ok());

    let mut out = good.clone();
    out.windows[0].x = LIVING_WIDTH * ROOM_GRID - 8;
    assert!(validate_layout(LIVING_ROOM, &out).is_err());

    let mut style = good.clone();
    style.windows[0].style = "nope".into();
    assert!(validate_layout(LIVING_ROOM, &style).is_err());

    let mut overlap = good.clone();
    overlap.windows.push(RoomWindow {
        id: "win-2".into(),
        wall: "right".into(),
        x: 30,
        y: 28,
        w: 16,
        h: 16,
        style: "small".into(),
    });
    assert!(validate_layout(LIVING_ROOM, &overlap).is_err());

    let mut left = good.clone();
    left.windows.push(RoomWindow {
        id: "win-left".into(),
        wall: "left".into(),
        x: 24,
        y: 24,
        w: 24,
        h: 20,
        style: "large".into(),
    });
    assert!(validate_layout(LIVING_ROOM, &left).is_ok());
}

#[test]
fn room_rejects_bad_style_bounds_and_overlap() {
    let mut bad_style = layout();
    bad_style.floor = "nope".into();
    assert!(validate_layout(LIVING_ROOM, &bad_style).is_err());

    let mut out = layout();
    out.furniture[0].x = LIVING_WIDTH * ROOM_GRID;
    assert!(validate_layout(LIVING_ROOM, &out).is_err());

    let mut overlap = layout();
    overlap.furniture.push(RoomFurniture {
        id: "bed-2".into(),
        def: "bed".into(),
        x: 8,
        y: 8,
        rot: 0,
        fish: vec![],
    });
    assert!(validate_layout(LIVING_ROOM, &overlap).is_err());

    let mut duplicate = layout();
    duplicate.furniture.push(RoomFurniture {
        id: "bed-1".into(),
        def: "chair".into(),
        x: 40,
        y: 8,
        rot: 0,
        fish: vec![],
    });
    assert!(validate_layout(LIVING_ROOM, &duplicate).is_err());

    let mut game = Game::new(1_000);
    assert!(matches!(
        game.dispatch(
            Action::SaveRoomLayout {
                room: "extra".into(),
                layout: default_layout(),
            },
            2_000
        ),
        Err(GameError::RoomLayoutInvalid(_))
    ));
}
