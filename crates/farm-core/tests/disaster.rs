use farm_core::{Action, ContentPack, CropInstance, Disaster, DisasterKind, Game, GameError};

const MIN: i64 = 60_000;

fn crop_with(disaster: Disaster, remaining: u32, ready_minutes: i64) -> Game {
    let mut state = Game::new(0).state().clone();
    state.plots[0].crop = Some(CropInstance {
        crop_id: "yumi".into(),
        planted_at: 0,
        ready_at: ready_minutes * MIN,
        season: 1,
        seasons: 1,
        remaining,
        watered: true,
        pest: None,
        disaster: Some(disaster),
        off_season: false,
        growth_factor: 1000,
        greenhouse: false,
        withered: false,
    });
    Game::from_parts(state, ContentPack::starter())
}

#[test]
fn flood_penalty_is_applied_at_harvest() {
    let mut game = crop_with(
        Disaster {
            kind: DisasterKind::Flood,
            since: 0,
            ends_at: 10 * 60 * MIN,
        },
        10,
        0,
    );
    let outcome = game.dispatch(Action::Harvest { plot: 1 }, 0).unwrap();
    let snapshot = game.snapshot(0);
    let qty = snapshot
        .inventory
        .iter()
        .find(|item| item.key == "crop:yumi")
        .map(|item| item.qty)
        .unwrap();
    assert_eq!(qty, 8);
    assert!(outcome.effects.iter().any(
        |effect| matches!(effect, farm_core::Effect::Log { text } if text.contains("灾害减产20%"))
    ));
}

#[test]
fn mitigating_drought_extends_ready_time_and_clears_it() {
    let mut game = crop_with(
        Disaster {
            kind: DisasterKind::Drought,
            since: 0,
            ends_at: 10 * 60 * MIN,
        },
        10,
        600,
    );
    game.dispatch(Action::MitigateDisaster { plot: 1 }, 2 * 60 * MIN)
        .unwrap();
    let snapshot = game.snapshot(2 * 60 * MIN);
    let crop = snapshot.plots[0].crop.as_ref().unwrap();
    assert!(crop.disaster.is_none());
    assert_eq!(crop.ready_at, 12 * 60 * MIN);
    assert_eq!(game.state().stats.disasters_mitigated, 1);
}

#[test]
fn hail_cannot_be_mitigated() {
    let mut game = crop_with(
        Disaster {
            kind: DisasterKind::Hail,
            since: 0,
            ends_at: 30 * MIN,
        },
        10,
        600,
    );
    assert!(matches!(
        game.dispatch(Action::MitigateDisaster { plot: 1 }, 0),
        Err(GameError::HailNoAction)
    ));
}

#[test]
fn frost_resistance_reduces_penalty() {
    let mut state = Game::new(0).state().clone();
    state.plots[0].crop = Some(CropInstance {
        crop_id: "yumi".into(),
        planted_at: 0,
        ready_at: 0,
        season: 1,
        seasons: 1,
        remaining: 10,
        watered: true,
        pest: None,
        disaster: Some(Disaster {
            kind: DisasterKind::Frost,
            since: 0,
            ends_at: 10 * 60 * MIN,
        }),
        off_season: false,
        growth_factor: 1000,
        greenhouse: false,
        withered: false,
    });
    let mut content = ContentPack::starter();
    content
        .crops
        .iter_mut()
        .find(|crop| crop.id == "yumi")
        .unwrap()
        .resistances
        .cold = 50;
    let mut game = Game::from_parts(state, content);
    game.dispatch(Action::Harvest { plot: 1 }, 0).unwrap();
    let qty = game
        .snapshot(0)
        .inventory
        .iter()
        .find(|item| item.key == "crop:yumi")
        .map(|item| item.qty)
        .unwrap();
    // 基础 10，霜冻 15% 被 50% 抗性削到 7% -> 10×93% = 9。
    assert_eq!(qty, 9);
}

#[test]
fn expired_disaster_only_pauses_until_its_end() {
    let mut game = crop_with(
        Disaster {
            kind: DisasterKind::Flood,
            since: 0,
            ends_at: 2 * 60 * MIN,
        },
        10,
        600,
    );
    game.sync(3 * 60 * MIN, "2026-07-01");
    let crop = game.snapshot(3 * 60 * MIN).plots[0].crop.clone().unwrap();
    assert!(crop.disaster.is_none());
    assert_eq!(crop.ready_at, 12 * 60 * MIN);
}

#[test]
fn long_growing_crop_eventually_gets_a_disaster() {
    let mut state = Game::new(0).state().clone();
    for plot in &mut state.plots {
        plot.crop = Some(CropInstance {
            crop_id: "renshen".into(),
            planted_at: 0,
            ready_at: 180 * 24 * 60 * MIN,
            season: 1,
            seasons: 1,
            remaining: 10,
            watered: false,
            pest: None,
            disaster: None,
            off_season: false,
            growth_factor: 1000,
            greenhouse: false,
            withered: false,
        });
    }
    let mut game = Game::from_parts(state, ContentPack::starter());
    for hour in 0..(180 * 24) {
        game.sync(hour * 60 * MIN, "2026-07-01");
    }
    assert!(game.state().stats.disasters_spawned > 0);
}
