use farm_core::{Action, DisciplineKind, Game, RanchAnimalKind};

fn chicken_game() -> Game {
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
    game
}

fn reset_product(game: &mut Game) {
    game.state_mut()
        .ranch
        .pen_mut(1)
        .unwrap()
        .animal
        .as_mut()
        .unwrap()
        .product_ready_at = 0;
}

fn collect(game: &mut Game, now: i64) {
    game.dispatch(Action::CollectRanchProduct { pen: 1 }, now)
        .unwrap();
}

fn egg_total(game: &Game) -> u32 {
    game.state()
        .inventory
        .iter()
        .filter(|(key, _)| key.starts_with("ranch:egg"))
        .map(|(_, qty)| *qty)
        .sum()
}

fn egg_better_than_base(game: &Game) -> u32 {
    game.state()
        .inventory
        .iter()
        .filter(|(key, _)| key.starts_with("ranch:egg."))
        .map(|(_, qty)| *qty)
        .sum()
}

#[test]
fn collect_grants_ranching_exp() {
    let mut game = chicken_game();
    reset_product(&mut game);
    collect(&mut game, 0);
    assert_eq!(game.state().skills.ranching.exp, 23);
    assert_eq!(game.state().skills.ranching.level, 1);
}

#[test]
fn quality_perk_can_raise_product_quality() {
    let mut game = chicken_game();
    game.state_mut()
        .skills
        .ranching
        .perks
        .insert("ranch_quality".into(), 4);
    let mut better = 0;
    for i in 0..40i64 {
        reset_product(&mut game);
        collect(&mut game, i * 1_000);
    }
    better += egg_better_than_base(&game);
    assert!(better > 0, "40% 优育，40 次没有一次升品质");
}

#[test]
fn yield_perk_adds_extra_product() {
    let mut game = chicken_game();
    game.state_mut()
        .skills
        .ranching
        .perks
        .insert("ranch_yield".into(), 4);
    for i in 0..40i64 {
        reset_product(&mut game);
        collect(&mut game, i * 1_000);
    }
    assert!(egg_total(&game) > 40, "32% 增产，40 次一颗都没多");
}

#[test]
fn feed_save_perk_sometimes_keeps_feed() {
    let mut game = chicken_game();
    game.state_mut()
        .skills
        .ranching
        .perks
        .insert("ranch_feed".into(), 4);
    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 60);
    for i in 0..40i64 {
        game.dispatch(Action::FeedRanchAnimal { pen: 1 }, i * 1_000)
            .unwrap();
    }
    let left = *game.state().inventory.get("crop:xiaobaicai").unwrap_or(&0);
    assert!(left > 0, "32% 省料，40 次一次都没省");
}

#[test]
fn care_perk_boosts_feed_and_heal() {
    let mut game = chicken_game();
    game.state_mut()
        .skills
        .ranching
        .perks
        .insert("ranch_care".into(), 4);
    {
        let animal = game
            .state_mut()
            .ranch
            .pen_mut(1)
            .unwrap()
            .animal
            .as_mut()
            .unwrap();
        animal.satiety = 0;
        animal.health = 50;
    }
    game.state_mut()
        .inventory
        .insert("crop:xiaobaicai".into(), 1);
    game.dispatch(Action::FeedRanchAnimal { pen: 1 }, 0)
        .unwrap();
    let animal = game.state().ranch.pen(1).unwrap().animal.as_ref().unwrap();
    assert_eq!(animal.satiety, 42); // 30 × 1.4
    assert_eq!(animal.health, 57); // 50 + 5 × 1.4

    {
        let animal = game
            .state_mut()
            .ranch
            .pen_mut(1)
            .unwrap()
            .animal
            .as_mut()
            .unwrap();
        animal.sick = true;
        animal.health = 50;
    }
    game.state_mut().coins = 1_000;
    game.dispatch(Action::HealRanchAnimal { pen: 1 }, 1_000)
        .unwrap();
    let animal = game.state().ranch.pen(1).unwrap().animal.as_ref().unwrap();
    assert_eq!(animal.health, 78); // 50 + 20 × 1.4
    assert!(!animal.sick);
}

#[test]
fn player_view_lists_ranching_skills() {
    let game = Game::new(0);
    let snapshot = game.snapshot(0);
    let ranching = snapshot
        .skills
        .disciplines
        .iter()
        .find(|d| d.kind == DisciplineKind::Ranching)
        .unwrap();
    assert_eq!(ranching.skills.len(), 4);
    for name in ["优育", "增产", "省料", "照料"] {
        assert!(ranching.skills.iter().any(|s| s.name == name), "{name}");
    }
}
