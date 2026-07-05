//! Download games from Lichess and Chess.com.

mod chesscom;
mod lichess;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use chrono::Datelike;

use gpui::SharedString;
use gpui_component::searchable_list::SearchableListItem;

use crate::graph::{GameMeta, MoveNotation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeControlFilter {
    pub bullet: bool,
    pub blitz: bool,
    pub rapid: bool,
    pub classical: bool,
}

impl TimeControlFilter {
    pub fn all_enabled() -> Self {
        Self {
            bullet: true,
            blitz: true,
            rapid: true,
            classical: true,
        }
    }

    pub fn any_selected(self) -> bool {
        self.bullet || self.blitz || self.rapid || self.classical
    }

    pub fn lichess_perf_types(self) -> Option<String> {
        if !self.any_selected() {
            return None;
        }
        let mut types = Vec::new();
        if self.bullet {
            types.push("bullet");
        }
        if self.blitz {
            types.push("blitz");
        }
        if self.rapid {
            types.push("rapid");
        }
        if self.classical {
            types.push("classical");
        }
        if types.len() == 4 {
            None
        } else {
            Some(types.join(","))
        }
    }

    pub fn matches_speed(self, speed: &str) -> bool {
        match speed {
            "bullet" => self.bullet,
            "blitz" => self.blitz,
            "rapid" => self.rapid,
            "classical" | "correspondence" => self.classical,
            _ => true,
        }
    }
}

impl Default for TimeControlFilter {
    fn default() -> Self {
        Self::all_enabled()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LoadPeriod {
    #[default]
    OneMonth,
    ThreeMonths,
    SixMonths,
    OneYear,
    TwoYears,
    All,
}

impl LoadPeriod {
    pub const ALL: [Self; 6] = [
        Self::OneMonth,
        Self::ThreeMonths,
        Self::SixMonths,
        Self::OneYear,
        Self::TwoYears,
        Self::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OneMonth => "1 mo",
            Self::ThreeMonths => "3 mo",
            Self::SixMonths => "6 mo",
            Self::OneYear => "1 yr",
            Self::TwoYears => "2 yr",
            Self::All => "all",
        }
    }

    pub fn loading_label(self) -> String {
        match self {
            Self::All => "all games".into(),
            other => format!("last {}", other.label()),
        }
    }

    fn months(self) -> u32 {
        match self {
            Self::OneMonth => 1,
            Self::ThreeMonths => 3,
            Self::SixMonths => 6,
            Self::OneYear => 12,
            Self::TwoYears => 24,
            Self::All => 0,
        }
    }

    pub fn since_millis(self) -> Option<i64> {
        if self == Self::All {
            return None;
        }
        let now = chrono::Utc::now();
        let since = now
            .checked_sub_months(chrono::Months::new(self.months()))
            .unwrap_or(now);
        Some(since.timestamp_millis())
    }

    pub(crate) fn since_year_month(self) -> Option<(i32, u32)> {
        if self == Self::All {
            return None;
        }
        let since = chrono::Utc::now()
            .checked_sub_months(chrono::Months::new(self.months()))
            .unwrap_or_else(chrono::Utc::now);
        Some((since.year(), since.month()))
    }
}

impl SearchableListItem for LoadPeriod {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.label())
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamOutcome {
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site {
    Lichess,
    ChessCom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerColor {
    White,
    Black,
}

impl PlayerColor {
    pub(crate) fn lichess_param(self) -> &'static str {
        self.orientation_value()
    }

    pub fn orientation_value(self) -> &'static str {
        match self {
            Self::White => "white",
            Self::Black => "black",
        }
    }

    pub fn from_orientation(value: &str) -> Self {
        if value.eq_ignore_ascii_case("black") {
            Self::Black
        } else {
            Self::White
        }
    }
}

#[derive(Clone, Debug)]
pub enum LoadedGame {
    Moves {
        moves: String,
        notation: MoveNotation,
        meta: GameMeta,
    },
    Pgn {
        pgn: String,
        meta: GameMeta,
    },
}

pub(crate) fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("novelty/0.1")
        .timeout(Duration::from_secs(300))
        .build()
        .expect("reqwest client")
}

pub(crate) fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn no_games_message(site: &str, user: &str, period: LoadPeriod) -> String {
    match period {
        LoadPeriod::All => format!("No games for {user} on {site}"),
        _ => format!(
            "No games for {user} on {site} in the last {}",
            period.label()
        ),
    }
}

/// Stream games in the given period; returns outcome and ingested count.
pub struct StreamGamesRequest<'a> {
    pub site: Site,
    pub username: &'a str,
    pub color: PlayerColor,
    pub period: LoadPeriod,
    pub time_controls: TimeControlFilter,
    pub lichess_token: Option<&'a str>,
    pub cancel: &'a Arc<AtomicBool>,
}

pub fn stream_games(
    request: StreamGamesRequest<'_>,
    mut on_game: impl FnMut(LoadedGame) -> Result<(), String>,
) -> Result<(StreamOutcome, u32), String> {
    let StreamGamesRequest {
        site,
        username,
        color,
        period,
        time_controls,
        lichess_token,
        cancel,
    } = request;
    match site {
        Site::Lichess => lichess::stream_lichess(
            username,
            color,
            period,
            time_controls,
            lichess_token,
            cancel,
            &mut on_game,
        ),
        Site::ChessCom => {
            chesscom::stream_chesscom(username, color, period, time_controls, cancel, &mut on_game)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_control_filter_matches_speed() {
        let filter = TimeControlFilter {
            bullet: false,
            blitz: true,
            rapid: false,
            classical: false,
        };
        assert!(!filter.matches_speed("bullet"));
        assert!(filter.matches_speed("blitz"));
    }
}
