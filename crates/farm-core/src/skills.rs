//! 玩家侧技能：四专业共用一套等级 / 经验 / 点数 / 专精。
//!
//! 宠物有自己的等级与岗位熟练度，不走这里（玩家不是宠物）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 单个专业的等级上限。
pub const DISCIPLINE_MAX_LEVEL: u32 = 30;
/// 单个专精的等级上限。
pub const SKILL_MAX_LEVEL: u32 = 4;

/// 专业升级曲线：`need(L) = BASE + SLOPE × L`。
/// 这两个数是按 90 天模拟结果定的；改它们只影响专业，不动农场等级。
pub const PROFESSION_EXP_BASE: u64 = 100;
pub const PROFESSION_EXP_SLOPE: u64 = 100;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub enum DisciplineKind {
    Planting,
    Ranching,
    Fishing,
    Processing,
}

impl DisciplineKind {
    pub const ALL: [DisciplineKind; 4] = [
        DisciplineKind::Planting,
        DisciplineKind::Ranching,
        DisciplineKind::Fishing,
        DisciplineKind::Processing,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DisciplineKind::Planting => "种植",
            DisciplineKind::Ranching => "畜牧",
            DisciplineKind::Fishing => "钓鱼",
            DisciplineKind::Processing => "加工",
        }
    }

    /// 与存档 / 动作 JSON 一致的键名。
    pub fn key(self) -> &'static str {
        match self {
            DisciplineKind::Planting => "planting",
            DisciplineKind::Ranching => "ranching",
            DisciplineKind::Fishing => "fishing",
            DisciplineKind::Processing => "processing",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SkillDef {
    pub id: &'static str,
    pub discipline: DisciplineKind,
    pub name: &'static str,
    pub desc: &'static str,
    pub max_level: u32,
}

/// 种植的四个专精（v0.42.0）。
pub const PLANTING_SKILLS: [SkillDef; 4] = [
    SkillDef {
        id: "plant_yield",
        discipline: DisciplineKind::Planting,
        name: "精耕",
        desc: "收获数量 +4/8/12/16%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "plant_growth",
        discipline: DisciplineKind::Planting,
        name: "时令",
        desc: "生长时间 −3/6/9/12%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "plant_water",
        discipline: DisciplineKind::Planting,
        name: "保墒",
        desc: "浇水额外缩短 2/4/6/8%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "plant_seed",
        discipline: DisciplineKind::Planting,
        name: "留种",
        desc: "收获时 10/20/30/40% 返还 1 颗种子",
        max_level: SKILL_MAX_LEVEL,
    },
];

/// 钓鱼的四个专精（v0.39.0 的四个钓术，原样迁入）。
pub const FISHING_SKILLS: [SkillDef; 4] = [
    SkillDef {
        id: "bait_saver",
        discipline: DisciplineKind::Fishing,
        name: "省饵",
        desc: "主动抛竿有 8%/16%/24%/32% 概率不消耗鱼饵",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "reel",
        discipline: DisciplineKind::Fishing,
        name: "收线",
        desc: "收线进度 +2/+4/+6/+8% 每秒",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "big_fish",
        discipline: DisciplineKind::Fishing,
        name: "巨物",
        desc: "主动钓获重量 +0.1/+0.2/+0.3/+0.4kg",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "auto_speed",
        discipline: DisciplineKind::Fishing,
        name: "自动化",
        desc: "自动钓鱼间隔 −4%/8%/12%/16%",
        max_level: SKILL_MAX_LEVEL,
    },
];

/// 畜牧的四个专精（v0.43.0）。
pub const RANCHING_SKILLS: [SkillDef; 4] = [
    SkillDef {
        id: "ranch_quality",
        discipline: DisciplineKind::Ranching,
        name: "优育",
        desc: "畜产品品质 +1 的概率 10/20/30/40%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "ranch_yield",
        discipline: DisciplineKind::Ranching,
        name: "增产",
        desc: "收牧产额外 +1 的概率 8/16/24/32%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "ranch_feed",
        discipline: DisciplineKind::Ranching,
        name: "省料",
        desc: "喂食不消耗饲料的概率 8/16/24/32%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "ranch_care",
        discipline: DisciplineKind::Ranching,
        name: "照料",
        desc: "喂食/治疗的效果 +10/20/30/40%",
        max_level: SKILL_MAX_LEVEL,
    },
];

/// 加工的四个专精（v0.43.0）。
pub const PROCESSING_SKILLS: [SkillDef; 4] = [
    SkillDef {
        id: "craft_yield",
        discipline: DisciplineKind::Processing,
        name: "出品",
        desc: "取出成品时额外 +1 的概率 8/16/24/32%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "craft_quality",
        discipline: DisciplineKind::Processing,
        name: "精制",
        desc: "牧场加工品品质 +1 的概率 10/20/30/40%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "craft_speed",
        discipline: DisciplineKind::Processing,
        name: "熟练",
        desc: "加工工时 −4/8/12/16%",
        max_level: SKILL_MAX_LEVEL,
    },
    SkillDef {
        id: "craft_save",
        discipline: DisciplineKind::Processing,
        name: "省料",
        desc: "开工时 8/16/24/32% 概率返还 1 份主料",
        max_level: SKILL_MAX_LEVEL,
    },
];

/// 某个专业当前已开放的专精。
pub fn for_discipline(kind: DisciplineKind) -> &'static [SkillDef] {
    match kind {
        DisciplineKind::Planting => &PLANTING_SKILLS,
        DisciplineKind::Ranching => &RANCHING_SKILLS,
        DisciplineKind::Fishing => &FISHING_SKILLS,
        DisciplineKind::Processing => &PROCESSING_SKILLS,
    }
}

