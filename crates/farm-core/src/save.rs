use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::content::ContentPack;
use crate::error::SaveError;
use crate::model::{Game, GameState, SAVE_SCHEMA};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFile {
    pub schema: u32,
    pub saved_at: i64,
    pub state: GameState,
    pub content: ContentPack,
}

fn ensure_stats(state: &mut serde_json::Map<String, Value>, schema: u32) {
    let stats = state.entry("stats").or_insert_with(|| json!({}));
    let Some(stats) = stats.as_object_mut() else {
        return;
    };
    for key in ["planted", "harvests", "coinsEarned", "levelUps"] {
        stats.entry(key).or_insert(json!(0));
    }
    if schema >= 2 {
        stats.entry("pestsCleared").or_insert(json!(0));
        stats.entry("itemsSold").or_insert(json!(0));
        stats.entry("signIns").or_insert(json!(0));
        stats.entry("plotsUnlocked").or_insert(json!(0));
    }
}

/// 把旧存档逐级升到当前 schema。返回的 Value 可再反序列化成 SaveFile。
pub fn migrate_save(mut value: Value) -> Result<Value, SaveError> {
    let mut schema = value.get("schema").and_then(Value::as_u64).unwrap_or(0) as u32;

    if schema > SAVE_SCHEMA {
        return Err(SaveError::TooNew(schema));
    }
    if schema == 0 {
        return Err(SaveError::Unknown(0));
    }

    if schema < 2 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        state
            .entry("facilities")
            .or_insert_with(|| json!({"sprinkler": false, "harvester": false, "windmill": false}));
        state.entry("settings").or_insert_with(|| {
            json!({
                "expRate": 1,
                "fontScale": 100,
                "skin": "classic",
                "onboarded": false
            })
        });
        ensure_stats(state, 1);
        state.insert("schema".into(), json!(2));
        value["schema"] = json!(2);
        schema = 2;
    }

    if schema < 3 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        state.entry("lastTickAt").or_insert(json!(0));
        state.entry("daily").or_insert_with(|| {
            json!({
                "day": "",
                "harvests": 0,
                "waterings": 0,
                "pestsCleared": 0,
                "signIn": false,
                "claimedHarvest": false,
                "claimedWater": false,
                "claimedPest": false
            })
        });
        state.entry("codex").or_insert_with(|| json!({}));
        state.entry("achievements").or_insert_with(|| json!({}));
        ensure_stats(state, 2);

        let unlocked = state
            .get("plots")
            .and_then(Value::as_array)
            .map(|plots| {
                plots
                    .iter()
                    .filter(|plot| plot.get("unlocked").and_then(Value::as_bool) == Some(true))
                    .count() as u64
            })
            .unwrap_or(0);
        if let Some(stats) = state.get_mut("stats").and_then(Value::as_object_mut) {
            let entry = stats.entry("plotsUnlocked").or_insert(json!(0));
            let current = entry.as_u64().unwrap_or(0);
            if current < unlocked {
                *entry = json!(unlocked);
            }
        }

        if let Some(plots) = state.get_mut("plots").and_then(Value::as_array_mut) {
            for plot in plots {
                if let Some(crop) = plot.get_mut("crop").and_then(Value::as_object_mut) {
                    crop.entry("pest").or_insert(Value::Null);
                }
            }
        }

        state.insert("schema".into(), json!(3));
        value["schema"] = json!(3);
        schema = 3;
    }

    if schema < 4 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        state.entry("npcs").or_insert_with(|| json!([]));
        state.entry("npcMessages").or_insert_with(|| json!([]));
        state.entry("lastNpcTickAt").or_insert(json!(0));

        if let Some(daily) = state.get_mut("daily").and_then(Value::as_object_mut) {
            daily.entry("steals").or_insert(json!(0));
            daily.entry("helps").or_insert(json!(0));
        }

        state.insert("schema".into(), json!(4));
        value["schema"] = json!(4);
        schema = 4;
    }

    if schema < 5 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        if let Some(plots) = state.get_mut("plots").and_then(Value::as_array_mut) {
            for plot in plots {
                if let Some(plot) = plot.as_object_mut() {
                    plot.entry("tier").or_insert(json!(0));
                }
            }
        }

        state.insert("schema".into(), json!(5));
        value["schema"] = json!(5);
        schema = 5;
    }

    if schema < 6 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        if let Some(plots) = state.get_mut("plots").and_then(Value::as_array_mut) {
            for plot in plots {
                if let Some(crop) = plot.get_mut("crop").and_then(Value::as_object_mut) {
                    crop.entry("disaster").or_insert(Value::Null);
                }
            }
        }

        state.insert("schema".into(), json!(6));
        value["schema"] = json!(6);
        schema = 6;
    }

    if schema < 7 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        state.entry("season").or_insert(json!("spring"));
        if let Some(plots) = state.get_mut("plots").and_then(Value::as_array_mut) {
            for plot in plots {
                if let Some(crop) = plot.get_mut("crop").and_then(Value::as_object_mut) {
                    crop.entry("offSeason").or_insert(json!(false));
                }
            }
        }

        state.insert("schema".into(), json!(7));
        value["schema"] = json!(7);
        schema = 7;
    }

    if schema < 8 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        if let Some(settings) = state.get_mut("settings").and_then(Value::as_object_mut) {
            settings.entry("region").or_insert(json!("north"));
        }
        if let Some(facilities) = state.get_mut("facilities").and_then(Value::as_object_mut) {
            facilities.entry("greenhouse").or_insert(json!(false));
        }
        if let Some(plots) = state.get_mut("plots").and_then(Value::as_array_mut) {
            for plot in plots {
                if let Some(crop) = plot.get_mut("crop").and_then(Value::as_object_mut) {
                    crop.entry("growthFactor").or_insert(json!(1000));
                    crop.entry("greenhouse").or_insert(json!(false));
                }
            }
        }

        state.insert("schema".into(), json!(8));
        value["schema"] = json!(8);
        schema = 8;
    }

    if schema < 9 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        if let Some(plots) = state.get_mut("plots").and_then(Value::as_array_mut) {
            for plot in plots {
                if let Some(crop) = plot.get_mut("crop").and_then(Value::as_object_mut) {
                    crop.entry("withered").or_insert(json!(false));
                }
            }
        }

        state.insert("schema".into(), json!(9));
        value["schema"] = json!(9);
        schema = 9;
    }

    if schema < 10 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        if let Some(plots) = state.get_mut("plots").and_then(Value::as_array_mut) {
            for plot in plots {
                if let Some(object) = plot.as_object_mut() {
                    object.entry("kind").or_insert(json!("field"));
                }
            }
        }

        state.insert("schema".into(), json!(10));
        value["schema"] = json!(10);
        schema = 10;
    }

    if schema < 11 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        if let Some(facilities) = state.get_mut("facilities").and_then(Value::as_object_mut) {
            let has_windmill = facilities
                .get("windmill")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let has_greenhouse = facilities
                .get("greenhouse")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let windmills = facilities
                .get("windmills")
                .and_then(Value::as_u64)
                .unwrap_or(if has_windmill { 1 } else { 0 });
            let greenhouses = facilities
                .get("greenhouses")
                .and_then(Value::as_u64)
                .unwrap_or(if has_greenhouse { 1 } else { 0 });
            facilities.insert("windmills".into(), json!(windmills));
            facilities.insert("greenhouses".into(), json!(greenhouses));
            facilities.entry("damaged_windmills").or_insert(json!(0));
        }

        state.insert("schema".into(), json!(11));
        value["schema"] = json!(11);
        schema = 11;
    }

    if schema < 12 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state.entry("pet").or_insert(Value::Null);
        state.entry("lastRaidDay").or_insert(json!(""));
        state.insert("schema".into(), json!(12));
        value["schema"] = json!(12);
        schema = 12;
    }

    if schema < 13 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pet) = state.get_mut("pet").and_then(Value::as_object_mut) {
            let species = pet.get("species").and_then(Value::as_str).unwrap_or("dog");
            let breed = if species == "cat" {
                "domestic_cat"
            } else {
                "corgi"
            };
            pet.entry("breed").or_insert(json!(breed));
        }
        state.insert("schema".into(), json!(13));
        value["schema"] = json!(13);
        schema = 13;
    }

    if schema < 14 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;

        let legacy_pet = state.remove("pet");
        let mut pets = state
            .get_mut("pets")
            .and_then(Value::as_array_mut)
            .cloned()
            .unwrap_or_default();
        if pets.is_empty() {
            if let Some(pet) = legacy_pet {
                if !pet.is_null() {
                    pets.push(pet);
                }
            }
        }
        for (index, pet) in pets.iter_mut().enumerate() {
            if let Some(object) = pet.as_object_mut() {
                object
                    .entry("id")
                    .or_insert(json!(format!("pet_{}", index + 1)));
                object.entry("boarded").or_insert(json!(false));
            }
        }
        state.insert("pets".into(), json!(pets));
        state.insert("petSeq".into(), json!(pets.len() as u64));
        state.entry("boardingDebt").or_insert(json!(0));
        state.entry("lastBoardingMonth").or_insert(json!(""));
        state.insert("schema".into(), json!(14));
        value["schema"] = json!(14);
        schema = 14;
    }

    if schema < 15 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(object) = pet.as_object_mut() {
                    object.entry("digSkill").or_insert(json!(1));
                    object.entry("digExp").or_insert(json!(0));
                    object.entry("guardSkill").or_insert(json!(1));
                    object.entry("guardExp").or_insert(json!(0));
                }
            }
        }
        state.insert("schema".into(), json!(15));
        value["schema"] = json!(15);
        schema = 15;
    }

    if schema < 16 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state.entry("petCare").or_insert(json!({
            "bed": false,
            "bowl": false,
            "toy": false,
            "medicine": 0
        }));
        state.insert("schema".into(), json!(16));
        value["schema"] = json!(16);
        schema = 16;
    }

    if schema < 17 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let care = state
            .get_mut("petCare")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let old_bed = care
            .remove("bed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let old_toy = care
            .remove("toy")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let old_bowl = care
            .remove("bowl")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        care.entry("dogBed").or_insert(json!(old_bed));
        care.entry("catBed").or_insert(json!(old_bed));
        care.entry("dogToy").or_insert(json!(old_toy));
        care.entry("catToy").or_insert(json!(old_toy));
        care.entry("bowls")
            .or_insert(json!(if old_bowl { 3 } else { 0 }));
        if let Some(facilities) = state.get_mut("facilities").and_then(Value::as_object_mut) {
            facilities.entry("trainingCenter").or_insert(json!(false));
        }
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(object) = pet.as_object_mut() {
                    object.entry("training").or_insert(json!({
                        "guard": 0, "dig": 0, "mischief": 0, "companion": 0
                    }));
                    object.entry("lastMischiefDay").or_insert(json!(""));
                }
            }
        }
        state.insert("schema".into(), json!(17));
        value["schema"] = json!(17);
        schema = 17;
    }

    if schema < 18 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(care) = state.get_mut("petCare").and_then(Value::as_object_mut) {
            care.entry("dogBedBroken").or_insert(json!(false));
            care.entry("catBedBroken").or_insert(json!(false));
            care.entry("dogToyBroken").or_insert(json!(false));
            care.entry("catToyBroken").or_insert(json!(false));
        }
        state.insert("schema".into(), json!(18));
        value["schema"] = json!(18);
        schema = 18;
    }

    if schema < 19 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for (index, pet) in pets.iter_mut().enumerate() {
                if let Some(object) = pet.as_object_mut() {
                    if !object.contains_key("personality") {
                        let name = object.get("name").and_then(Value::as_str).unwrap_or("宠物");
                        let mut seed = 0x9E37_79B9_7F4A_7C15u64
                            ^ (index as u64 + 1).wrapping_mul(1_099_511_628_211);
                        for byte in name.bytes() {
                            seed ^= u64::from(byte);
                            seed = seed.wrapping_mul(1_099_511_628_211);
                        }
                        object.insert(
                            "personality".into(),
                            serde_json::to_value(crate::pet::PetPersonality::from_seed(seed))
                                .expect("serialize personality"),
                        );
                    }
                }
            }
        }
        state.insert("schema".into(), json!(19));
        value["schema"] = json!(19);
        schema = 19;
    }

    if schema < 20 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(object) = pet.as_object_mut() {
                    object.entry("lastSkillAt").or_insert(json!(0));
                }
            }
        }
        state.insert("schema".into(), json!(20));
        value["schema"] = json!(20);
        schema = 20;
    }

    if schema < 21 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(object) = pet.as_object_mut() {
                    object.entry("lastActivityDay").or_insert(json!(""));
                }
            }
        }
        state.insert("schema".into(), json!(21));
        value["schema"] = json!(21);
        schema = 21;
    }

    if schema < 22 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(object) = pet.as_object_mut() {
                    object.entry("lastActivity").or_insert(Value::Null);
                    object.entry("activityStreak").or_insert(json!(0));
                }
            }
        }
        state.insert("schema".into(), json!(22));
        value["schema"] = json!(22);
        schema = 22;
    }

    if schema < 23 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state.entry("ranch").or_insert(json!({"pens": []}));
        state.insert("schema".into(), json!(23));
        value["schema"] = json!(23);
        schema = 23;
    }

    if schema < 24 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(ranch) = state.get_mut("ranch").and_then(Value::as_object_mut) {
            ranch.entry("workshop").or_insert(json!(false));
            ranch.entry("craft").or_insert(Value::Null);
        }
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(object) = pet.as_object_mut() {
                    object.entry("job").or_insert(json!({"level": 1, "exp": 0}));
                    object.entry("lastJobDay").or_insert(json!(""));
                }
            }
        }
        state.insert("schema".into(), json!(24));
        value["schema"] = json!(24);
        schema = 24;
    }

    if schema < 25 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let mut built = false;
        let mut jobs = vec![Value::Null, Value::Null, Value::Null];
        if let Some(ranch) = state.get_mut("ranch").and_then(Value::as_object_mut) {
            built = ranch
                .remove("workshop")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            if let Some(craft) = ranch.remove("craft").filter(Value::is_object) {
                let recipe = craft.get("recipe").cloned().unwrap_or(Value::Null);
                let crop_id = craft.get("cropId").cloned().unwrap_or(Value::Null);
                let started_at = craft.get("startedAt").cloned().unwrap_or(json!(0));
                let ready_at = craft.get("readyAt").cloned().unwrap_or(json!(0));
                let quality = craft.get("quality").cloned().unwrap_or(json!(1));
                if !recipe.is_null() {
                    jobs[0] = json!({
                        "kind": "ranch",
                        "recipe": recipe,
                        "startedAt": started_at,
                        "readyAt": ready_at,
                        "quality": quality
                    });
                } else if let Some(crop_id) = crop_id.as_str() {
                    jobs[0] = json!({
                        "kind": "cropBox",
                        "cropId": crop_id,
                        "startedAt": started_at,
                        "readyAt": ready_at
                    });
                }
            }
        }
        state.insert(
            "workshop".into(),
            json!({
                "built": built,
                "slots": 3,
                "equipment": ["manual", null, null],
                "jobs": jobs
            }),
        );
        state.insert("schema".into(), json!(25));
        value["schema"] = json!(25);
    }

    if schema < 26 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state.insert("fishing".into(), json!({}));
        state.insert("schema".into(), json!(26));
        value["schema"] = json!(26);
        schema = 26;
    }

    if schema < 27 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let inventory = state
            .get("inventory")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let fishing = state
            .entry("fishing")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or(SaveError::Unknown(schema))?;
        let batches = fishing
            .entry("fishBatches")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or(SaveError::Unknown(schema))?;
        if batches.is_empty() {
            for (key, value) in inventory {
                let Some(qty) = value.as_u64() else { continue };
                if qty == 0 {
                    continue;
                }
                let Some(id) = key.strip_prefix("fish:") else {
                    continue;
                };
                let Some(def) = crate::fishing::fish(id) else {
                    continue;
                };
                batches.insert(
                    id.to_string(),
                    json!({
                        "count": qty,
                        "totalWeightTenths": qty.saturating_mul(10),
                        "totalQuality": qty,
                        "totalValue": qty.saturating_mul(def.price),
                    }),
                );
            }
        }
        fishing.entry("lastCatch").or_insert(Value::Null);
        state.insert("schema".into(), json!(27));
        value["schema"] = json!(27);
        schema = 27;
    }

    if schema < 28 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let fishing = state
            .entry("fishing")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or(SaveError::Unknown(schema))?;
        fishing.entry("baitFine").or_insert(json!(0));
        fishing.entry("baitPrime").or_insert(json!(0));
        fishing.entry("baitMaster").or_insert(json!(0));
        fishing.entry("lastAutoReport").or_insert(Value::Null);
        fishing
            .entry("auto")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or(SaveError::Unknown(schema))?
            .entry("baitQuality")
            .or_insert(json!("normal"));
        state.insert("schema".into(), json!(28));
        value["schema"] = json!(28);
        schema = 28;
    }

    if schema < 29 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let fishing = state
            .entry("fishing")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or(SaveError::Unknown(schema))?;
        fishing.entry("activeBait").or_insert(json!("normal"));
        fishing.entry("rodWear").or_insert(json!({}));
        fishing.entry("lockedFish").or_insert(json!([]));
        state.insert("schema".into(), json!(29));
        value["schema"] = json!(29);
    }

    if schema < 30 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(daily) = state.get_mut("daily").and_then(Value::as_object_mut) {
            daily.entry("ranchCollects").or_insert(json!(0));
            daily.entry("fishCatches").or_insert(json!(0));
            daily.entry("claimedRanch").or_insert(json!(false));
            daily.entry("claimedFish").or_insert(json!(false));
        }
        state
            .entry("levelRewardsClaimed")
            .or_insert_with(|| json!([]));
        state.insert("schema".into(), json!(30));
        value["schema"] = json!(30);
    }

    if schema < 31 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let fishing = state
            .entry("fishing")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or(SaveError::Unknown(schema))?;
        fishing.entry("level").or_insert(json!(1));
        fishing.entry("exp").or_insert(json!(0));
        fishing.entry("skillPoints").or_insert(json!(0));
        fishing.entry("skills").or_insert(json!({}));
        state.insert("schema".into(), json!(31));
        value["schema"] = json!(31);
    }

    if schema < 32 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        let farm_level = state.get("level").and_then(Value::as_u64).unwrap_or(1) as u32;
        // 迁移前按农场等级已解锁的钓点，记成 legacy；新档这条集合是空的。
        let legacy: Vec<Value> = crate::fishing::FishingSpot::ALL
            .iter()
            .filter(|spot| farm_level >= spot.unlock_level())
            .map(|spot| Value::String(spot.key().into()))
            .collect();
        let mut moved = serde_json::Map::new();
        {
            let fishing = state.entry("fishing").or_insert_with(|| json!({}));
            if let Some(fishing) = fishing.as_object_mut() {
                for key in ["level", "exp", "skillPoints", "skills"] {
                    if let Some(value) = fishing.remove(key) {
                        moved.insert(key.into(), value);
                    }
                }
                fishing
                    .entry("legacyUnlocks")
                    .or_insert_with(|| Value::Array(legacy));
            }
        }
        let fishing_skill = json!({
            "level": moved.get("level").cloned().unwrap_or(json!(1)),
            "exp": moved.get("exp").cloned().unwrap_or(json!(0)),
            "points": moved.get("skillPoints").cloned().unwrap_or(json!(0)),
            "perks": moved.get("skills").cloned().unwrap_or(json!({})),
        });
        state.insert(
            "skills".into(),
            json!({
                "planting": { "level": 1, "exp": 0, "points": 0, "perks": {} },
                "ranching": { "level": 1, "exp": 0, "points": 0, "perks": {} },
                "fishing": fishing_skill,
                "processing": { "level": 1, "exp": 0, "points": 0, "perks": {} },
            }),
        );
        state.insert("schema".into(), json!(32));
        value["schema"] = json!(32);
    }

    if schema < 33 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(pet) = pet.as_object_mut() {
                    pet.entry("fishingJob")
                        .or_insert_with(|| json!({"level": 1, "exp": 0}));
                }
            }
        }
        state.insert("schema".into(), json!(33));
        value["schema"] = json!(33);
    }

    if schema < 34 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(pet) = pet.as_object_mut() {
                    pet.entry("talents").or_insert_with(|| {
                        json!({"tier1": null, "tier2": null, "tier3": null})
                    });
                }
            }
        }
        state.insert("schema".into(), json!(34));
        value["schema"] = json!(34);
    }

    if schema < 35 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(workshop) = state.get_mut("workshop").and_then(Value::as_object_mut) {
            let slots = workshop
                .get("slots")
                .and_then(Value::as_u64)
                .unwrap_or(3)
                .clamp(3, 5) as usize;
            workshop
                .entry("equipmentLevels")
                .or_insert_with(|| json!(vec![1u8; slots]));
        }
        state.insert("schema".into(), json!(35));
        value["schema"] = json!(35);
    }

    if schema < 36 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pens) = state
            .get_mut("ranch")
            .and_then(|ranch| ranch.get_mut("pens"))
            .and_then(Value::as_array_mut)
        {
            // 旧档的动物没有性格：按栏位与名字给一个固定的，别每次读档都变。
            const TRAITS: [&str; 5] = ["high_yield", "gentle", "picky", "delicate", "homebody"];
            for (pos, pen) in pens.iter_mut().enumerate() {
                let Some(animal) = pen.get_mut("animal").and_then(Value::as_object_mut) else {
                    continue;
                };
                let name = animal.get("name").and_then(Value::as_str).unwrap_or("");
                let pick = (pos + name.chars().count()) % TRAITS.len();
                animal
                    .entry("trait")
                    .or_insert_with(|| json!(TRAITS[pick]));
                animal.entry("affection").or_insert_with(|| json!(40));
                animal.entry("cleanliness").or_insert_with(|| json!(80));
                animal.entry("lastGroomDay").or_insert_with(|| json!(""));
                animal.entry("lastGrazeDay").or_insert_with(|| json!(""));
                animal.entry("lastPetDay").or_insert_with(|| json!(""));
            }
        }
        state.insert("schema".into(), json!(36));
        value["schema"] = json!(36);
    }

    if schema < 37 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pens) = state
            .get_mut("ranch")
            .and_then(|ranch| ranch.get_mut("pens"))
            .and_then(Value::as_array_mut)
        {
            for pen in pens {
                let Some(pen) = pen.as_object_mut() else {
                    continue;
                };
                // v0.89.0 的卫生挂在动物身上；v0.89.1 起改按畜栏。
                let animal_clean = pen
                    .get("animal")
                    .and_then(|animal| animal.get("cleanliness"))
                    .and_then(Value::as_u64)
                    .unwrap_or(80);
                pen.entry("cleanliness")
                    .or_insert_with(|| json!(animal_clean));
                pen.entry("lastCleanDay").or_insert_with(|| json!(""));
                if let Some(animal) = pen.get_mut("animal").and_then(Value::as_object_mut) {
                    // 陪伴语义的字段整批删掉，别留在存档里。
                    for key in [
                        "affection",
                        "cleanliness",
                        "lastGroomDay",
                        "lastGrazeDay",
                        "lastPetDay",
                    ] {
                        animal.remove(key);
                    }
                    // 旧性格名映射成生产特性。
                    if let Some(old) = animal.get("trait").and_then(Value::as_str) {
                        let mapped = match old {
                            "gentle" | "picky" => "efficient",
                            "homebody" => "twin_yield",
                            other => other,
                        };
                        animal.insert("trait".into(), json!(mapped));
                    }
                }
            }
        }
        state.insert("schema".into(), json!(37));
        value["schema"] = json!(37);
    }

    if schema < 38 {
        let saved_at = value.get("savedAt").and_then(Value::as_u64).unwrap_or(0);
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(ranch) = state.get_mut("ranch").and_then(Value::as_object_mut) {
            let mut line = 0u64;
            if let Some(pens) = ranch.get_mut("pens").and_then(Value::as_array_mut) {
                for pen in pens {
                    let Some(pen) = pen.as_object_mut() else {
                        continue;
                    };
                    let Some(animal) = pen.get_mut("animal").and_then(Value::as_object_mut) else {
                        continue;
                    };
                    // 老档每只动物各开一条血统线，避免一上来就全是近亲。
                    line += 1;
                    animal.entry("lineId").or_insert_with(|| json!(line));
                    let bred = animal
                        .remove("bred")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    animal
                        .entry("lastBreedAt")
                        .or_insert_with(|| json!(if bred { saved_at } else { 0 }));
                    animal.entry("adultAt").or_insert_with(|| json!(0));
                    animal.entry("inbred").or_insert_with(|| json!(false));
                    animal.entry("parents").or_insert_with(|| json!(""));
                }
            }
            ranch.entry("breedSeq").or_insert_with(|| json!(0));
            ranch.entry("lineSeq").or_insert_with(|| json!(line));
        }
        state.insert("schema".into(), json!(38));
        value["schema"] = json!(38);
    }

    if schema < 39 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state.entry("ranchOrders").or_insert_with(|| {
            json!({"day": "", "offers": [], "active": []})
        });
        if let Some(ranch) = state.get_mut("ranch").and_then(Value::as_object_mut) {
            ranch.entry("codexAnimals").or_insert_with(|| json!([]));
            ranch.entry("codexSpecials").or_insert_with(|| json!([]));
        }
        state.insert("schema".into(), json!(39));
        value["schema"] = json!(39);
    }

    if schema < 40 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state.entry("ranchContest").or_insert_with(|| {
            json!({
                "week": "",
                "entered": false,
                "result": null,
                "medals": 0,
                "bestRank": 0
            })
        });
        state.insert("schema".into(), json!(40));
        value["schema"] = json!(40);
    }

    if schema < 41 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state
            .entry("pond")
            .or_insert_with(|| json!({"ponds": []}));
        state.insert("schema".into(), json!(41));
        value["schema"] = json!(41);
    }

    if schema < 42 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pond) = state.get_mut("pond").and_then(Value::as_object_mut) {
            pond.entry("milestones").or_insert_with(|| json!([]));
            pond.entry("orderDay").or_insert_with(|| json!(""));
            pond.entry("orderDone").or_insert(json!(false));
            pond.entry("harvestedTotal").or_insert(json!(0));
            if let Some(ponds) = pond.get_mut("ponds").and_then(Value::as_array_mut) {
                for pond in ponds {
                    if let Some(pond) = pond.as_object_mut() {
                        pond.entry("autoFry").or_insert(json!(false));
                        pond.entry("lastFryAt").or_insert(json!(0));
                    }
                }
            }
        }
        state.insert("schema".into(), json!(42));
        value["schema"] = json!(42);
    }

    if schema < 43 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(npcs) = state.get_mut("npcs").and_then(Value::as_array_mut) {
            for npc in npcs {
                if let Some(npc) = npc.as_object_mut() {
                    npc.entry("seenEvents").or_insert_with(|| json!([]));
                    npc.entry("lastChatDay").or_insert_with(|| json!(""));
                }
            }
        }
        state.insert("schema".into(), json!(43));
        value["schema"] = json!(43);
    }

    if schema < 44 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(npcs) = state.get_mut("npcs").and_then(Value::as_array_mut) {
            for npc in npcs {
                if let Some(npc) = npc.as_object_mut() {
                    npc.entry("memories").or_insert_with(|| json!([]));
                }
            }
        }
        state.insert("schema".into(), json!(44));
        value["schema"] = json!(44);
    }

    if schema < 45 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(npcs) = state.get_mut("npcs").and_then(Value::as_array_mut) {
            for npc in npcs {
                if let Some(npc) = npc.as_object_mut() {
                    npc.entry("requestWeek").or_insert(json!(-1));
                    npc.entry("requestDone").or_insert(json!(false));
                    npc.entry("souvenirs").or_insert_with(|| json!([]));
                }
            }
        }
        state.insert("schema".into(), json!(45));
        value["schema"] = json!(45);
    }

    if schema < 46 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(npcs) = state.get_mut("npcs").and_then(Value::as_array_mut) {
            for npc in npcs {
                if let Some(npc) = npc.as_object_mut() {
                    npc.entry("pendingEvent").or_insert_with(|| json!(""));
                    npc.entry("choices").or_insert_with(|| json!({}));
                }
            }
        }
        state.insert("schema".into(), json!(46));
        value["schema"] = json!(46);
    }

    if schema < 47 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(npcs) = state.get_mut("npcs").and_then(Value::as_array_mut) {
            for npc in npcs {
                if let Some(npc) = npc.as_object_mut() {
                    npc.entry("birthdayGiftYear").or_insert(json!(0));
                }
            }
        }
        state.insert("schema".into(), json!(47));
        value["schema"] = json!(47);
    }

    if schema < 48 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state
            .entry("festivalClaims")
            .or_insert_with(|| json!({}));
        if let Some(npcs) = state.get_mut("npcs").and_then(Value::as_array_mut) {
            for npc in npcs {
                if let Some(npc) = npc.as_object_mut() {
                    npc.entry("lastGiftDay").or_insert_with(|| json!(""));
                }
            }
        }
        state.insert("schema".into(), json!(48));
        value["schema"] = json!(48);
    }

    if schema < 49 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state
            .entry("handbookClaimed")
            .or_insert_with(|| json!([]));
        state.insert("schema".into(), json!(49));
        value["schema"] = json!(49);
    }

    if schema < 50 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(npcs) = state.get_mut("npcs").and_then(Value::as_array_mut) {
            for npc in npcs {
                if let Some(npc) = npc.as_object_mut() {
                    npc.entry("lastHelpDay").or_insert_with(|| json!(""));
                }
            }
        }
        state.insert("schema".into(), json!(50));
        value["schema"] = json!(50);
    }

    if schema < 51 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state
            .entry("cropFreshDay")
            .or_insert_with(|| json!({}));
        state.insert("schema".into(), json!(51));
        value["schema"] = json!(51);
    }

    if schema < 52 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state.entry("cropQuality").or_insert_with(|| json!({}));
        state.insert("schema".into(), json!(52));
        value["schema"] = json!(52);
    }

    if schema < 53 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state
            .entry("festivalMarket")
            .or_insert_with(|| json!({}));
        state.insert("schema".into(), json!(53));
        value["schema"] = json!(53);
    }

    if schema < 54 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state
            .entry("festivalStories")
            .or_insert_with(|| json!({}));
        state
            .entry("festivalMemories")
            .or_insert_with(|| json!([]));
        state.insert("schema".into(), json!(54));
        value["schema"] = json!(54);
    }

    if schema < 55 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        state
            .entry("festivalYears")
            .or_insert_with(|| json!({}));
        state.insert("schema".into(), json!(55));
        value["schema"] = json!(55);
    }

    if schema < 56 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(market) = state.get_mut("festivalMarket").and_then(Value::as_object_mut) {
            market.entry("talked").or_insert_with(|| json!([]));
        }
        state.insert("schema".into(), json!(56));
        value["schema"] = json!(56);
    }

    if schema < 57 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(pet) = pet.as_object_mut() {
                    pet.entry("pondJob").or_insert_with(|| json!({"level": 1, "exp": 0}));
                }
            }
        }
        if let Some(codex) = state
            .get_mut("fishing")
            .and_then(Value::as_object_mut)
            .and_then(|fishing| fishing.get_mut("codex"))
            .and_then(Value::as_object_mut)
        {
            for entry in codex.values_mut() {
                let Some(entry) = entry.as_object_mut() else {
                    continue;
                };
                let count = entry.get("count").and_then(Value::as_u64).unwrap_or(0);
                if count == 0 || entry.get("sizeTiers").and_then(Value::as_u64).unwrap_or(0) != 0 {
                    continue;
                }
                let weight = entry
                    .get("bestWeightTenths")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32;
                let bit = crate::fishing::size_tier(weight).bit();
                entry.insert("sizeTiers".into(), json!(bit));
            }
        }
        state.insert("schema".into(), json!(57));
        value["schema"] = json!(57);
    }

    if schema < 58 {
        let state = value
            .get_mut("state")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Unknown(schema))?;
        if let Some(pets) = state.get_mut("pets").and_then(Value::as_array_mut) {
            for pet in pets {
                if let Some(pet) = pet.as_object_mut() {
                    pet.entry("milestones").or_insert_with(|| json!([]));
                    pet.entry("lastEventDay").or_insert_with(|| json!(""));
                    pet.entry("lastEvent").or_insert_with(|| json!(""));
                }
            }
        }
        state.insert("schema".into(), json!(58));
        value["schema"] = json!(58);
    }

    Ok(value)
}

impl Game {
    pub fn to_save(&self, saved_at: i64) -> SaveFile {
        SaveFile {
            schema: SAVE_SCHEMA,
            saved_at,
            state: self.state.clone(),
            content: self.content.clone(),
        }
    }

    pub fn from_save(mut save: SaveFile) -> Result<Self, SaveError> {
        if save.schema > SAVE_SCHEMA {
            return Err(SaveError::TooNew(save.schema));
        }
        if save.schema < SAVE_SCHEMA {
            return Err(SaveError::Unknown(save.schema));
        }
        save.content.migrate_balance();
        let issues = save.content.validate();
        if !issues.is_empty() {
            return Err(SaveError::InvalidContent(issues.join("；")));
        }
        save.state.schema = save.schema;
        save.state.room.ensure_defaults();
        Ok(Game::from_parts(save.state, save.content))
    }
}
