//! Lichess NDJSON game export with `until` pagination.

use std::io::{BufRead, BufReader};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::Deserialize;

use crate::graph::{GameMeta, GameResult, MoveNotation};

use super::{
    cancelled, http_client, no_games_message, LoadPeriod, LoadedGame, PlayerColor, StreamOutcome,
    TimeControlFilter,
};

/// Games per Lichess request (paginate with `until`).
pub const LICHESS_CHUNK_SIZE: u32 = 200;

#[derive(Deserialize)]
struct LichessGame {
    moves: String,
    #[serde(default)]
    speed: Option<String>,
    #[serde(rename = "lastMoveAt", default)]
    last_move_at: Option<i64>,
    #[serde(default)]
    winner: Option<String>,
    #[serde(default)]
    players: Option<LichessPlayers>,
    #[serde(default)]
    id: Option<String>,
}

#[derive(Deserialize)]
struct LichessPlayers {
    white: LichessPlayer,
    black: LichessPlayer,
}

#[derive(Deserialize, Default)]
struct LichessPlayer {
    #[serde(default)]
    rating: Option<u32>,
}

pub(crate) fn lichess_games_url(
    username: &str,
    color: PlayerColor,
    since: Option<i64>,
    until: Option<i64>,
    perf_types: Option<&str>,
) -> String {
    let user = urlencoding::encode(username.trim());
    let mut url = format!(
        "https://lichess.org/api/games/user/{user}?max={}&color={}&clocks=false&evals=false",
        LICHESS_CHUNK_SIZE,
        color.lichess_param()
    );
    if let Some(since) = since {
        url.push_str(&format!("&since={since}"));
    }
    if let Some(until) = until {
        url.push_str(&format!("&until={until}"));
    }
    if let Some(perf_types) = perf_types {
        url.push_str(&format!("&perfType={}", urlencoding::encode(perf_types)));
    }
    url
}

/// Oldest `lastMoveAt` in a full chunk becomes the next `until` cursor.
pub(crate) fn next_lichess_until(
    chunk_count: u32,
    oldest_last_move_at: Option<i64>,
    since: Option<i64>,
) -> Option<i64> {
    if chunk_count < LICHESS_CHUNK_SIZE {
        return None;
    }
    let oldest = oldest_last_move_at?;
    if let Some(since) = since
        && oldest <= since
    {
        return None;
    }
    Some(oldest - 1)
}

fn lichess_meta(game: &LichessGame) -> GameMeta {
    let result = match game.winner.as_deref() {
        Some("white") => GameResult::WhiteWin,
        Some("black") => GameResult::BlackWin,
        _ => GameResult::Draw,
    };
    let (white_elo, black_elo) = game
        .players
        .as_ref()
        .map(|p| (p.white.rating, p.black.rating))
        .unwrap_or((None, None));
    let url = game
        .id
        .as_ref()
        .map(|id| format!("https://lichess.org/{id}"));
    GameMeta {
        result,
        white_elo,
        black_elo,
        date: None,
        url,
        timestamp: game.last_move_at,
    }
}

fn stream_lichess_chunk(
    client: &reqwest::blocking::Client,
    url: &str,
    since: Option<i64>,
    time_controls: TimeControlFilter,
    cancel: &Arc<AtomicBool>,
    on_game: &mut impl FnMut(LoadedGame) -> Result<(), String>,
) -> Result<(StreamOutcome, u32, u32, Option<i64>), String> {
    let response = client
        .get(url)
        .header("Accept", "application/x-ndjson")
        .send()
        .map_err(|e| format!("Lichess request failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Lichess HTTP {} — check username",
            response.status()
        ));
    }

    let reader = BufReader::new(response);
    let mut ingested = 0u32;
    let mut games_in_response = 0u32;
    let mut oldest_ts: Option<i64> = None;
    for line in reader.lines() {
        if cancelled(cancel) {
            return Ok((
                StreamOutcome::Cancelled,
                ingested,
                games_in_response,
                oldest_ts,
            ));
        }
        let line = line.map_err(|e| format!("Lichess stream: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(game) = serde_json::from_str::<LichessGame>(&line) else {
            continue;
        };
        games_in_response += 1;
        if let Some(ts) = game.last_move_at {
            oldest_ts = Some(oldest_ts.map_or(ts, |o| o.min(ts)));
        }
        if game.moves.is_empty() {
            continue;
        }
        if let Some(speed) = game.speed.as_deref()
            && !time_controls.matches_speed(speed)
        {
            continue;
        }
        if let Some(since) = since
            && let Some(ts) = game.last_move_at
            && ts < since
        {
            continue;
        }
        let meta = lichess_meta(&game);
        let _ = on_game(LoadedGame::Moves {
            moves: game.moves,
            notation: MoveNotation::San,
            meta,
        });
        ingested += 1;
    }
    Ok((
        StreamOutcome::Completed,
        ingested,
        games_in_response,
        oldest_ts,
    ))
}

pub(crate) fn stream_lichess(
    username: &str,
    color: PlayerColor,
    period: LoadPeriod,
    time_controls: TimeControlFilter,
    cancel: &Arc<AtomicBool>,
    on_game: &mut impl FnMut(LoadedGame) -> Result<(), String>,
) -> Result<(StreamOutcome, u32), String> {
    let since = period.since_millis();
    let perf_types = time_controls.lichess_perf_types();
    let client = http_client();
    let mut ingested = 0u32;
    let mut until: Option<i64> = None;
    loop {
        if cancelled(cancel) {
            return Ok((StreamOutcome::Cancelled, ingested));
        }
        let url = lichess_games_url(
            username,
            color,
            since,
            until,
            perf_types.as_deref(),
        );
        let (outcome, chunk_ingested, games_in_response, oldest_ts) = stream_lichess_chunk(
            &client,
            &url,
            since,
            time_controls,
            cancel,
            on_game,
        )?;
        ingested += chunk_ingested;
        if outcome == StreamOutcome::Cancelled {
            return Ok((StreamOutcome::Cancelled, ingested));
        }
        until = next_lichess_until(games_in_response, oldest_ts, since);
        if until.is_none() {
            break;
        }
    }
    if ingested == 0 {
        return Err(no_games_message("Lichess", username, period));
    }
    Ok((StreamOutcome::Completed, ingested))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::PlayerColor;
    use crate::graph::{GameMeta, GameResult, OpeningGraph, start_fen};

    #[test]
    fn next_lichess_until_stops_on_partial_chunk() {
        assert_eq!(next_lichess_until(50, Some(1000), None), None);
    }

    #[test]
    fn lichess_ndjson_moves_are_san() {
        let sample = "d4 Nf6 c4 g6 Nc3 Bg7 e4 O-O";
        let mut graph = OpeningGraph::default();
        let meta = GameMeta {
            result: GameResult::WhiteWin,
            white_elo: Some(2800),
            black_elo: Some(2700),
            ..Default::default()
        };
        graph
            .add_game(sample, MoveNotation::San, Some(&meta), PlayerColor::White)
            .expect("lichess SAN movetext");
        assert_eq!(graph.game_count(), 1);
        let moves = graph.moves_at(&start_fen());
        assert!(moves.iter().any(|m| m.san == "d4"));
    }

    #[test]
    fn uci_notation_parses_lichess_style_uci() {
        let mut graph = OpeningGraph::default();
        graph
            .add_game("e2e4 e7e5", MoveNotation::Uci, None, PlayerColor::White)
            .expect("UCI movetext");
        let moves = graph.moves_at(&start_fen());
        assert!(moves.iter().any(|m| m.san == "e4"));
    }
}
