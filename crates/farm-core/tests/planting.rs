use farm_core::{Action, ContentPack, DisciplineKind, Game};

fn game_with(level: u32, crop_id: &str, seeds: u32) -> Game {
    let mut state = Game::new(0).state().clone();
    state.level = level;
    state.coins = 1_000_000;
    state.inventory.clear();
    if seeds > 0 {
        state.inventory.insert(format!("seed:{crop_id}"), seeds);
    }
    Game::from_parts(state, ContentPack::starter())
}

fn plant(game: &mut Game, crop_id: &str, now: i64) -> i64 {
    game.dispatch(
        Action::Plant {
            plot: 1,
            crop_id: crop_id.into(),
        },
        now,
    )
    .unwrap();
    game.state()
        .plots
        .iter()
        .find(|plot| plot.index == 1)
        .and_then(|plot| plot.crop.as_ref())
        .unwrap()
        .ready_at
}

fn harvest(game: &mut Game, now: i64) {
    game.dispatch(Action::Harvest { plot: 1 }, now).unwrap();
}

fn crop_qty(game: &Game, crop_id: &str) -> u32 {
    *game
        .state()
        .inventory
        .get(&format!("crop:{crop_id}"))
        .unwrap_or(&0)
}

fn seed_qty(game: &Game, crop_id: &str) -> u32 {
    *game
        .state()
        .inventory
        .get(&format!("seed:{crop_id}"))
        .unwrap_or(&0)
}

#[test]
fn harvest_grants_planting_exp() {
    let mut game = game_with(30, "luobo", 1);
    let ready = plant(&mut game, "luobo", 0);
    harvest(&mut game, ready);

    let qty = crop_qty(&game, "luobo");
    let def = ContentPack::starter().crop("luobo").unwrap().clone();
    let expected = farm_core::harvest_exp(&def, qty, 1);
    assert_eq!(game.state().skills.planting.exp, expected);
    assert_eq!(game.state().skills.planting.level, 1);
}

#[test]
fn yield_perk_adds_extra_crop() {
    let mut base = game_with(30, "xiangrikui", 1);
    let ready = plant(&mut base, "xiangrikui", 0);
    harvest(&mut base, ready);
    let base_qty = crop_qty(&base, "xiangrikui");

    let mut perked = game_with(30, "xiangrikui", 1);
    perked
        .state_mut()
        .skills
        .planting
        .perks
        .insert("plant_yield".into(), 4);
    let ready = plant(&mut perked, "xiangrikui", 0);
    harvest(&mut perked, ready);
    let perked_qty = crop_qty(&perked, "xiangrikui");

    assert!(perked_qty > base_qty, "{perked_qty} vs {base_qty}");
    assert_eq!(perked_qty, base_qty + farm_core::yield_bonus(base_qty, 4));
}

#[test]
fn growth_perk_shortens_ready_time() {
    let mut base = game_with(30, "luobo", 1);
    let base_ready = plant(&mut base, "luobo", 0);

    let mut perked = game_with(30, "luobo", 1);
    perked
        .state_mut()
        .skills
        .planting
        .perks
        .insert("plant_growth".into(), 4);
    let perked_ready = plant(&mut perked, "luobo", 0);

    assert_eq!(base_ready, 10 * 60_000);
    assert_eq!(perked_ready, 10 * 60_000 * 88 / 100);
}

#[test]
fn water_perk_shortens_more() {
    let mut base = game_with(30, "luobo", 1);
    let base_ready = plant(&mut base, "luobo", 0);
    base.dispatch(Action::Water { plot: 1 }, 0).unwrap();
    let base_after = plant_ready(&base);

    let mut perked = game_with(30, "luobo", 1);
    perked
        .state_mut()
        .skills
        .planting
        .perks
        .insert("plant_water".into(), 4);
    let perked_ready = plant(&mut perked, "luobo", 0);
    perked.dispatch(Action::Water { plot: 1 }, 0).unwrap();
    let perked_after = plant_ready(&perked);

    assert_eq!(base_ready, 10 * 60_000);
    assert_eq!(perked_ready, 10 * 60_000);
    assert_eq!(base_after, base_ready * 90 / 100);
    assert_eq!(perked_after, perked_ready * 82 / 100);
}

fn plant_ready(game: &Game) -> i64 {
    game.state()
        .plots
        .iter()
        .find(|plot| plot.index == 1)
        .and_then(|plot| plot.crop.as_ref())
        .unwrap()
        .ready_at
}

#[test]
fn seed_perk_returns_seed_over_time() {
    let mut game = game_with(30, "xiaobaicai", 100);
    game.state_mut()
        .skills
        .planting
        .perks
        .insert("plant_seed".into(), 4);

    let mut returned = 0;
    for i in 0..30i64 {
        let now = i * 1_000_000;
        let before = seed_qty(&game, "xiaobaicai");
        let ready = plant(&mut game, "xiaobaicai", now);
        harvest(&mut game, ready);
        if seed_qty(&game, "xiaobaicai") == before {
            returned += 1;
        }
    }
    assert!(returned > 0, "40% 留种，30 次一颗都没返");
}

#[test]
fn harvest_exp_soft_caps_after_forty() {
    let def = ContentPack::starter().crop("luobo").unwrap().clone();
    assert_eq!(farm_core::harvest_exp(&def, 4, 40), 4);
    assert_eq!(farm_core::harvest_exp(&def, 4, 41), 2);
}

#[test]
fn player_view_lists_planting_skills() {
    let game = Game::new(0);
    let snapshot = game.snapshot(0);
    let planting = snapshot
        .skills
        .disciplines
        .iter()
        .find(|d| d.kind == DisciplineKind::Planting)
        .unwrap();
    assert_eq!(planting.skills.len(), 4);
    for name in ["精耕", "时令", "保墒", "留种"] {
        assert!(planting.skills.iter().any(|s| s.name == name), "{name}");
    }
}
