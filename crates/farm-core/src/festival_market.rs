//! 节日集市：节前 3 天备货，节日当天开市。
//!
//! 不新增货币：备货给金币和“集市贡献”，贡献只用来领回礼；
//! 限定摊用金币买，寄卖摊按原价 +30% 收 3 次。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::content::ContentPack;
use crate::model::GameState;
use crate::orders::add_days;

pub const PREPARE_DAYS: u32 = 3;
pub const MAX_SELLS: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FestivalMarketState {
    pub year: i32,
    pub name: String,
    pub target_day: String,
    pub requests: Vec<MarketRequest>,
    pub supplied: BTreeSet<String>,
    pub contribution: u32,
    pub offers: Vec<MarketOffer>,
    pub bought: BTreeSet<String>,
    pub sold: u32,
    pub gifts: Vec<MarketGift>,
    pub gifts_claimed: BTreeSet<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MarketRequest {
    pub id: String,
    pub npc_name: String,
    pub item_key: String,
    pub item_name: String,
    pub qty: u32,
    pub coins: u64,
    pub contribution: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MarketOffer {
    pub id: String,
    pub item_key: String,
    pub item_name: String,
    pub price: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MarketGift {
    pub threshold: u32,
    pub label: String,
    pub item_key: String,
    pub item_name: String,
    pub qty: u32,
    pub coins: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FestivalMarketView {
    pub phase: String,
    pub name: String,
    pub target_day: String,
    pub days_until: u32,
    pub contribution: u32,
    pub requests: Vec<MarketRequestView>,
    pub offers: Vec<MarketOfferView>,
    pub gifts: Vec<MarketGiftView>,
    pub sold: u32,
    pub max_sells: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRequestView {
    pub id: String,
    pub npc_name: String,
    pub item_name: String,
    pub qty: u32,
    pub have: u32,
    pub coins: u64,
    pub contribution: u32,
    pub supplied: bool,
    pub can_supply: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketOfferView {
    pub id: String,
    pub item_name: String,
    pub price: u64,
    pub bought: bool,
    pub can_buy: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketGiftView {
    pub threshold: u32,
    pub label: String,
    pub item_name: String,
    pub qty: u32,
    pub coins: u64,
    pub claimed: bool,
    pub can_claim: bool,
}

impl FestivalMarketState {
    /// 跨天时滚动到下一个节日；返回是否重建。
    pub fn ensure(&mut self, today: &str, level: u32, content: &ContentPack) -> bool {
        let Some((name, offset, target_day)) = next_festival(today) else {
            let changed = !self.name.is_empty();
            if changed {
                *self = Self::default();
            }
            return changed;
        };
        let year = crate::npc_story::year_of(&target_day);
        if self.name == name && self.year == year {
            self.target_day = target_day;
            return false;
        }
        *self = Self::generate(name, year, target_day, offset, level, content);
        true
    }

    fn generate(
        name: &str,
        year: i32,
        target_day: String,
        _offset: u32,
        level: u32,
        content: &ContentPack,
    ) -> Self {
        let crops = &content.crops;
        let find = |id: &str| crops.iter().find(|crop| crop.id == id);
        let crop_name = |id: &str| {
            find(id)
                .map(|crop| crop.name.clone())
                .unwrap_or_else(|| id.to_string())
        };
        let mut requests = Vec::new();
        requests.push(MarketRequest {
            id: "r0".into(),
            npc_name: "阿禾".into(),
            item_key: "crop:xiaobaicai".into(),
            item_name: crop_name("xiaobaicai"),
            qty: 4,
            coins: 80,
            contribution: 2,
        });
        requests.push(MarketRequest {
            id: "r1".into(),
            npc_name: "青杏".into(),
            item_key: "crop:luobo".into(),
            item_name: crop_name("luobo"),
            qty: 3,
            coins: 100,
            contribution: 3,
        });
        if level >= 3 {
            requests.push(MarketRequest {
                id: "r2".into(),
                npc_name: "小满".into(),
                item_key: "crop:caomei".into(),
                item_name: crop_name("caomei"),
                qty: 2,
                coins: 160,
                contribution: 4,
            });
        } else {
            requests.push(MarketRequest {
                id: "r2".into(),
                npc_name: "小满".into(),
                item_key: "seed:xiaobaicai".into(),
                item_name: format!("{}种子", crop_name("xiaobaicai")),
                qty: 3,
                coins: 60,
                contribution: 4,
            });
        }

        let offer_ids = ["caomei", "yumi", "xigua"];
        let offers = offer_ids
            .iter()
            .filter_map(|id| {
                let crop = find(id)?;
                Some(MarketOffer {
                    id: format!("o-{id}"),
                    item_key: format!("seed:{id}"),
                    item_name: format!("{}种子", crop.name),
                    price: (crop.seed_price * 2).max(60),
                })
            })
            .collect();

        let gifts = [
            (3, "集市回礼·一", "caomei", 100),
            (6, "集市回礼·二", "yumi", 200),
            (9, "集市回礼·三", "xigua", 300),
        ]
        .into_iter()
        .filter_map(|(threshold, label, id, coins)| {
            let crop = find(id)?;
            Some(MarketGift {
                threshold,
                label: label.into(),
                item_key: format!("seed:{id}"),
                item_name: format!("{}种子", crop.name),
                qty: 1,
                coins,
            })
        })
        .collect();

        Self {
            year,
            name: name.to_string(),
            target_day,
            requests,
            supplied: BTreeSet::new(),
            contribution: 0,
            offers,
            bought: BTreeSet::new(),
            sold: 0,
            gifts,
            gifts_claimed: BTreeSet::new(),
        }
    }

    pub fn request(&self, id: &str) -> Option<&MarketRequest> {
        self.requests.iter().find(|request| request.id == id)
    }

    pub fn offer(&self, id: &str) -> Option<&MarketOffer> {
        self.offers.iter().find(|offer| offer.id == id)
    }

    pub fn gift(&self, threshold: u32) -> Option<&MarketGift> {
        self.gifts.iter().find(|gift| gift.threshold == threshold)
    }
}

pub fn next_festival(today: &str) -> Option<(&'static str, u32, String)> {
    (0..=PREPARE_DAYS).find_map(|offset| {
        let day = add_days(today, offset);
        crate::npc_story::festival_of(&day).map(|name| (name, offset, day))
    })
}

pub fn active_today(state: &GameState) -> bool {
    let market = &state.festival_market;
    !market.name.is_empty() && !market.target_day.is_empty() && state.daily.day <= market.target_day
}

pub fn view(state: &GameState) -> Option<FestivalMarketView> {
    let market = &state.festival_market;
    if !active_today(state) {
        return None;
    }
    let days_until = if state.daily.day == market.target_day {
        0
    } else {
        next_festival(&state.daily.day)
            .map(|(_, offset, _)| offset)
            .unwrap_or(0)
    };
    let phase = if days_until == 0 { "open" } else { "prepare" };
    let requests = market
        .requests
        .iter()
        .map(|request| {
            let have = state.inventory.get(&request.item_key).copied().unwrap_or(0);
            MarketRequestView {
                id: request.id.clone(),
                npc_name: request.npc_name.clone(),
                item_name: request.item_name.clone(),
                qty: request.qty,
                have,
                coins: request.coins,
                contribution: request.contribution,
                supplied: market.supplied.contains(&request.id),
                can_supply: phase == "prepare"
                    && !market.supplied.contains(&request.id)
                    && have >= request.qty,
            }
        })
        .collect();
    let offers = market
        .offers
        .iter()
        .map(|offer| MarketOfferView {
            id: offer.id.clone(),
            item_name: offer.item_name.clone(),
            price: offer.price,
            bought: market.bought.contains(&offer.id),
            can_buy: phase == "open"
                && !market.bought.contains(&offer.id)
                && state.coins >= offer.price,
        })
        .collect();
    let gifts = market
        .gifts
        .iter()
        .map(|gift| MarketGiftView {
            threshold: gift.threshold,
            label: gift.label.clone(),
            item_name: gift.item_name.clone(),
            qty: gift.qty,
            coins: gift.coins,
            claimed: market.gifts_claimed.contains(&gift.threshold),
            can_claim: phase == "open"
                && market.contribution >= gift.threshold
                && !market.gifts_claimed.contains(&gift.threshold),
        })
        .collect();
    Some(FestivalMarketView {
        phase: phase.into(),
        name: market.name.clone(),
        target_day: market.target_day.clone(),
        days_until,
        contribution: market.contribution,
        requests,
        offers,
        gifts,
        sold: market.sold,
        max_sells: MAX_SELLS,
    })
}
