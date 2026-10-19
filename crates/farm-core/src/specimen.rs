use crate::content::{ContentPack, CropDef};

#[derive(Debug, Clone, Copy)]
pub struct SpecimenRecipe {
    pub need_qty: u32,
    pub coins: u64,
}

pub fn recipe(def: &CropDef) -> SpecimenRecipe {
    SpecimenRecipe {
        need_qty: 6 + def.unlock_level / 3,
        coins: 500 + u64::from(def.unlock_level) * 300,
    }
}

fn seed(now: i64, crop_id: &str) -> u64 {
    let mut hash = 0x9E37_79B9_7F4A_7C15u64 ^ (now as u64);
    for byte in crop_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash ^ (hash >> 33)
}

/// 从比标本作物高 1–10 级的作物里随机掉 1–2 颗种子；没有更高等级时退回同类。
pub fn roll_reward<'a>(content: &'a ContentPack, def: &'a CropDef, now: i64) -> (&'a CropDef, u32) {
    let upper = (def.unlock_level + 10).min(30);
    let mut pool: Vec<&CropDef> = content
        .crops
        .iter()
        .filter(|crop| crop.unlock_level > def.unlock_level && crop.unlock_level <= upper)
        .collect();
    if pool.is_empty() {
        pool = content
            .crops
            .iter()
            .filter(|crop| crop.unlock_level >= def.unlock_level)
            .collect();
    }
    if pool.is_empty() {
        pool.push(def);
    }
    let roll = seed(now, &def.id);
    let chosen = pool[(roll as usize) % pool.len()];
    let qty = 1 + (roll >> 32) as u32 % 2;
    (chosen, qty)
}
