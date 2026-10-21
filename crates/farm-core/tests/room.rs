use farm_core::{
    default_layout, validate_layout, Action, Game, GameError, RoomFurniture, RoomLayout,
    LIVING_ROOM, LIVING_WIDTH, ROOM_GRID,
};

fn layout() -> RoomLayout {
    RoomLayout {
        floor: "wood".into(),
        wall: "cream".into(),
        furniture: vec![RoomFurniture {
            id: "bed-1".into(),
            def: "bed".into(),
            x: 8,
            y: 8,
            rot: 0,
        }],
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
    assert_eq!(room.catalog.len(), 6);
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
    });
    assert!(validate_layout(LIVING_ROOM, &overlap).is_err());

    let mut duplicate = layout();
    duplicate.furniture.push(RoomFurniture {
        id: "bed-1".into(),
        def: "chair".into(),
        x: 40,
        y: 8,
        rot: 0,
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
