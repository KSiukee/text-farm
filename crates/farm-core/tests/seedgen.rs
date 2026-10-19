use farm_core::{generate_seeds, review_candidates, ContentPack, ReviewStatus, CONTENT_SCHEMA};

#[test]
fn generated_seeds_are_valid_and_unique() {
    let seeds = generate_seeds(40, 20261018);
    assert_eq!(seeds.len(), 40);
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for crop in &seeds {
        assert!(ids.insert(crop.id.clone()), "重复 id: {}", crop.id);
        assert!(names.insert(crop.name.clone()), "重复名: {}", crop.name);
        assert!(!crop.sow_seasons.is_empty());
        assert!((1..=30).contains(&crop.unlock_level));
        assert!(crop.grow_minutes > 0);
        assert!(crop.yield_per_season > 0);
    }
    assert!(seeds
        .iter()
        .any(|crop| crop.region != farm_core::RegionAffinity::All));
}

#[test]
fn generated_seeds_pass_content_validation() {
    let mut pack = ContentPack::starter();
    let generated = generate_seeds(30, 42);
    pack.crops.extend(generated);
    assert_eq!(pack.schema, CONTENT_SCHEMA);
    assert!(pack.validate().is_empty(), "{:?}", pack.validate());
}

#[test]
fn review_rejects_bad_names_and_clamps_extreme_resistance() {
    let base = generate_seeds(2, 7);
    let mut bad_name = base[0].clone();
    bad_name.name = "坏名字的".into();
    let mut cheaty = base[1].clone();
    cheaty.resistances.drought = 99;
    cheaty.resistances.flood = 99;
    cheaty.resistances.cold = 99;
    cheaty.resistances.pest = 99;

    let reviews = review_candidates(vec![bad_name, cheaty]);
    assert_eq!(reviews[0].status, ReviewStatus::Rejected);
    assert_eq!(reviews[1].status, ReviewStatus::Revised);
    assert!(reviews[1].crop.resistances.drought <= 50);
    assert!(reviews[1].metrics.resistance_total <= 80);
}

#[test]
fn review_rejects_broken_economy() {
    let mut crop = generate_seeds(1, 11).remove(0);
    crop.sell_price = 100_000;
    let reviews = review_candidates(vec![crop]);
    assert_eq!(reviews[0].status, ReviewStatus::Rejected);
    assert!(reviews[0]
        .reasons
        .iter()
        .any(|reason| reason.contains("时收益")));
}
