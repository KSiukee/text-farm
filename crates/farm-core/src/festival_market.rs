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
pub const MAJOR_PREPARE_DAYS: u32 = 5;
pub const MAX_SELLS: u32 = 3;
pub const MAJOR_MAX_SELLS: u32 = 5;

pub fn prepare_days(name: &str) -> u32 {
    if crate::npc_story::is_major_festival(name) {
        MAJOR_PREPARE_DAYS
    } else {
        PREPARE_DAYS
    }
}

pub fn max_sells(major: bool) -> u32 {
    if major {
        MAJOR_MAX_SELLS
    } else {
        MAX_SELLS
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FestivalMarketState {
    pub year: i32,
    pub name: String,
    pub target_day: String,
    pub major: bool,
    pub requests: Vec<MarketRequest>,
    pub supplied: BTreeSet<String>,
    pub contribution: u32,
    pub offers: Vec<MarketOffer>,
    pub bought: BTreeSet<String>,
    pub sold: u32,
    pub gifts: Vec<MarketGift>,
    pub gifts_claimed: BTreeSet<u32>,
    /// 本次集市已经聊过的人（vendor:... / crowd:...）。
    #[serde(default)]
    pub talked: BTreeSet<String>,
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
    pub vendor: String,
    pub stall: String,
    pub item_key: String,
    pub item_name: String,
    pub price: u64,
    pub talk: String,
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
    pub major: bool,
    pub days_until: u32,
    pub contribution: u32,
    pub requests: Vec<MarketRequestView>,
    pub offers: Vec<MarketOfferView>,
    pub gifts: Vec<MarketGiftView>,
    pub crowd: Vec<MarketCrowdView>,
    pub sold: u32,
    pub max_sells: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCrowdView {
    pub id: String,
    pub name: String,
    pub text: String,
    pub talk: String,
    pub named: bool,
    pub talked: bool,
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
    pub vendor: String,
    pub stall: String,
    pub item_name: String,
    pub price: u64,
    pub talk: String,
    pub bought: bool,
    pub talked: bool,
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
        let major = crate::npc_story::is_major_festival(name);
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
        if major {
            requests.push(MarketRequest {
                id: "r3".into(),
                npc_name: "老陶".into(),
                item_key: if level >= 3 {
                    "crop:caomei".into()
                } else {
                    "seed:luobo".into()
                },
                item_name: if level >= 3 {
                    crop_name("caomei")
                } else {
                    format!("{}种子", crop_name("luobo"))
                },
                qty: 3,
                coins: 120,
                contribution: 3,
            });
            requests.push(MarketRequest {
                id: "r4".into(),
                npc_name: "六月".into(),
                item_key: if level >= 5 {
                    "crop:yumi".into()
                } else {
                    "seed:xiaobaicai".into()
                },
                item_name: if level >= 5 {
                    crop_name("yumi")
                } else {
                    format!("{}种子", crop_name("xiaobaicai"))
                },
                qty: 3,
                coins: 120,
                contribution: 3,
            });
        }

        let mut offer_ids = vec!["caomei", "yumi", "xigua"];
        if major {
            offer_ids.push("putao");
            offer_ids.push("xiangrikui");
        }
        let offers = offer_ids
            .iter()
            .filter_map(|id| {
                let crop = find(id)?;
                let (vendor, stall) = stall_info(id);
                Some(MarketOffer {
                    id: format!("o-{id}"),
                    vendor: vendor.into(),
                    stall: stall.into(),
                    item_key: format!("seed:{id}"),
                    item_name: format!("{}种子", crop.name),
                    price: (crop.seed_price * 2).max(60),
                    talk: stall_talk(id).into(),
                })
            })
            .collect();

        let gifts: Vec<MarketGift> = if major {
            [
                (5, "大集回礼·一", "caomei", 150),
                (10, "大集回礼·二", "yumi", 300),
                (15, "大集回礼·三", "xigua", 500),
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
            .collect()
        } else {
            [
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
            .collect()
        };

        Self {
            year,
            name: name.to_string(),
            target_day,
            major,
            requests,
            supplied: BTreeSet::new(),
            contribution: 0,
            offers,
            bought: BTreeSet::new(),
            sold: 0,
            gifts,
            gifts_claimed: BTreeSet::new(),
            talked: BTreeSet::new(),
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

fn stall_info(id: &str) -> (&'static str, &'static str) {
    match id {
        "caomei" => ("卖种子的阿婆", "种子摊"),
        "yumi" => ("货郎老周", "杂货担"),
        "xigua" => ("隔壁村的老把式", "瓜摊"),
        "putao" => ("绣坊的秦婶", "绣品摊"),
        "xiangrikui" => ("木匠阿梁", "木器摊"),
        _ => ("赶集的村人", "临时摊"),
    }
}

fn stall_talk(id: &str) -> &'static str {
    match id {
        "caomei" => "阿婆把种子袋口拢了拢：这批是自家留的，出芽齐，别埋太深。",
        "yumi" => "老周摇了下拨浪鼓：下个月还来，针线、盐巴都带。",
        "xigua" => "老把式敲了敲瓜：听声，闷的才熟。",
        "putao" => "秦婶抖开一块花布：这颜色衬你，做窗帘好看。",
        "xiangrikui" => "阿梁蹲下看了看榫头：松了就来，别等散架。",
        _ => "摊主笑着招呼：随便看看，不买也没事。",
    }
}

fn crowd(
    name: &str,
    major: bool,
    phase: &str,
    talked: &BTreeSet<String>,
) -> Vec<MarketCrowdView> {
    let mut lines = Vec::new();
    let mut add = |id: &str, name: &str, text: String, talk: &str, named: bool| {
        lines.push(MarketCrowdView {
            id: id.to_string(),
            name: name.to_string(),
            text,
            talk: talk.to_string(),
            named,
            talked: talked.contains(id),
        });
    };
    add(
        "crowd-ahe",
        "阿禾",
        if phase == "prepare" {
            "把长凳搬到树下，等会儿有人坐。".into()
        } else {
            "菜摊前挤着人，她一边称菜一边笑。".into()
        },
        "阿禾说：今天不急着收摊，你也歇会儿。",
        true,
    );
    add(
        "crowd-xiaoman",
        "小满",
        if phase == "prepare" {
            "灶上蒸着米糕，香味先到了。".into()
        } else {
            "米糕切成一角一角，热气还没散。".into()
        },
        "小满说：米糕趁热吃，凉了就硬了。",
        true,
    );
    add(
        "crowd-qingxing",
        "青杏",
        if phase == "prepare" {
            "把药箱放在棚子边，怕有人磕着。".into()
        } else {
            "给一个扭了脚的孩子看脚，没要钱。".into()
        },
        "青杏说：逛归逛，别踩湿滑的地方。",
        true,
    );
    add(
        "crowd-laotao",
        "老陶",
        if phase == "prepare" {
            "最后一根横梁敲进去，棚子稳了。".into()
        } else {
            "木器摊前有人试凳子，他蹲着看榫头。".into()
        },
        "老陶说：这棚子再撑十年没问题。",
        true,
    );
    add(
        "crowd-liuyue",
        "六月",
        if phase == "prepare" {
            "河风把灯笼吹得直转。".into()
        } else {
            "鱼摊上的水花溅到人脚边，她笑着道歉。".into()
        },
        "六月说：河上今天有风，灯会漂得远。",
        true,
    );
    add(
        "crowd-sugar",
        "卖糖画的老汉",
        if phase == "prepare" {
            "先熬一锅糖，明天给孩子画兔子。".into()
        } else {
            "小铜勺一转，糖丝在石板上变成一只鸟。".into()
        },
        "老汉说：糖画不经放，买了就吃。",
        false,
    );
    add(
        "crowd-basket",
        "挑担的婶子",
        if phase == "prepare" {
            "从邻村赶来，担子还没放下。".into()
        } else {
            "担子一头是花布，一头是自家晒的干菜。".into()
        },
        "婶子说：干菜拿水一泡，炖肉最香。",
        false,
    );
    add(
        "crowd-zhou",
        "货郎老周",
        if phase == "prepare" {
            "摇着拨浪鼓，在村口转了一圈。".into()
        } else {
            "拨浪鼓一响，半条街的孩子都围过去了。".into()
        },
        "老周说：下个月还来，给你留个好物件。",
        true,
    );
    add(
        "crowd-kids",
        "两个小孩",
        if phase == "prepare" {
            "围着空摊子跑，被大人喊回来。".into()
        } else {
            "一人手里一支糖画，谁也不肯先咬。".into()
        },
        "两个小孩说：等会儿还要去看戏台。",
        false,
    );
    add(
        "crowd-hat",
        "戴草帽的姑娘",
        if phase == "prepare" {
            "蹲在地上摆自家编的小竹篮。".into()
        } else {
            "竹篮卖得只剩最后两个，她正收摊。".into()
        },
        "姑娘说：篮子是自己编的，装菜不漏。",
        false,
    );
    if major {
        add(
            "crowd-liang",
            "隔壁村的木匠阿梁",
            "挑着两把新椅子来赶集，当场给人修凳腿。".into(),
            "阿梁说：椅子坐坏了别扔，拿来我看看。",
            true,
        );
        add(
            "crowd-stage",
            "戏台边",
            if phase == "prepare" {
                "有人在试锣鼓，断断续续敲了两声。".into()
            } else {
                "锣鼓一响，老人孩子都往戏台那边走。".into()
            },
            "戏台边有人说：再等一会儿就开唱。",
            false,
        );
    }
    let _ = name;
    lines
}

pub fn next_festival(today: &str) -> Option<(&'static str, u32, String)> {
    (0..=MAJOR_PREPARE_DAYS).find_map(|offset| {
        let day = add_days(today, offset);
        crate::npc_story::festival_of(&day).and_then(|name| {
            (offset <= prepare_days(name)).then_some((name, offset, day))
        })
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
            vendor: offer.vendor.clone(),
            stall: offer.stall.clone(),
            item_name: offer.item_name.clone(),
            price: offer.price,
            talk: offer.talk.clone(),
            bought: market.bought.contains(&offer.id),
            talked: market
                .talked
                .contains(&format!("vendor:{}", offer.id)),
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
        major: market.major,
        days_until,
        contribution: market.contribution,
        requests,
        offers,
        gifts,
        crowd: crowd(&market.name, market.major, phase, &market.talked),
        sold: market.sold,
        max_sells: max_sells(market.major),
    })
}
