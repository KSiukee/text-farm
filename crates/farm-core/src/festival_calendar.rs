//! 节日日历：今年 7 个节、下一个节、年度小结。

use serde::Serialize;

use crate::model::GameState;
use crate::npc_story;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FestivalCalendarView {
    pub year: i32,
    pub today: String,
    pub next: Option<FestivalDayView>,
    pub festivals: Vec<FestivalDayView>,
    pub attended: u32,
    pub market_visits: u32,
    pub memories: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FestivalDayView {
    pub name: String,
    pub date: String,
    pub passed: bool,
    pub is_today: bool,
    pub days_until: Option<i64>,
    pub claimed: bool,
    pub story_done: bool,
    pub market_seen: bool,
}

pub fn view(state: &GameState) -> FestivalCalendarView {
    let today = state.daily.day.clone();
    let year = npc_story::year_of(&today);
    let mut festivals = Vec::new();
    for (name, date) in npc_story::festival_dates_for_year(year) {
        festivals.push(day_view(state, name, &date, year, &today));
    }
    let next = festivals
        .iter()
        .find(|festival| !festival.passed)
        .cloned()
        .or_else(|| {
            npc_story::festival_dates_for_year(year + 1)
                .into_iter()
                .next()
                .map(|(name, date)| day_view(state, name, &date, year + 1, &today))
        });
    let attended = festivals.iter().filter(|festival| festival.claimed).count() as u32;
    let market_visits = state
        .festival_years
        .get(&year)
        .map(|stat| stat.market_festivals.len() as u32)
        .unwrap_or(0);
    let memories = state
        .festival_memories
        .iter()
        .filter(|memory| memory.year == year)
        .count() as u32;
    let summary = if attended == 0 && market_visits == 0 && memories == 0 {
        match &next {
            Some(next) => format!(
                "今年还没过节。下一个是{}，还有 {} 天。",
                next.name,
                next.days_until.unwrap_or(0)
            ),
            None => "今年还没有节日记录。".to_string(),
        }
    } else {
        format!(
            "今年过了 {attended} 个节，赶了 {market_visits} 次集，留下 {memories} 段回忆。"
        )
    };
    FestivalCalendarView {
        year,
        today,
        next,
        festivals,
        attended,
        market_visits,
        memories,
        summary,
    }
}

fn day_view(
    state: &GameState,
    name: &'static str,
    date: &str,
    year: i32,
    today: &str,
) -> FestivalDayView {
    let passed = date < today;
    let is_today = date == today;
    let days_until = if passed {
        None
    } else {
        Some(crate::orders::days_between(today, date))
    };
    let story_key = crate::festival_story::story_key(year, name);
    FestivalDayView {
        name: name.to_string(),
        date: date.to_string(),
        passed,
        is_today,
        days_until,
        claimed: state.festival_claims.get(name) == Some(&year),
        story_done: state.festival_stories.contains_key(&story_key),
        market_seen: state
            .festival_years
            .get(&year)
            .map(|stat| stat.market_festivals.contains(name))
            .unwrap_or(false),
    }
}
