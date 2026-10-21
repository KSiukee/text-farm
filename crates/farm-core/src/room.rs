use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::GameError;

/// 房间坐标用 1/8 格；16×12 的客厅就是 128×96 个位置。
pub const ROOM_GRID: u32 = 8;
pub const LIVING_ROOM: &str = "living";
pub const LIVING_WIDTH: u32 = 16;
pub const LIVING_HEIGHT: u32 = 12;
pub const DEFAULT_FLOOR: &str = "wood";
pub const DEFAULT_WALL: &str = "cream";

#[derive(Debug, Clone, Copy)]
pub struct FurnitureDef {
    pub id: &'static str,
    pub name: &'static str,
    pub width: u32,
    pub depth: u32,
    pub layer: u32,
    pub collision: bool,
    pub kind: &'static str,
}

pub const FURNITURE: [FurnitureDef; 6] = [
    FurnitureDef {
        id: "bed",
        name: "木床",
        width: 2,
        depth: 3,
        layer: 2,
        collision: true,
        kind: "bed",
    },
    FurnitureDef {
        id: "desk",
        name: "书桌",
        width: 2,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "desk",
    },
    FurnitureDef {
        id: "chair",
        name: "木椅",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "chair",
    },
    FurnitureDef {
        id: "rug",
        name: "四格地毯",
        width: 3,
        depth: 2,
        layer: 0,
        collision: false,
        kind: "rug",
    },
    FurnitureDef {
        id: "cabinet",
        name: "小柜子",
        width: 1,
        depth: 2,
        layer: 2,
        collision: true,
        kind: "cabinet",
    },
    FurnitureDef {
        id: "plant",
        name: "绿植",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "plant",
    },
];

pub fn furniture_def(id: &str) -> Option<&'static FurnitureDef> {
    FURNITURE.iter().find(|def| def.id == id)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoomFurniture {
    pub id: String,
    pub def: String,
    /// 1/8 格坐标。
    pub x: u32,
    pub y: u32,
    /// 0/1/2/3，90° 一档。
    pub rot: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoomLayout {
    pub floor: String,
    pub wall: String,
    pub furniture: Vec<RoomFurniture>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoomState {
    #[serde(default)]
    pub rooms: BTreeMap<String, RoomLayout>,
}

impl Default for RoomState {
    fn default() -> Self {
        let mut rooms = BTreeMap::new();
        rooms.insert(LIVING_ROOM.to_string(), default_layout());
        Self { rooms }
    }
}

impl RoomState {
    pub fn layout(&self, room: &str) -> Option<&RoomLayout> {
        self.rooms.get(room)
    }
}

pub fn default_layout() -> RoomLayout {
    RoomLayout {
        floor: DEFAULT_FLOOR.into(),
        wall: DEFAULT_WALL.into(),
        furniture: vec![
            RoomFurniture {
                id: "starter-rug".into(),
                def: "rug".into(),
                x: 56,
                y: 52,
                rot: 0,
            },
            RoomFurniture {
                id: "starter-bed".into(),
                def: "bed".into(),
                x: 88,
                y: 24,
                rot: 0,
            },
            RoomFurniture {
                id: "starter-desk".into(),
                def: "desk".into(),
                x: 24,
                y: 16,
                rot: 0,
            },
            RoomFurniture {
                id: "starter-chair".into(),
                def: "chair".into(),
                x: 40,
                y: 24,
                rot: 0,
            },
            RoomFurniture {
                id: "starter-cabinet".into(),
                def: "cabinet".into(),
                x: 8,
                y: 40,
                rot: 0,
            },
            RoomFurniture {
                id: "starter-plant".into(),
                def: "plant".into(),
                x: 112,
                y: 64,
                rot: 0,
            },
        ],
    }
}

pub fn floor_styles() -> [(&'static str, &'static str); 3] {
    [("wood", "暖木"), ("tile", "浅砖"), ("stone", "青石")]
}

pub fn wall_styles() -> [(&'static str, &'static str); 3] {
    [("cream", "米白"), ("sage", "鼠尾草"), ("sky", "雾蓝")]
}

fn style_valid(styles: &[(&str, &str)], id: &str) -> bool {
    styles.iter().any(|(key, _)| *key == id)
}

fn footprint(def: &FurnitureDef, rot: u32) -> (u32, u32) {
    let (w, d) = if rot % 2 == 1 {
        (def.depth, def.width)
    } else {
        (def.width, def.depth)
    };
    (w * ROOM_GRID, d * ROOM_GRID)
}

fn overlaps(a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// 校验装修草稿：房间、样式、边界、重复 id、家具碰撞。
pub fn validate_layout(room: &str, layout: &RoomLayout) -> Result<(), GameError> {
    if room != LIVING_ROOM {
        return Err(GameError::RoomLayoutInvalid("这个房间还没开放".into()));
    }
    if !style_valid(&floor_styles(), &layout.floor) {
        return Err(GameError::RoomLayoutInvalid("没有这个地板样式".into()));
    }
    if !style_valid(&wall_styles(), &layout.wall) {
        return Err(GameError::RoomLayoutInvalid("没有这个墙纸样式".into()));
    }
    if layout.furniture.len() > 48 {
        return Err(GameError::RoomLayoutInvalid("家具太多了".into()));
    }

    let mut ids = std::collections::BTreeSet::new();
    let mut boxes: Vec<(String, (u32, u32, u32, u32), bool)> = Vec::new();
    for item in &layout.furniture {
        if item.id.trim().is_empty() || !ids.insert(item.id.clone()) {
            return Err(GameError::RoomLayoutInvalid("家具 id 重复或为空".into()));
        }
        if item.rot > 3 {
            return Err(GameError::RoomLayoutInvalid("家具朝向不合法".into()));
        }
        let def = furniture_def(&item.def)
            .ok_or_else(|| GameError::RoomLayoutInvalid("没有这件家具".into()))?;
        let (w, d) = footprint(def, item.rot);
        if item.x + w > LIVING_WIDTH * ROOM_GRID || item.y + d > LIVING_HEIGHT * ROOM_GRID {
            return Err(GameError::RoomLayoutInvalid("家具超出房间范围".into()));
        }
        if def.collision {
            let rect = (item.x, item.y, w, d);
            for (_, other, other_collision) in &boxes {
                if *other_collision && overlaps(rect, *other) {
                    return Err(GameError::RoomLayoutInvalid("家具和家具重叠了".into()));
                }
            }
            boxes.push((item.id.clone(), rect, true));
        } else {
            boxes.push((item.id.clone(), (item.x, item.y, w, d), false));
        }
    }
    Ok(())
}
