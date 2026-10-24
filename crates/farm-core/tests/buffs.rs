use farm_core::{
    Action, ContentPack, DisciplineKind, Game, GameError, RanchAnimalKind, RanchRecipe,
};

const JUICE_PLANT_EXP: &str = "craft:strawberry_juice";
const JUICE_PLANT_YIELD: &str = "craft:grape_juice";
const JUICE_CRAFT_SPEED: &str = "craft:cane_juice";
const CHEESE: &str = "ranch:cheese";

fn give(game: &mut Game, key: &str, qty: u32) {
    game.state_mut().inventory.insert(key.into(), qty);
}

fn use_item(game: &mut Game, key: &str, now: i64) {
    game.dispatch(Action::UseBuffItem { key: key.into() }, now)
        .unwrap();
}

fn planting_game(crop_id: &str) -> (Game, i64) {
    let mut game = Game::new(0);
    game.state_mut().level = 30;
    game.state_mut().coins = 10_000;
    game.state_mut().inventory.clear();
    game.state_mut()
        .inventory
        .insert(format!("seed:{crop_id}"), 1);
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: crop_id.into(),
        },
        0,
    )
    .unwrap();
    let ready = game.state().plots[0].crop.as_ref().unwrap().ready_at;
    (game, ready)
}

fn crop_qty(game: &Game, crop_id: &str) -> u32 {
    *game
        .state()
        .inventory
        .get(&format!("crop:{crop_id}"))
        .unwrap_or(&0)
}

#[test]
fn use_item_consumes_and_activates() {
    let mut game = Game::new(0);
    give(&mut game, JUICE_PLANT_EXP, 1);
    use_item(&mut game, JUICE_PLANT_EXP, 0);
    assert_eq!(game.state().inventory.get(JUICE_PLANT_EXP), None);
    assert_eq!(game.state().buffs.len(), 1);
    assert_eq!(game.state().buffs[0].expires_at, 30 * 60_000);
    assert_eq!(
        farm_core::buff_exp_pct(&game.state().buffs, 0, DisciplineKind::Planting),
        20
    );
}

#[test]
fn same_item_refreshes_instead_of_stacking() {
    let mut game = Game::new(0);
    give(&mut game, JUICE_PLANT_EXP, 2);
    use_item(&mut game, JUICE_PLANT_EXP, 0);
    use_item(&mut game, JUICE_PLANT_EXP, 10 * 60_000);
    assert_eq!(game.state().buffs.len(), 1);
    assert_eq!(game.state().buffs[0].expires_at, 40 * 60_000);
}

#[test]
fn bad_or_missing_item_is_rejected() {
    let mut game = Game::new(0);
    assert!(matches!(
        game.dispatch(
            Action::UseBuffItem {
                key: "crop:luobo".into()
            },
            0
        ),
        Err(GameError::BadBuffItem)
    ));
    assert!(matches!(
        game.dispatch(
            Action::UseBuffItem {
                key: JUICE_PLANT_EXP.into()
            },
            0
        ),
        Err(GameError::NotEnoughItems)
    ));
}

#[test]
fn exp_buff_applies_to_planting() {
    let (mut game, ready) = planting_game("xiangrikui");
    give(&mut game, JUICE_PLANT_EXP, 1);
    use_item(&mut game, JUICE_PLANT_EXP, ready);
    game.dispatch(Action::Harvest { plot: 1 }, ready).unwrap();

    let qty = crop_qty(&game, "xiangrikui");
    let def = ContentPack::starter().crop("xiangrikui").unwrap().clone();
    let base = farm_core::harvest_exp(&def, qty, 1);
    assert_eq!(
        game.state().skills.planting.exp,
        base * 120 / 100,
        "qty={qty} base={base}"
    );
}

