use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::decor::{self, DecorShopState};
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
    /// 鱼缸容量；0 表示不是鱼缸。
    pub capacity: u32,
    /// PNG 基名；多向时文件为 `<sprite>_00..03.png`，单向时 `<sprite>.png`。
    pub sprite: Option<&'static str>,
    pub sprite_rotations: u8,
}

pub const FURNITURE: [FurnitureDef; 16] = [
    FurnitureDef {
        id: "bed",
        name: "木床",
        width: 2,
        depth: 3,
        layer: 2,
        collision: true,
        kind: "bed",
        capacity: 0,
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
        capacity: 0,
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
        capacity: 0,
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
        capacity: 0,
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
        capacity: 0,
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
        capacity: 0,
        sprite: Some("prop_plant_green_01"),
        sprite_rotations: 1,
    },
    FurnitureDef {
        id: "flower_vase",
        name: "花瓶",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "flower",
        capacity: 0,
        sprite: Some("prop_flower_vase_01"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "planter",
        name: "落地花盆",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "planter",
        capacity: 0,
        sprite: Some("prop_planter_01"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "aquarium_small",
        name: "小鱼缸",
        width: 2,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "aquarium",
        capacity: 2,
        sprite: Some("prop_aquarium_small_01"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "aquarium_large",
        name: "大鱼缸",
        width: 2,
        depth: 2,
        layer: 2,
        collision: true,
        kind: "aquarium_large",
        capacity: 5,
        sprite: Some("prop_aquarium_large_01"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "sofa",
        name: "布艺沙发",
        width: 3,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "sofa",
        capacity: 0,
        sprite: Some("furn_sofa_green"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "bookshelf",
        name: "书架",
        width: 1,
        depth: 2,
        layer: 2,
        collision: true,
        kind: "bookshelf",
        capacity: 0,
        sprite: Some("furn_bookshelf_wood"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "floor_lamp",
        name: "落地灯",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "lamp",
        capacity: 0,
        sprite: Some("furn_lamp_floor"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "nightstand",
        name: "床头柜",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "nightstand",
        capacity: 0,
        sprite: Some("furn_nightstand_wood"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "tv",
        name: "液晶电视",
        width: 2,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "tv",
        capacity: 0,
        sprite: Some("furn_tv_wood"),
        sprite_rotations: 4,
    },
    FurnitureDef {
        id: "beanbag",
        name: "懒人沙发",
        width: 1,
        depth: 1,
        layer: 2,
        collision: true,
        kind: "beanbag",
        capacity: 0,
        sprite: Some("furn_beanbag_sage"),
        sprite_rotations: 4,
    },
];

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceDef {
    pub id: &'static str,
    pub name: &'static str,
    pub sprite: &'static str,
    pub sprite_rotations: u8,
    pub support_kinds: &'static [&'static str],
}

pub const SURFACE_ITEMS: [SurfaceDef; 1] = [SurfaceDef {
    id: "table_lamp",
    name: "台灯",
    sprite: "prop_table_lamp",
    sprite_rotations: 4,
    support_kinds: &["desk", "nightstand"],
}];

pub fn surface_item_def(id: &str) -> Option<&'static SurfaceDef> {
    SURFACE_ITEMS.iter().find(|def| def.id == id)
}

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
    /// 鱼缸里的鱼（fish id）；非鱼缸为空。
    #[serde(default)]
    pub fish: Vec<String>,
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
pub struct RoomSurfaceItem {
    pub id: String,
    pub def: String,
    pub support_id: String,
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
    #[serde(default)]
    pub surface_items: Vec<RoomSurfaceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoomState {
    #[serde(default)]
    pub rooms: BTreeMap<String, RoomLayout>,
    /// 已拥有家具（含已摆放）；商店/赠送/神秘商店都加到这里。
    #[serde(default)]
    pub owned_furniture: BTreeMap<String, u32>,
    #[serde(default)]
    pub owned_floors: BTreeSet<String>,
    #[serde(default)]
    pub owned_walls: BTreeSet<String>,
    /// 旧档第一次读入时按已摆放家具回填一次。
    #[serde(default)]
    pub ownership_initialized: bool,
    #[serde(default)]
    pub decor_shop: DecorShopState,
}

impl Default for RoomState {
    fn default() -> Self {
        let mut rooms = BTreeMap::new();
        rooms.insert(LIVING_ROOM.to_string(), default_layout());
        let mut state = Self {
            rooms,
            owned_furniture: BTreeMap::new(),
            owned_floors: BTreeSet::new(),
            owned_walls: BTreeSet::new(),
            ownership_initialized: false,
            decor_shop: DecorShopState::default(),
        };
        state.ensure_ownership();
        state
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
        self.ensure_ownership();
    }

    /// 首次读旧档：把当前已摆放的家具/地板/墙纸算成已拥有。
    pub fn ensure_ownership(&mut self) {
        if self.ownership_initialized {
            return;
        }
        let (furniture, floors, walls) = decor::seed_ownership(&self.rooms);
        self.owned_furniture = furniture;
        self.owned_floors = floors;
        self.owned_walls = walls;
        self.ownership_initialized = true;
    }

    /// 解锁里屋时，默认摆进去的家具也一并算已拥有。
    pub fn grant_layout(&mut self, layout: &RoomLayout) {
        self.ensure_ownership();
        for (id, qty) in decor::layout_furniture(layout) {
            *self.owned_furniture.entry(id).or_insert(0) += qty;
        }
        self.owned_floors.insert(layout.floor.clone());
        self.owned_walls.insert(layout.wall.clone());
    }

    /// 当前还能拿出来摆的家具数量（总拥有 - 所有房间已摆放）。
    pub fn available_furniture(&self) -> BTreeMap<String, u32> {
        let mut available = self.owned_furniture.clone();
        for layout in self.rooms.values() {
            for (id, qty) in decor::layout_furniture(layout) {
                let entry = available.entry(id).or_insert(0);
                *entry = entry.saturating_sub(qty);
            }
        }
        available
    }

    pub fn ensure_decor_shop(&mut self, day: &str) -> bool {
        self.decor_shop
            .ensure_day(day, &self.owned_floors, &self.owned_walls)
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
                fish: vec![],
            },
            RoomFurniture {
                id: "starter-bed".into(),
                def: "bed".into(),
                x: 88,
                y: 24,
                rot: 0,
                fish: vec![],
            },
            RoomFurniture {
                id: "starter-desk".into(),
                def: "desk".into(),
                x: 24,
                y: 16,
                rot: 0,
                fish: vec![],
            },
            RoomFurniture {
                id: "starter-chair".into(),
                def: "chair".into(),
                x: 40,
                y: 24,
                rot: 0,
                fish: vec![],
            },
            RoomFurniture {
                id: "starter-cabinet".into(),
                def: "cabinet".into(),
                x: 8,
                y: 40,
                rot: 0,
                fish: vec![],
            },
            RoomFurniture {
                id: "starter-plant".into(),
                def: "plant".into(),
                x: 112,
                y: 64,
                rot: 0,
                fish: vec![],
            },
        ],
        surface_items: vec![],
    }
}

pub fn floor_styles() -> [(&'static str, &'static str); 6] {
    [
        ("wood", "暖木"),
        ("tile", "浅砖"),
        ("stone", "青石"),
        ("terracotta", "陶土"),
        ("tatami", "榻榻米"),
        ("cafe", "咖啡木"),
    ]
}

pub fn default_extra_layout() -> RoomLayout {
    RoomLayout {
        floor: DEFAULT_FLOOR.into(),
        wall: DEFAULT_WALL.into(),
        door: default_door(EXTRA_ROOM),
        windows: vec![],
        furniture: vec![],
        surface_items: vec![],
    }
}

pub fn wall_styles() -> [(&'static str, &'static str); 5] {
    [
        ("cream", "米白"),
        ("sage", "鼠尾草"),
        ("sky", "雾蓝"),
        ("dusk", "暮色"),
        ("linen", "亚麻"),
    ]
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

/// 校验这次保存没有摆出“没买过的家具/样式”。
pub fn validate_ownership(
    room_id: &str,
    layout: &RoomLayout,
    room: &RoomState,
) -> Result<(), GameError> {
    if !room.owned_floors.contains(&layout.floor) {
        return Err(GameError::RoomLayoutInvalid("这个地板还没买".into()));
    }
    if !room.owned_walls.contains(&layout.wall) {
        return Err(GameError::RoomLayoutInvalid("这个墙纸还没买".into()));
    }
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for (id, current) in &room.rooms {
        let source = if id == room_id { layout } else { current };
        for (def, qty) in decor::layout_furniture(source) {
            *counts.entry(def).or_insert(0) += qty;
        }
    }
    if !room.rooms.contains_key(room_id) {
        for (def, qty) in decor::layout_furniture(layout) {
            *counts.entry(def).or_insert(0) += qty;
        }
    }
    for (def, qty) in counts {
        let owned = room.owned_furniture.get(&def).copied().unwrap_or(0);
        if qty > owned {
            let name = furniture_def(&def).map(|def| def.name).unwrap_or(&def);
            return Err(GameError::RoomLayoutInvalid(format!(
                "「{name}」还没买够（要 {qty}，只有 {owned}）"
            )));
        }
    }
    Ok(())
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
        if !item.fish.is_empty() {
            if def.capacity == 0 || item.fish.len() > def.capacity as usize {
                return Err(GameError::RoomLayoutInvalid("鱼缸装不下这么多鱼".into()));
            }
        }
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

    if layout.surface_items.len() > 8 {
        return Err(GameError::RoomLayoutInvalid("桌面摆件太多了".into()));
    }
    let mut surface_ids = std::collections::BTreeSet::new();
    let mut surface_supports = std::collections::BTreeSet::new();
    for item in &layout.surface_items {
        if item.id.trim().is_empty() || !surface_ids.insert(item.id.clone()) {
            return Err(GameError::RoomLayoutInvalid("摆件 id 重复或为空".into()));
        }
        let def = surface_item_def(&item.def)
            .ok_or_else(|| GameError::RoomLayoutInvalid("没有这个摆件".into()))?;
        let support = layout
            .furniture
            .iter()
            .find(|furniture| furniture.id == item.support_id)
            .ok_or_else(|| GameError::RoomLayoutInvalid("摆件没有支撑家具".into()))?;
        let support_def = furniture_def(&support.def)
            .ok_or_else(|| GameError::RoomLayoutInvalid("没有这个支撑家具".into()))?;
        if !def.support_kinds.contains(&support_def.kind) {
            return Err(GameError::RoomLayoutInvalid("这个摆件不能放在这件家具上".into()));
        }
        if !surface_supports.insert(item.support_id.clone()) {
            return Err(GameError::RoomLayoutInvalid("同一件家具上摆了太多东西".into()));
        }
    }
    Ok(())
}



