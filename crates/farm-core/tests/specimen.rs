use farm_core::{specimen_recipe, Action, ContentPack, Game, GameError};

fn game_with(crop_id: &str, qty: u32, coins: u64) -> Game {
    let mut state = Game::new(0).state().clone();
    state.level = 30;
    state.coins = coins;
    state.inventory.clear();
    state.inventory.insert(format!("crop:{crop_id}"), qty);
    Game::from_parts(state, ContentPack::starter())
}

#[test]
fn recipe_cost_scales_with_crop_level() {
    let content = ContentPack::starter();
    let low = specimen_recipe(content.crop("xiaobaicai").unwrap());
    let high = specimen_recipe(content.crop("renshen").unwrap());
    assert!(high.need_qty > low.need_qty);
    assert!(high.coins > low.coins);
}

#[test]
fn crafting_consumes_crop_and_coins_and_drops_a_rare_seed() {
    let mut game = game_with("xiaobaicai", 20, 10_000);
    let before = game.state().coins;
    game.dispatch(
        Action::CraftSpecimen {
            crop_id: "xiaobaicai".into(),
        },
        0,
    )
    .unwrap();
    let snapshot = game.snapshot(0);
    let crop_left = snapshot
        .inventory
        .iter()
        .find(|item| item.key == "crop:xiaobaicai")
        .map(|item| item.qty)
        .unwrap_or(0);
    assert!(crop_left < 20);
    assert!(game.state().coins > before - 2_000);
    assert!(snapshot
        .inventory
        .iter()
        .any(|item| item.key.starts_with("seed:") && item.name.contains("种子")));
    assert_eq!(game.state().stats.specimens_crafted, 1);
    assert!(game.state().stats.rare_seeds_gained >= 1);
}

#[test]
fn crafting_returns_a_result_and_marks_collection() {
    let mut game = game_with("xiaobaicai", 20, 10_000);
    let outcome = game
        .dispatch(
            Action::CraftSpecimen {
                crop_id: "xiaobaicai".into(),
            },
            0,
        )
        .unwrap();
    let result = outcome.effects.iter().find_map(|effect| match effect {
        farm_core::Effect::Specimen {
            crop_name,
            seed_qty,
            ..
        } => Some((crop_name.clone(), *seed_qty)),
        _ => None,
    });
    let (crop_name, seed_qty) = result.expect("specimen result effect");
    assert_eq!(crop_name, "小青菜");
    assert!(seed_qty >= 1);
    assert_eq!(
        game.state().specimen_collection.get("xiaobaicai"),
        Some(&1)
    );
}

#[test]
fn featured_specimen_pays_fifty_percent_more() {
    let mut game = game_with("xiaobaicai", 0, 100_000);
    game.sync(0, "2026-10-24");
    let featured_id = game.snapshot(0).specimen_featured.expect("featured crop");
    let recipe = specimen_recipe(game.content().crop(&featured_id).unwrap());
    game.state_mut()
        .inventory
        .insert(format!("crop:{featured_id}"), recipe.need_qty);
    let before = game.state().coins;
    let outcome = game
        .dispatch(
            Action::CraftSpecimen {
                crop_id: featured_id,
            },
            1_000,
        )
        .unwrap();
    let result = outcome.effects.iter().find_map(|effect| match effect {
        farm_core::Effect::Specimen {
            coins, featured, ..
        } => Some((*coins, *featured)),
        _ => None,
    });
    let (coins, featured) = result.expect("featured result");
    assert!(featured);
    assert_eq!(coins, recipe.coins / 2 + recipe.coins / 4);
    assert!(game.state().coins > before);
}

#[test]
fn specimen_milestone_requires_unique_crops_and_only_pays_once() {
    let mut game = Game::new(0);
    game.state_mut().coins = 100_000;
    assert!(matches!(
        game.dispatch(Action::ClaimSpecimenMilestone { unique: 3 }, 0),
        Err(GameError::SpecimenMilestoneLocked)
    ));
    let ids: Vec<String> = game
        .content()
        .crops
        .iter()
        .take(3)
        .map(|crop| crop.id.clone())
        .collect();
    for id in ids {
        game.state_mut().specimen_collection.insert(id, 1);
    }
    let before = game.state().coins;
    game.dispatch(Action::ClaimSpecimenMilestone { unique: 3 }, 0)
        .unwrap();
    assert_eq!(game.state().coins, before + 3_000);
    assert!(matches!(
        game.dispatch(Action::ClaimSpecimenMilestone { unique: 3 }, 0),
        Err(GameError::AlreadyClaimed)
    ));
}

#[test]
fn crafting_requires_materials_and_coins() {
    let mut game = game_with("xiaobaicai", 1, 10_000);
    assert!(matches!(
        game.dispatch(
            Action::CraftSpecimen {
                crop_id: "xiaobaicai".into()
            },
            0
        ),
        Err(GameError::NotEnoughItems)
    ));

    let mut game = game_with("xiaobaicai", 20, 0);
    assert!(matches!(
        game.dispatch(
            Action::CraftSpecimen {
                crop_id: "xiaobaicai".into()
            },
            0
        ),
        Err(GameError::NotEnoughCoins { .. })
    ));
}
