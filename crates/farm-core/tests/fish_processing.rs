use farm_core::{Action, BaitQuality, FishRecipe, Game, GameError, WorkshopJob};

fn workshop_game() -> Game {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game
}

fn finish_job(game: &mut Game, slot: u8) {
    if let Some(Some(job)) = game.state_mut().workshop.job_mut(slot) {
        match job {
            WorkshopJob::Ranch { ready_at, .. }
            | WorkshopJob::CropBox { ready_at, .. }
            | WorkshopJob::Equipment { ready_at, .. }
            | WorkshopJob::Bait { ready_at, .. }
            | WorkshopJob::Fish { ready_at, .. } => *ready_at = 0,
        }
    }
}

fn collect(game: &mut Game, slot: u8) {
    finish_job(game, slot);
    game.dispatch(Action::CollectCraft { slot }, 0).unwrap();
}

#[test]
fn jerky_consumes_cheapest_unlocked_fish() {
    let mut game = workshop_game();
    game.state_mut()
        .inventory
        .insert("fish:white_bait".into(), 1);
    game.state_mut().inventory.insert("fish:crucian".into(), 1);
    game.state_mut().inventory.insert("fish:carp".into(), 1);
    game.state_mut().fishing.locked_fish.insert("carp".into());

    game.dispatch(
        Action::StartFishCraft {
            recipe: FishRecipe::Jerky,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().inventory.get("fish:white_bait"), None);
    assert_eq!(game.state().inventory.get("fish:crucian"), None);
    assert_eq!(game.state().inventory.get("fish:carp"), Some(&1));

    collect(&mut game, 0);
    assert_eq!(game.state().inventory.get("craft:fish_jerky"), Some(&1));
    assert_eq!(game.state().skills.processing.exp, 18);
}

#[test]
fn fish_soup_is_a_usable_buff_food() {
    let mut game = workshop_game();
    game.state_mut().inventory.insert("fish:crucian".into(), 1);
    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 2);
    game.dispatch(
        Action::StartFishCraft {
            recipe: FishRecipe::Soup,
        },
        0,
    )
    .unwrap();
    collect(&mut game, 0);
    assert_eq!(game.state().inventory.get("craft:fish_soup"), Some(&1));

    game.dispatch(
        Action::UseBuffItem {
            key: "craft:fish_soup".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(game.state().buffs.len(), 1);
    assert_eq!(game.state().buffs[0].id, "buff_fish_soup");
    assert_eq!(game.state().buffs[0].stat, farm_core::BuffStat::ExpFishing);
}

#[test]
fn fish_processing_requires_fish_and_side_inputs() {
    let mut game = workshop_game();
    assert!(matches!(
        game.dispatch(
            Action::StartFishCraft {
                recipe: FishRecipe::Jerky
            },
            0
        ),
        Err(GameError::NotEnoughItems)
    ));

    game.state_mut().inventory.insert("fish:crucian".into(), 1);
    assert!(matches!(
        game.dispatch(
            Action::StartFishCraft {
                recipe: FishRecipe::Soup
            },
            0
        ),
        Err(GameError::RanchIngredientsShort)
    ));
}

#[test]
fn locked_fish_are_not_consumed() {
    let mut game = workshop_game();
    game.state_mut().inventory.insert("fish:carp".into(), 4);
    game.state_mut().fishing.locked_fish.insert("carp".into());
    assert!(matches!(
        game.dispatch(
            Action::StartFishCraft {
                recipe: FishRecipe::Oil
            },
            0
        ),
        Err(GameError::NotEnoughItems)
    ));
    assert_eq!(game.state().inventory.get("fish:carp"), Some(&4));
}

#[test]
fn fish_oil_output_is_sellable_craft_item() {
    let mut game = workshop_game();
    game.state_mut()
        .inventory
        .insert("fish:white_bait".into(), 4);
    game.dispatch(
        Action::StartFishCraft {
            recipe: FishRecipe::Oil,
        },
        0,
    )
    .unwrap();
    collect(&mut game, 0);
    assert_eq!(game.state().inventory.get("craft:fish_oil"), Some(&1));
    let item = game
        .snapshot(0)
        .inventory
        .iter()
        .find(|item| item.key == "craft:fish_oil")
        .cloned()
        .unwrap();
    assert!(item.sellable);
    assert_eq!(item.unit_price, 120);
}

#[test]
fn fish_processing_view_lists_four_recipes() {
    let mut game = workshop_game();
    game.state_mut()
        .inventory
        .insert("fish:white_bait".into(), 4);
    let shop = &game.snapshot(0).workshop;
    assert_eq!(shop.fish_recipes.len(), 4);
    let jerky = shop
        .fish_recipes
        .iter()
        .find(|recipe| recipe.recipe == FishRecipe::Jerky)
        .unwrap();
    assert!(jerky.can_start);
    assert!(jerky.input_text.contains("任意鱼×2"));
}

#[test]
fn fish_recipe_uses_normal_quality_bait_type_independent() {
    // 鱼加工只吃鱼，不消耗鱼饵；顺手确认初始鱼饵没被动过。
    let mut game = workshop_game();
    game.state_mut()
        .inventory
        .insert("fish:white_bait".into(), 2);
    game.dispatch(
        Action::StartFishCraft {
            recipe: FishRecipe::Jerky,
        },
        0,
    )
    .unwrap();
    assert_eq!(
        game.state()
            .fishing
            .bait_count_of(farm_core::BaitType::Dough, BaitQuality::Normal),
        10
    );
}
