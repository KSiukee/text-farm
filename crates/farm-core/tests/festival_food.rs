use farm_core::{Action, EquipmentKind, EquipmentRecipe, Game, GameError, WorkshopJob};

fn workshop_game() -> Game {
    let mut game = Game::new(0);
    game.state_mut().level = 20;
    game.state_mut().coins = 100_000;
    game.dispatch(Action::ShowGuide, 0).unwrap();
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game.dispatch(
        Action::InstallEquipment {
            slot: 1,
            kind: EquipmentKind::BoilingPot,
        },
        0,
    )
    .unwrap();
    game
}

#[test]
fn festival_food_is_wired_to_recipe_item_and_buff() {
    assert_eq!(EquipmentRecipe::ALL.len(), 24);
    assert_eq!(EquipmentRecipe::Dumpling.festival(), Some("春节"));
    assert_eq!(EquipmentRecipe::Dumpling.output_key(), "craft:dumpling");
    assert_eq!(
        farm_core::craft_item("craft:dumpling")
            .map(|item| item.name)
            .unwrap(),
        "饺子"
    );
    assert_eq!(
        farm_core::buff_item("craft:dumpling")
            .map(|item| item.name)
            .unwrap(),
        "饺子"
    );
}

#[test]
fn festival_food_only_starts_on_its_festival() {
    let mut game = workshop_game();
    game.state_mut().daily.day = "2026-02-17".into();
    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 2);
    game.state_mut().inventory.insert("ranch:egg".into(), 2);
    game.state_mut().inventory.insert("crop:yumi".into(), 1);

    game.dispatch(
        Action::StartEquipmentCraft {
            recipe: EquipmentRecipe::Dumpling,
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        game.state().workshop.job(1),
        Some(WorkshopJob::Equipment {
            recipe: EquipmentRecipe::Dumpling,
            ..
        })
    ));

    assert!(matches!(
        game.dispatch(
            Action::StartEquipmentCraft {
                recipe: EquipmentRecipe::Tangyuan,
            },
            0
        ),
        Err(GameError::FestivalRecipeUnavailable)
    ));
}
