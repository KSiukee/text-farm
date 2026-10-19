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
