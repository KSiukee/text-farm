use farm_core::{Action, Game, GameError};

#[test]
fn profile_rename_is_trimmed_and_validated() {
    let mut game = Game::new(0);
    assert_eq!(game.snapshot(0).owner_name, game.state().owner_name);
    game.dispatch(
        Action::UpdateProfile {
            owner_name: "  小禾  ".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.snapshot(0).owner_name, "小禾");
    assert!(matches!(
        game.dispatch(
            Action::UpdateProfile {
                owner_name: "   ".into(),
            },
            0,
        ),
        Err(GameError::BadPlayerName)
    ));
    assert!(matches!(
        game.dispatch(
            Action::UpdateProfile {
                owner_name: "一二三四五六七八九十一二三".into(),
            },
            0,
        ),
        Err(GameError::BadPlayerName)
    ));
}
