//! 神秘商店：每天 6 格，5 格稀有 + 1 格专栏；专栏 3% 出传说。
//! 第一期只卖“可越级种一次的稀有种子”和“神秘药剂”，家具另开。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::buffs::buff_item;
use crate::content::{ContentPack, CropDef};
use crate::workshop::craft_item;

pub const MYSTERY_OFFERS: usize = 10;
pub const MYSTERY_NORMAL_OFFERS: usize = 9;
pub const MYSTERY_LEGENDARY_PCT: u64 = 3;
pub const MYSTERY_VERSION: u32 = 4;
const RARE_SEED_MULT: u64 = 3;
const LEGENDARY_SEED_MULT: u64 = 6;
const BUFF_MULT: u64 = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MysteryRarity {
    Rare,
    Legendary,
}

impl MysteryRarity {
    pub fn label(self) -> &'static str {
        match self {
            MysteryRarity::Rare => "稀有",
            MysteryRarity::Legendary => "传说",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MysteryKind {
    Seed,
    Card,
    Furniture,
    Buff,
}

impl MysteryKind {
    pub fn label(self) -> &'static str {
        match self {
            MysteryKind::Seed => "种子",
            MysteryKind::Card => "一键卡",
            MysteryKind::Furniture => "家具",
            MysteryKind::Buff => "加成",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MysteryOffer {
    pub id: String,
    pub kind: MysteryKind,
    pub item_key: String,
    pub crop_id: String,
    pub name: String,
    pub desc: String,
    /// 分属：农场 / 牧场 / 钓鱼 / 制造。
    #[serde(default)]
    pub tag: String,
    pub price: u64,
    pub rarity: MysteryRarity,
    pub featured: bool,
    pub sold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MysteryShopState {
    pub day: String,
    #[serde(default)]
    pub version: u32,
    pub offers: Vec<MysteryOffer>,
}

impl MysteryShopState {
    /// 跨天刷新；布局版本变化时也强制重刷（旧档 4+4 → 新档 3+5）。
    pub fn ensure_day(&mut self, today: &str, level: u32, content: &ContentPack) -> bool {
        if self.day == today && self.version == MYSTERY_VERSION {
            return false;
        }
        self.day = today.to_string();
        self.version = MYSTERY_VERSION;
        self.offers = generate(today, level, content);
        true
    }

    pub fn offer(&self, id: &str) -> Option<&MysteryOffer> {
        self.offers.iter().find(|offer| offer.id == id)
    }

    pub fn mark_sold(&mut self, id: &str) -> bool {
        let Some(offer) = self.offers.iter_mut().find(|offer| offer.id == id) else {
            return false;
        };
        if offer.sold {
            return false;
        }
        offer.sold = true;
        true
    }
}

fn generate(today: &str, level: u32, content: &ContentPack) -> Vec<MysteryOffer> {
    let mut crops: Vec<&CropDef> = content
        .crops
        .iter()
        .filter(|crop| crop.unlock_level > level)
        .collect();
    if crops.is_empty() {
        crops = content.crops.iter().collect();
    }
    crops.sort_by_key(|crop| (crop.unlock_level, crop.id.clone()));

    let legendary = roll(today, 0x5EED) % 100 < MYSTERY_LEGENDARY_PCT;
    let mut used_crops = BTreeSet::new();
    let mut used_buffs = BTreeSet::new();
    let mut offers = Vec::with_capacity(MYSTERY_OFFERS);

    let featured_rarity = if legendary {
        MysteryRarity::Legendary
    } else {
        MysteryRarity::Rare
    };
    let featured = seed_offer(
        today,
        crops.last().copied(),
        featured_rarity,
        true,
        0,
        &mut used_crops,
    );
    if let Some(offer) = featured {
        offers.push(offer);
    }

    for index in 0..MYSTERY_NORMAL_OFFERS {
        let offer = if index < 2 {
            let roll = roll(today, 0x100 + index as u64);
            let candidate = crops
                .iter()
                .copied()
                .filter(|crop| !used_crops.contains(crop.id.as_str()))
                .nth((roll as usize) % crops.len().max(1));
            seed_offer(
                today,
                candidate,
                MysteryRarity::Rare,
                false,
                index + 1,
                &mut used_crops,
            )
        } else if index == 2 {
            card_offer(today, index + 1)
        } else if index == 3 {
            furniture_offer(today, index + 1)
        } else {
            buff_offer(today, index, index - 4, &mut used_buffs)
        };
        if let Some(offer) = offer {
            offers.push(offer);
        }
    }

    offers
}

fn seed_offer(
    today: &str,
    crop: Option<&CropDef>,
    rarity: MysteryRarity,
    featured: bool,
    index: usize,
    used: &mut BTreeSet<String>,
) -> Option<MysteryOffer> {
    let crop = crop?;
    if !used.insert(crop.id.clone()) {
        return None;
    }
    let multiplier = match rarity {
        MysteryRarity::Rare => RARE_SEED_MULT,
        MysteryRarity::Legendary => LEGENDARY_SEED_MULT,
    };
    let price = crop
        .seed_price
        .saturating_mul(multiplier)
        .max(300);
    Some(MysteryOffer {
        id: format!("{today}-m{index}"),
        kind: MysteryKind::Seed,
        item_key: format!("rare_seed:{}", crop.id),
        crop_id: crop.id.clone(),
        name: format!("{}·{}种子", rarity.label(), crop.name),
        desc: format!(
            "可越级种一次 · 成熟 {} 分 · 单季 {} 个",
            crop.grow_minutes, crop.yield_per_season
        ),
        tag: "农场".into(),
        price,
        rarity,
        featured,
        sold: false,
    })
}

fn card_offer(today: &str, index: usize) -> Option<MysteryOffer> {
    Some(MysteryOffer {
        id: format!("{today}-m{index}"),
        kind: MysteryKind::Card,
        item_key: "craft:farm_card".into(),
        crop_id: String::new(),
        name: "农场一键卡".into(),
        desc: "一键除虫/浇水/收获/补种（只用仓库普通种子）".into(),
        tag: "农场".into(),
        price: 8000,
        rarity: MysteryRarity::Rare,
        featured: false,
        sold: false,
    })
}

fn furniture_offer(today: &str, index: usize) -> Option<MysteryOffer> {
    let choices = ["mystery_glass_tank", "mystery_snow_tv"];
    let id = choices[(roll(today, 0xF10) as usize) % choices.len()];
    let def = crate::room::furniture_def(id)?;
    let legendary = id == "mystery_glass_tank";
    Some(MysteryOffer {
        id: format!("{today}-m{index}"),
        kind: MysteryKind::Furniture,
        item_key: format!("furniture:{id}"),
        crop_id: String::new(),
        name: format!("{}·{}", if legendary { "传说" } else { "稀有" }, def.name),
        desc: "买下后进家具仓库，可在牧村装修里摆".into(),
        tag: "家具".into(),
        price: if legendary { 36_000 } else { 22_000 },
        rarity: if legendary {
            MysteryRarity::Legendary
        } else {
            MysteryRarity::Rare
        },
        featured: false,
        sold: false,
    })
}

fn buff_offer(
    today: &str,
    index: usize,
    slot: usize,
    used: &mut BTreeSet<String>,
) -> Option<MysteryOffer> {
    let item_key = match slot {
        0 => {
            if roll(today, 0xF00) % 2 == 0 {
                "craft:mystery_elixir"
            } else {
                "craft:mystery_tonic"
            }
        }
        1 => "craft:ranch_elixir",
        2 => "craft:fishing_elixir",
        3 => "craft:craft_elixir",
        4 => "craft:coin_elixir",
        _ => return None,
    };
    if !used.insert(item_key.to_string()) {
        return None;
    }
    let item = craft_item(item_key)?;
    let buff = buff_item(item_key)?;
    Some(MysteryOffer {
        id: format!("{today}-m{}", index + 1),
        kind: MysteryKind::Buff,
        item_key: item_key.to_string(),
        crop_id: String::new(),
        name: format!("稀有·{}", item.name),
        desc: buff.desc.to_string(),
        tag: buff_tag(item_key).into(),
        price: item.price.saturating_mul(BUFF_MULT).max(600),
        rarity: MysteryRarity::Rare,
        featured: false,
        sold: false,
    })
}

fn buff_tag(key: &str) -> &'static str {
    match key {
        "craft:ranch_elixir" => "牧场",
        "craft:fishing_elixir" => "钓鱼",
        "craft:craft_elixir" => "制造",
        "craft:coin_elixir" => "金币",
        _ => "农场",
    }
}

fn roll(day: &str, salt: u64) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ salt;
    for byte in day.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^ (hash >> 33)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_is_deterministic_and_has_featured() {
        let content = ContentPack::starter();
        let mut first = MysteryShopState::default();
        let mut second = MysteryShopState::default();
        assert!(first.ensure_day("2026-10-24", 12, &content));
        assert!(second.ensure_day("2026-10-24", 12, &content));
        assert_eq!(first.offers, second.offers);
        assert_eq!(first.offers.len(), MYSTERY_OFFERS);
        assert_eq!(first.offers.iter().filter(|o| o.featured).count(), 1);
        assert_eq!(
            first
                .offers
                .iter()
                .filter(|o| o.kind == MysteryKind::Buff)
                .count(),
            5
        );
        assert_eq!(
            first
                .offers
                .iter()
                .filter(|o| o.kind == MysteryKind::Card)
                .count(),
            1
        );
        assert_eq!(
            first
                .offers
                .iter()
                .filter(|o| o.kind == MysteryKind::Furniture)
                .count(),
            1
        );
        assert!(first.offers.iter().all(|o| o.price > 0));
        assert!(first
            .offers
            .iter()
            .all(|o| o.rarity == MysteryRarity::Rare || o.featured));
        assert!(!first.ensure_day("2026-10-24", 12, &content));
    }
}