#[test]
fn exp_buff_does_not_apply_after_expiry() {
    let (mut game, ready) = planting_game("xiangrikui");
    give(&mut game, JUICE_PLANT_EXP, 1);
    use_item(&mut game, JUICE_PLANT_EXP, 0);
    let late = ready + 1;
    assert!(late > ready);
    game.dispatch(Action::Harvest { plot: 1 }, late).unwrap();

    let qty = crop_qty(&game, "xiangrikui");
    let def = ContentPack::starter().crop("xiangrikui").unwrap().clone();
    assert_eq!(
        game.state().skills.planting.exp,
        farm_core::harvest_exp(&def, qty, 1)
    );
    game.sync(late, "2026-10-20");
    assert!(game.state().buffs.is_empty());
}

#[test]
fn yield_buff_adds_extra_crop() {
    let (mut base, base_ready) = planting_game("xiangrikui");
    base.dispatch(Action::Harvest { plot: 1 }, base_ready)
        .unwrap();
    let base_qty = crop_qty(&base, "xiangrikui");

    let (mut buffed, ready) = planting_game("xiangrikui");
    give(&mut buffed, JUICE_PLANT_YIELD, 1);
    use_item(&mut buffed, JUICE_PLANT_YIELD, ready);
    buffed.dispatch(Action::Harvest { plot: 1 }, ready).unwrap();
    let buffed_qty = crop_qty(&buffed, "xiangrikui");

    assert_eq!(buffed_qty, base_qty + base_qty * 10 / 100);
}

#[test]
fn craft_speed_buff_shortens_craft() {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    give(&mut game, "ranch:milk", 2);
    give(&mut game, JUICE_CRAFT_SPEED, 1);
    use_item(&mut game, JUICE_CRAFT_SPEED, 0);
    game.dispatch(
        Action::StartRanchCraft {
            recipe: RanchRecipe::Cheese,
        },
        0,
    )
    .unwrap();
    let ready = game.state().workshop.job(0).unwrap().ready_at();
    assert_eq!(ready, 4 * 60 * 60 * 1000 * 90 / 100);
}

#[test]
fn ranch_quality_buff_adds_to_perk_chance() {
    let mut game = Game::new(0);
    game.state_mut().coins = 10_000;
    game.dispatch(
        Action::BuyRanchAnimal {
            pen: 1,
            kind: RanchAnimalKind::Chicken,
        },
        0,
    )
    .unwrap();
    game.state_mut()
        .skills
        .ranching
        .perks
        .insert("ranch_quality".into(), 4);
    give(&mut game, CHEESE, 1);
    use_item(&mut game, CHEESE, 0);

    for i in 0..40i64 {
        let animal = game
            .state_mut()
            .ranch
            .pen_mut(1)
            .unwrap()
            .animal
            .as_mut()
            .unwrap();
        animal.product_ready_at = 0;
        game.dispatch(Action::CollectRanchProduct { pen: 1 }, i * 1_000)
            .unwrap();
    }
    let better: u32 = game
        .state()
        .inventory
        .iter()
        .filter(|(key, _)| key.starts_with("ranch:egg."))
        .map(|(_, qty)| *qty)
        .sum();
    assert!(better > 0, "40% 专精 + 10% 食物，40 次没升过品质");
}

#[test]
fn quality_variant_cheese_is_usable() {
    let mut game = Game::new(0);
    give(&mut game, "ranch:cheese.fine", 1);
    use_item(&mut game, "ranch:cheese.fine", 0);
    assert_eq!(game.state().inventory.get("ranch:cheese.fine"), None);
    assert_eq!(game.state().buffs.len(), 1);
    assert_eq!(game.state().buffs[0].id, "buff_ranch_quality");
}

#[test]
fn player_view_shows_buffs_and_food_menu() {
    let mut game = Game::new(0);
    give(&mut game, JUICE_PLANT_EXP, 1);
    use_item(&mut game, JUICE_PLANT_EXP, 0);
    let snapshot = game.snapshot(0);
    assert_eq!(snapshot.skills.buffs.len(), 1);
    assert_eq!(snapshot.skills.buff_items.len(), 13);
    assert!(snapshot
        .skills
        .buff_items
        .iter()
        .any(|item| item.key == JUICE_PLANT_EXP));
}
