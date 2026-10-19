use farm_core::{Action, DisciplineKind, Game, RanchRecipe, WorkshopJob};

fn workshop_game() -> Game {
    let mut game = Game::new(0);
    game.state_mut().level = 8;
    game.state_mut().coins = 50_000;
    game.dispatch(Action::BuildWorkshop, 0).unwrap();
    game
}

fn add_milk(game: &mut Game, qty: u32) {
    *game
        .state_mut()
        .inventory
        .entry("ranch:milk".into())
        .or_insert(0) += qty;
}

fn start_cheese(game: &mut Game, now: i64) {
    game.dispatch(
        Action::StartRanchCraft {
            recipe: RanchRecipe::Cheese,
        },
        now,
    )
    .unwrap();
}

fn finish_job(game: &mut Game) {
    if let Some(Some(job)) = game.state_mut().workshop.job_mut(0) {
        match job {
            WorkshopJob::Ranch { ready_at, .. }
            | WorkshopJob::CropBox { ready_at, .. }
            | WorkshopJob::Equipment { ready_at, .. } => *ready_at = 0,
            WorkshopJob::Bait { ready_at, .. } => *ready_at = 0,
            WorkshopJob::Fish { ready_at, .. } => *ready_at = 0,
        }
    }
}

fn collect(game: &mut Game, now: i64) {
    game.dispatch(Action::CollectCraft { slot: 0 }, now)
        .unwrap();
}

fn cheese_total(game: &Game) -> u32 {
    game.state()
        .inventory
        .iter()
        .filter(|(key, _)| key.starts_with("ranch:cheese"))
        .map(|(_, qty)| *qty)
        .sum()
}

#[test]
fn collect_grants_processing_exp() {
    let mut game = workshop_game();
    add_milk(&mut game, 2);
    start_cheese(&mut game, 0);
    finish_job(&mut game);
    collect(&mut game, 0);
    assert_eq!(game.state().skills.processing.exp, 18);
    assert_eq!(cheese_total(&game), 1);
}

#[test]
fn quality_perk_can_raise_craft_quality() {
    let mut game = workshop_game();
    game.state_mut()
        .skills
        .processing
        .perks
        .insert("craft_quality".into(), 4);
    let mut better = 0;
    for i in 0..40i64 {
        add_milk(&mut game, 2);
        start_cheese(&mut game, i * 1_000);
        if game.state().workshop.job(0).unwrap().quality() > 1 {
            better += 1;
        }
        finish_job(&mut game);
        collect(&mut game, i * 1_000);
    }
    assert!(better > 0, "40% 精制，40 次没有一次升品质");
}

#[test]
fn speed_perk_shortens_craft_time() {
    let mut base = workshop_game();
    add_milk(&mut base, 2);
    start_cheese(&mut base, 0);
    let base_ready = base.state().workshop.job(0).unwrap().ready_at();

    let mut perked = workshop_game();
    perked
        .state_mut()
        .skills
        .processing
        .perks
        .insert("craft_speed".into(), 4);
    add_milk(&mut perked, 2);
    start_cheese(&mut perked, 0);
    let perked_ready = perked.state().workshop.job(0).unwrap().ready_at();

    assert_eq!(base_ready, 4 * 60 * 60 * 1000);
    assert_eq!(perked_ready, 4 * 60 * 60 * 1000 * 84 / 100);
}

#[test]
fn yield_perk_adds_extra_output() {
    let mut game = workshop_game();
    game.state_mut()
        .skills
        .processing
        .perks
        .insert("craft_yield".into(), 4);
    for i in 0..40i64 {
        add_milk(&mut game, 2);
        start_cheese(&mut game, i * 1_000);
        finish_job(&mut game);
        collect(&mut game, i * 1_000);
    }
    assert!(cheese_total(&game) > 40, "32% 出品，40 次一次都没多");
}

#[test]
fn save_perk_returns_ingredient() {
    let mut game = workshop_game();
    game.state_mut()
        .skills
        .processing
        .perks
        .insert("craft_save".into(), 4);
    add_milk(&mut game, 80);
    for i in 0..40i64 {
        start_cheese(&mut game, i * 1_000);
        finish_job(&mut game);
        collect(&mut game, i * 1_000);
    }
    let left = *game.state().inventory.get("ranch:milk").unwrap_or(&0);
    assert!(left > 0, "32% 省料，40 次一份都没退");
}

#[test]
fn player_view_lists_processing_skills() {
    let game = Game::new(0);
    let snapshot = game.snapshot(0);
    let processing = snapshot
        .skills
        .disciplines
        .iter()
        .find(|d| d.kind == DisciplineKind::Processing)
        .unwrap();
    assert_eq!(processing.skills.len(), 4);
    for name in ["出品", "精制", "熟练", "省料"] {
        assert!(processing.skills.iter().any(|s| s.name == name), "{name}");
    }
}
