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
pub const EXTRA_ROOM: &str = "extra_1";
pub const EXTRA_WIDTH: u32 = 12;
pub const EXTRA_HEIGHT: u32 = 9;
pub const EXTRA_ROOM_COST: u64 = 20_000;
pub const DOOR_WIDTH: u32 = 16;
pub const DOOR_HEIGHT: u32 = 32;
pub const WALL_HEIGHT_TILES: u32 = 6;
pub const WALL_HEIGHT_UNITS: u32 = WALL_HEIGHT_TILES * ROOM_GRID;
pub const MAX_WINDOWS: usize = 5;

#[derive(Debug, Clone, Copy)]
pub struct FurnitureDef {
    pub id: &'static str,
    pub name: &'static str,
    pub width: u32,
    pub depth: u32,
    pub layer: u32,
    pub collision: bool,
    pub kind: &'static str,
    /// PNG 基名；多向时文件为 `<sprite>_00..03.png`，单向时 `<sprite>.png`。
    pub sprite: Option<&'static str>,
    pub sprite_rotations: u8,
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
        sprite: Some("furn_bed_wood"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "desk",
        name: "书桌",
        width: 2,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "desk",
        sprite: Some("furn_desk_wood"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "chair",
        name: "木椅",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "chair",
        sprite: Some("furn_chair_wood"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "rug",
        name: "四格地毯",
        width: 3,
        depth: 2,
        layer: 0,
        collision: false,
        kind: "rug",
        sprite: Some("furn_rug_sage_01"),
        sprite_rotations: 1,
    },
    FurnitureDef {
        id: "cabinet",
        name: "小柜子",
        width: 1,
        depth: 2,
        layer: 2,
        collision: true,
        kind: "cabinet",
        sprite: Some("furn_cabinet_wood"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "plant",
        name: "绿植",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "plant",
        sprite: Some("prop_plant_green_01"),
        sprite_rotations: 1,
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
pub struct RoomDoor {
    pub id: String,
    pub wall: String,
    /// 沿墙方向，1/8 格；门从地面起，不能上下移动。
    pub x: u32,
    pub w: u32,
    pub h: u32,
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoomWindow {
    pub id: String,
    /// left / right，只有两面可见墙。
    pub wall: String,
    /// 沿墙方向，1/8 格。
    pub x: u32,
    /// 离地高度，1/8 格。
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoomLayout {
    pub floor: String,
    pub wall: String,
    #[serde(default)]
    pub door: Option<RoomDoor>,
    #[serde(default)]
    pub windows: Vec<RoomWindow>,
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
    pub fn ensure_defaults(&mut self) {
        let living = self
            .rooms
            .entry(LIVING_ROOM.to_string())
            .or_insert_with(default_layout);
        if living.door.is_none() {
            living.door = default_door(LIVING_ROOM);
        }
        if let Some(extra) = self.rooms.get_mut(EXTRA_ROOM) {
            if extra.door.is_none() {
                extra.door = default_door(EXTRA_ROOM);
            }
        }
    }

    pub fn is_unlocked(&self, room: &str) -> bool {
        self.rooms.contains_key(room)
    }

    pub fn layout(&self, room: &str) -> Option<&RoomLayout> {
        self.rooms.get(room)
    }
}

pub fn room_size(room: &str) -> Option<(u32, u32)> {
    match room {
        LIVING_ROOM => Some((LIVING_WIDTH, LIVING_HEIGHT)),
        EXTRA_ROOM => Some((EXTRA_WIDTH, EXTRA_HEIGHT)),
        _ => None,
    }
}

pub fn default_door(room: &str) -> Option<RoomDoor> {
    match room {
        LIVING_ROOM => Some(RoomDoor {
            id: "door-living".into(),
            wall: "right".into(),
            x: 88,
            w: DOOR_WIDTH,
            h: DOOR_HEIGHT,
            style: "wood".into(),
        }),
        EXTRA_ROOM => Some(RoomDoor {
            id: "door-extra".into(),
            wall: "left".into(),
            x: 24,
            w: DOOR_WIDTH,
            h: DOOR_HEIGHT,
            style: "wood".into(),
        }),
        _ => None,
    }
}

pub fn default_layout() -> RoomLayout {
    RoomLayout {
        floor: DEFAULT_FLOOR.into(),
        wall: DEFAULT_WALL.into(),
        door: default_door(LIVING_ROOM),
        windows: vec![RoomWindow {
            id: "starter-window".into(),
            wall: "right".into(),
            x: 64,
            y: 24,
            w: 24,
            h: 20,
            style: "large".into(),
        }],
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

pub fn default_extra_layout() -> RoomLayout {
    RoomLayout {
        floor: DEFAULT_FLOOR.into(),
        wall: DEFAULT_WALL.into(),
        door: default_door(EXTRA_ROOM),
        windows: vec![],
        furniture: vec![],
    }
}

pub fn wall_styles() -> [(&'static str, &'static str); 3] {
    [("cream", "米白"), ("sage", "鼠尾草"), ("sky", "雾蓝")]
}

/// 窗户预设：id / 名称 / 宽 / 高（1/8 格；墙高 48）。
pub fn window_styles() -> [(&'static str, &'static str, u32, u32); 3] {
    [
        ("small", "小窗", 16, 16),
        ("large", "大窗", 24, 20),
        ("floor", "落地窗", 32, WALL_HEIGHT_UNITS),
    ]
}

pub fn window_style(id: &str) -> Option<(&'static str, &'static str, u32, u32)> {
    window_styles().into_iter().find(|(key, _, _, _)| *key == id)
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
    let (width, height) = room_size(room)
        .ok_or_else(|| GameError::RoomLayoutInvalid("这个房间还没开放".into()))?;
    if !style_valid(&floor_styles(), &layout.floor) {
        return Err(GameError::RoomLayoutInvalid("没有这个地板样式".into()));
    }
    if !style_valid(&wall_styles(), &layout.wall) {
        return Err(GameError::RoomLayoutInvalid("没有这个墙纸样式".into()));
    }
    if layout.furniture.len() > 48 {
        return Err(GameError::RoomLayoutInvalid("家具太多了".into()));
    }
    if layout.windows.len() > MAX_WINDOWS {
        return Err(GameError::RoomLayoutInvalid("窗户最多 5 扇".into()));
    }

    if let Some(door) = &layout.door {
        let wall_len = match door.wall.as_str() {
            "left" => height * ROOM_GRID,
            "right" => width * ROOM_GRID,
            _ => return Err(GameError::RoomLayoutInvalid("门只能放左/右墙".into())),
        };
        if door.id.trim().is_empty() || door.w < 8 || door.h < 8 {
            return Err(GameError::RoomLayoutInvalid("门尺寸不合法".into()));
        }
        if door.style != "wood" {
            return Err(GameError::RoomLayoutInvalid("没有这个门样式".into()));
        }
        if door.x + door.w > wall_len || door.h > WALL_HEIGHT_UNITS {
            return Err(GameError::RoomLayoutInvalid("门超出墙面范围".into()));
        }
    }

    let mut window_ids = std::collections::BTreeSet::new();
    let mut window_boxes: Vec<(u32, u32, u32, u32, String)> = Vec::new();
    for window in &layout.windows {
        if window.id.trim().is_empty() || !window_ids.insert(window.id.clone()) {
            return Err(GameError::RoomLayoutInvalid("窗户 id 重复或为空".into()));
        }
        let wall_len = match window.wall.as_str() {
            "left" => height * ROOM_GRID,
            "right" => width * ROOM_GRID,
            _ => return Err(GameError::RoomLayoutInvalid("窗户只能放左/右墙".into())),
        };
        if window_style(&window.style).is_none() {
            return Err(GameError::RoomLayoutInvalid("没有这个窗户样式".into()));
        }
        if window.w == 0 || window.h == 0 || window.w < 8 || window.h < 8 {
            return Err(GameError::RoomLayoutInvalid("窗户太小了".into()));
        }
        if window.x + window.w > wall_len || window.y + window.h > WALL_HEIGHT_UNITS {
            return Err(GameError::RoomLayoutInvalid("窗户超出墙面范围".into()));
        }
        for (x, y, w, h, wall) in &window_boxes {
            if wall == &window.wall
                && window.x < x + w
                && *x < window.x + window.w
                && window.y < y + h
                && *y < window.y + window.h
            {
                return Err(GameError::RoomLayoutInvalid("窗户和窗户重叠了".into()));
            }
        }
        window_boxes.push((
            window.x,
            window.y,
            window.w,
            window.h,
            window.wall.clone(),
        ));
    }

    if let Some(door) = &layout.door {
        for (x, y, w, h, wall) in &window_boxes {
            if wall == &door.wall
                && door.x < x + w
                && *x < door.x + door.w
                && 0 < y + h
                && *y < door.h
            {
                return Err(GameError::RoomLayoutInvalid("门和窗户重叠了".into()));
            }
        }
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
        if item.x + w > width * ROOM_GRID || item.y + d > height * ROOM_GRID {
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