pub fn skill_def(kind: DisciplineKind, id: &str) -> Option<&'static SkillDef> {
    for_discipline(kind).iter().find(|def| def.id == id)
}

/// 从当前等级升到下一级需要的技能点。
pub fn skill_cost(current_level: u32) -> u32 {
    current_level.saturating_add(1)
}

/// 一个专业的成长数据。存档里就是这四项。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Discipline {
    pub level: u32,
    pub exp: u64,
    pub points: u32,
    pub perks: BTreeMap<String, u32>,
}

impl Default for Discipline {
    fn default() -> Self {
        Self {
            level: 1,
            exp: 0,
            points: 0,
            perks: BTreeMap::new(),
        }
    }
}

impl Discipline {
    pub fn new() -> Self {
        Self::default()
    }

    /// 本专业 `level` 升到下一级需要的经验；满级返回 0。
    ///
    /// 曲线暂时保持 0.39.0 的 `200 + 200×等级`，这样 v31→v32 迁移时
    /// 玩家已有的等级/经验逐项不变。要缩短曲线，先跑 90 天模拟再定。
    pub fn need(level: u32) -> u64 {
        if level >= DISCIPLINE_MAX_LEVEL {
            0
        } else {
            PROFESSION_EXP_BASE + u64::from(level) * PROFESSION_EXP_SLOPE
        }
    }

    /// 加经验；返回升了几级。满级后经验清零。
    pub fn add_exp(&mut self, amount: u64) -> u32 {
        if self.level >= DISCIPLINE_MAX_LEVEL {
            self.exp = 0;
            return 0;
        }
        self.exp = self.exp.saturating_add(amount);
        let mut gained = 0;
        while self.level < DISCIPLINE_MAX_LEVEL {
            let need = Self::need(self.level);
            if need == 0 || self.exp < need {
                break;
            }
            self.exp -= need;
            self.level += 1;
            self.points = self.points.saturating_add(1);
            gained += 1;
        }
        if self.level >= DISCIPLINE_MAX_LEVEL {
            self.exp = 0;
        }
        gained
    }

    pub fn skill_level(&self, id: &str) -> u32 {
        self.perks
            .get(id)
            .copied()
            .unwrap_or(0)
            .min(SKILL_MAX_LEVEL)
    }

    /// 下一个可加点时返回 `(当前等级, 花费)`；满级返回 None。
    pub fn next_upgrade(&self, def: &SkillDef) -> Option<(u32, u32)> {
        let current = self.skill_level(def.id);
        if current >= def.max_level {
            None
        } else {
            Some((current, skill_cost(current)))
        }
    }
}

/// 四专业。固定字段，排序稳定；以后加「采矿」是加一个带默认值的字段。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PlayerSkills {
    pub planting: Discipline,
    pub ranching: Discipline,
    pub fishing: Discipline,
    pub processing: Discipline,
}

impl PlayerSkills {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, kind: DisciplineKind) -> &Discipline {
        match kind {
            DisciplineKind::Planting => &self.planting,
            DisciplineKind::Ranching => &self.ranching,
            DisciplineKind::Fishing => &self.fishing,
            DisciplineKind::Processing => &self.processing,
        }
    }

    pub fn get_mut(&mut self, kind: DisciplineKind) -> &mut Discipline {
        match kind {
            DisciplineKind::Planting => &mut self.planting,
            DisciplineKind::Ranching => &mut self.ranching,
            DisciplineKind::Fishing => &mut self.fishing,
            DisciplineKind::Processing => &mut self.processing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disciplines_start_at_level_one() {
        let skills = PlayerSkills::new();
        for kind in DisciplineKind::ALL {
            let d = skills.get(kind);
            assert_eq!(d.level, 1);
            assert_eq!(d.exp, 0);
            assert_eq!(d.points, 0);
            assert!(d.perks.is_empty());
        }
    }

    #[test]
    fn leveling_grants_one_point_per_level() {
        let mut d = Discipline::new();
        // 100+100×1 = 200，100+100×2 = 300；500 经验正好升两级。
        assert_eq!(d.add_exp(500), 2);
        assert_eq!(d.level, 3);
        assert_eq!(d.exp, 0);
        assert_eq!(d.points, 2);
    }

    #[test]
    fn max_level_clears_exp_and_stops_points() {
        let mut d = Discipline {
            level: DISCIPLINE_MAX_LEVEL,
            exp: 0,
            points: 29,
            perks: BTreeMap::new(),
        };
        assert_eq!(d.add_exp(99_999), 0);
        assert_eq!(d.level, DISCIPLINE_MAX_LEVEL);
        assert_eq!(d.exp, 0);
        assert_eq!(d.points, 29);
    }

    #[test]
    fn skill_cost_and_cap() {
        assert_eq!(skill_cost(0), 1);
        assert_eq!(skill_cost(3), 4);
        let def = &FISHING_SKILLS[1]; // reel
        let mut d = Discipline::new();
        assert_eq!(d.next_upgrade(def), Some((0, 1)));
        d.perks.insert("reel".into(), 4);
        assert_eq!(d.next_upgrade(def), None);
        assert_eq!(d.skill_level("reel"), 4);
    }

    #[test]
    fn skill_tables_are_open_per_discipline() {
        for kind in DisciplineKind::ALL {
            assert_eq!(for_discipline(kind).len(), 4, "{kind:?}");
        }
        assert!(skill_def(DisciplineKind::Fishing, "reel").is_some());
        assert!(skill_def(DisciplineKind::Planting, "plant_yield").is_some());
        assert!(skill_def(DisciplineKind::Ranching, "ranch_quality").is_some());
        assert!(skill_def(DisciplineKind::Processing, "craft_yield").is_some());
        assert!(skill_def(DisciplineKind::Fishing, "nope").is_none());
    }
}
