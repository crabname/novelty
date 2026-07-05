//! Chess.com monthly archive export.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::Deserialize;

use crate::graph::{GameMeta, GameResult};

use super::{
    cancelled, http_client, no_games_message, LoadPeriod, LoadedGame, PlayerColor, StreamOutcome,
    TimeControlFilter,
};

#[derive(Deserialize)]
struct ChessArchives {
    archives: Vec<String>,
}

#[derive(Deserialize)]
struct ChessMonth {
    games: Vec<ChessComGame>,
}

#[derive(Deserialize)]
struct ChessComGame {
    pgn: String,
    #[serde(default)]
    white: ChessComPlayer,
    #[serde(default)]
    black: ChessComPlayer,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ChessComPlayer {
    #[serde(default)]
    username: String,
}

pub(crate) fn stream_chesscom(
    username: &str,
    color: PlayerColor,
    period: LoadPeriod,
    time_controls: TimeControlFilter,
    cancel: &Arc<AtomicBool>,
    on_game: &mut impl FnMut(LoadedGame) -> Result<(), String>,
) -> Result<(StreamOutcome, u32), String> {
    let user = username.trim().to_lowercase();
    let archives_url = format!("https://api.chess.com/pub/player/{user}/games/archives");
    let client = http_client();
    let archives: ChessArchives = client
        .get(&archives_url)
        .send()
        .map_err(|e| format!("Chess.com archives: {e}"))?
        .json()
        .map_err(|e| format!("Chess.com archives JSON: {e}"))?;
    if archives.archives.is_empty() {
        return Err(format!("No archives for {user} on Chess.com"));
    }

    let since_ym = period.since_year_month();
    let user_lower = user.as_str();
    let mut ingested = 0u32;
    for month_url in archives.archives.iter().rev() {
        if cancelled(cancel) {
            return Ok((StreamOutcome::Cancelled, ingested));
        }
        if !archive_in_period(month_url, since_ym) {
            continue;
        }
        let month: ChessMonth = client
            .get(month_url.as_str())
            .send()
            .map_err(|e| format!("Chess.com month: {e}"))?
            .json()
            .map_err(|e| format!("Chess.com month JSON: {e}"))?;
        for game in month.games.into_iter().rev() {
            if cancelled(cancel) {
                return Ok((StreamOutcome::Cancelled, ingested));
            }
            if !color_matches(&game, user_lower, color) {
                continue;
            }
            if game.pgn.contains("Variant \"") && !game.pgn.contains("Variant \"Standard\"") {
                continue;
            }
            if let Some(speed) = chesscom_speed(&game.pgn)
                && !time_controls.matches_speed(speed)
            {
                continue;
            }
            let _ = on_game(LoadedGame::Pgn {
                pgn: game.pgn.clone(),
                meta: chesscom_meta(&game.pgn),
            });
            ingested += 1;
        }
    }
    if ingested == 0 {
        return Err(no_games_message("Chess.com", &user, period));
    }
    Ok((StreamOutcome::Completed, ingested))
}

pub(crate) fn archive_in_period(url: &str, since: Option<(i32, u32)>) -> bool {
    let Some(since) = since else {
        return true;
    };
    let Some((year, month)) = parse_archive_year_month(url) else {
        return false;
    };
    year * 100 + month as i32 >= since.0 * 100 + since.1 as i32
}

fn parse_archive_year_month(url: &str) -> Option<(i32, u32)> {
    let mut parts = url.trim_end_matches('/').rsplit('/');
    let month: u32 = parts.next()?.parse().ok()?;
    let year: i32 = parts.next()?.parse().ok()?;
    Some((year, month))
}

fn color_matches(game: &ChessComGame, user: &str, color: PlayerColor) -> bool {
    match color {
        PlayerColor::White => game.white.username.eq_ignore_ascii_case(user),
        PlayerColor::Black => game.black.username.eq_ignore_ascii_case(user),
    }
}

fn pgn_header(pgn: &str, tag: &str) -> Option<String> {
    let needle = format!("[{tag} \"");
    for line in pgn.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&needle)
            && let Some(value) = rest.strip_suffix("\"]")
        {
            return Some(value.to_string());
        }
    }
    None
}

fn parse_elo(value: Option<String>) -> Option<u32> {
    value.and_then(|v| v.parse().ok())
}

fn chesscom_meta(pgn: &str) -> GameMeta {
    let result = match pgn_header(pgn, "Result").as_deref() {
        Some("1-0") => GameResult::WhiteWin,
        Some("0-1") => GameResult::BlackWin,
        _ => GameResult::Draw,
    };
    GameMeta {
        result,
        white_elo: parse_elo(pgn_header(pgn, "WhiteElo")),
        black_elo: parse_elo(pgn_header(pgn, "BlackElo")),
        date: pgn_header(pgn, "Date"),
        url: pgn_header(pgn, "Link"),
        timestamp: None,
    }
}

pub(crate) fn chesscom_speed(pgn: &str) -> Option<&'static str> {
    for line in pgn.lines() {
        let line = line.trim();
        if !line.starts_with("[Event ") {
            continue;
        }
        let event = line.to_ascii_lowercase();
        if event.contains("bullet") {
            return Some("bullet");
        }
        if event.contains("blitz") {
            return Some("blitz");
        }
        if event.contains("rapid") {
            return Some("rapid");
        }
        if event.contains("daily") || event.contains("classical") {
            return Some("classical");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_in_period_compares_year_month() {
        let since = Some((2025, 6));
        assert!(archive_in_period(
            "https://api.chess.com/pub/player/x/games/2026/03",
            since
        ));
        assert!(!archive_in_period(
            "https://api.chess.com/pub/player/x/games/2024/01",
            since
        ));
    }

    #[test]
    fn archive_in_period_accepts_all_when_no_since() {
        assert!(archive_in_period(
            "https://api.chess.com/pub/player/x/games/2010/01",
            None
        ));
    }

    #[test]
    fn chesscom_speed_from_event_tag() {
        assert_eq!(
            chesscom_speed("[Event \"Live Blitz\"]\n"),
            Some("blitz")
        );
        assert_eq!(
            chesscom_speed("[Event \"Live Rapid\"]\n"),
            Some("rapid")
        );
        assert_eq!(
            chesscom_speed("[Event \"Daily Chess\"]\n"),
            Some("classical")
        );
    }
}
