//! 家具/地板/墙纸的金币商店。
//!
//! 家具不接制造屋：普通家具金币直购；地板/墙纸每天随机上架 2+2 件，
//! 买到的样式永久拥有。旧档按当前已摆放的家具/样式回填拥有数。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const DECOR_STOCK_PER_KIND: usize = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecorKind {
    Floor,
    Wall,
}

impl DecorKind {
    pub fn label(self) -> &'static str {
        match self {
            DecorKind::Floor => "地板",
            DecorKind::Wall => "墙纸",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecorOffer {
    pub id: String,
    pub kind: DecorKind,
    pub item_id: String,
    pub name: String,
    pub price: u64,
    #[serde(default)]
    pub sold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct DecorShopState {
    pub day: String,
    pub offers: Vec<DecorOffer>,
}

impl DecorShopState {
    pub fn ensure_day(
        &mut self,
        day: &str,
        owned_floors: &BTreeSet<String>,
        owned_walls: &BTreeSet<String>,
    ) -> bool {
        if self.day == day {
            return false;
        }
        self.day = day.to_string();
        self.offers = generate(day, owned_floors, owned_walls);
        true
    }

    pub fn offer(&self, id: &str) -> Option<&DecorOffer> {
        self.offers.iter().find(|offer| offer.id == id)
    }

    pub fn mark_sold(&mut self, id: &str) -> bool {
        if let Some(offer) = self.offers.iter_mut().find(|offer| offer.id == id) {
            if offer.sold {
                return false;
            }
            offer.sold = true;
            return true;
        }
        false
    }
}

pub fn furniture_price(id: &str) -> u64 {
    match id {
        "bed" => 6_000,
        "desk" => 2_800,
        "chair" => 800,
        "rug" => 1_800,
        "cabinet" => 2_400,
        "plant" => 600,
        "flower_vase" => 500,
        "planter" => 900,
        "aquarium_small" => 8_000,
        "aquarium_large" => 18_000,
        "sofa" => 5_200,
        "bookshelf" => 2_600,
        "floor_lamp" => 1_600,
        "nightstand" => 1_200,
        "tv" => 4_800,
        "beanbag" => 1_500,
        _ => 0,
    }
}

pub fn furniture_source(id: &str) -> &'static str {
    if id.starts_with("gift_") {
        "gift"
    } else if id.starts_with("mystery_") {
        "mystery"
    } else {
        "shop"
    }
}

pub fn floor_price(id: &str) -> u64 {
    match id {
        "wood" | "tile" => 0,
        "stone" => 3_200,
        "terracotta" => 4_500,
        "tatami" => 5_000,
        "cafe" => 6_500,
        _ => 3_000,
    }
}

pub fn wall_price(id: &str) -> u64 {
    match id {
        "cream" | "sage" => 0,
        "sky" => 2_200,
        "dusk" => 3_800,
        "linen" => 4_200,
        _ => 2_000,
    }
}

pub fn free_floors() -> BTreeSet<String> {
    ["wood", "tile"].into_iter().map(String::from).collect()
}

pub fn free_walls() -> BTreeSet<String> {
    ["cream", "sage"].into_iter().map(String::from).collect()
}

fn generate(
    day: &str,
    owned_floors: &BTreeSet<String>,
    owned_walls: &BTreeSet<String>,
) -> Vec<DecorOffer> {
    let mut offers = Vec::new();
    let floors: Vec<(&str, &str)> = crate::room::floor_styles()
        .into_iter()
        .filter(|(id, _)| !owned_floors.contains(*id) && floor_price(id) > 0)
        .collect();
    let walls: Vec<(&str, &str)> = crate::room::wall_styles()
        .into_iter()
        .filter(|(id, _)| !owned_walls.contains(*id) && wall_price(id) > 0)
        .collect();
    push_offers(day, DecorKind::Floor, &floors, &mut offers);
    push_offers(day, DecorKind::Wall, &walls, &mut offers);
    offers
}

fn push_offers(
    day: &str,
    kind: DecorKind,
    pool: &[(&str, &str)],
    offers: &mut Vec<DecorOffer>,
) {
    if pool.is_empty() {
        return;
    }
    let start = (roll(day, kind as u64 + 1) as usize) % pool.len();
    let take = DECOR_STOCK_PER_KIND.min(pool.len());
    for offset in 0..take {
        let (id, name) = pool[(start + offset) % pool.len()];
        let price = match kind {
            DecorKind::Floor => floor_price(id),
            DecorKind::Wall => wall_price(id),
        };
        let kind_key = match kind {
            DecorKind::Floor => "floor",
            DecorKind::Wall => "wall",
        };
        offers.push(DecorOffer {
            id: format!("{day}-{kind_key}-{offset}"),
            kind,
            item_id: id.into(),
            name: name.into(),
            price,
            sold: false,
        });
    }
}

fn roll(day: &str, salt: u64) -> u64 {
    let mut hash = 0xcbf29ce484222325u64 ^ salt;
    for byte in day.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// 从当前房间回填拥有数：旧档不会因为改成商店而丢掉已摆的家具/样式。
pub fn seed_ownership(
    rooms: &BTreeMap<String, crate::room::RoomLayout>,
) -> (BTreeMap<String, u32>, BTreeSet<String>, BTreeSet<String>) {
    let mut furniture = BTreeMap::new();
    let mut floors = free_floors();
    let mut walls = free_walls();
    for layout in rooms.values() {
        floors.insert(layout.floor.clone());
        walls.insert(layout.wall.clone());
        for item in &layout.furniture {
            *furniture.entry(item.def.clone()).or_insert(0) += 1;
        }
    }
    (furniture, floors, walls)
}

pub fn layout_furniture(layout: &crate::room::RoomLayout) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for item in &layout.furniture {
        *counts.entry(item.def.clone()).or_insert(0) += 1;
    }
    counts
}
