//! Lichess game export via [litchee] (NDJSON streaming with `until` pagination).

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use futures_util::StreamExt;
use litchee::api::gameplay::games::LichessGame;
use litchee::model::{LichessColor, LichessSpeed};

use crate::graph::{GameMeta, GameResult, MoveNotation};
use crate::lichess::{self, block_on};

use super::{
    cancelled, no_games_message, LoadPeriod, LoadedGame, PlayerColor, StreamOutcome,
    TimeControlFilter,
};

/// Games per Lichess request (paginate with `until`).
pub const LICHESS_CHUNK_SIZE: u32 = 200;

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

fn speed_name(speed: LichessSpeed) -> &'static str {
    match speed {
        LichessSpeed::UltraBullet => "ultraBullet",
        LichessSpeed::Bullet => "bullet",
        LichessSpeed::Blitz => "blitz",
        LichessSpeed::Rapid => "rapid",
        LichessSpeed::Classical => "classical",
        LichessSpeed::Correspondence => "correspondence",
        _ => "unknown",
    }
}

fn lichess_meta(game: &LichessGame) -> GameMeta {
    let result = match game.winner {
        Some(LichessColor::White) => GameResult::WhiteWin,
        Some(LichessColor::Black) => GameResult::BlackWin,
        None => GameResult::Draw,
    };
    let (white_elo, black_elo) = game
        .players
        .as_ref()
        .map(|players| (players.white.rating, players.black.rating))
        .unwrap_or((None, None));
    let url = Some(format!("https://lichess.org/{}", game.id));
    GameMeta {
        result,
        white_elo,
        black_elo,
        date: None,
        url,
        timestamp: game.last_move_at,
    }
}

struct LichessChunkParams<'a> {
    username: &'a str,
    color: PlayerColor,
    since: Option<i64>,
    until: Option<i64>,
    perf_types: Option<&'a str>,
    time_controls: TimeControlFilter,
    cancel: &'a Arc<AtomicBool>,
}

async fn stream_lichess_chunk(
    client: &litchee::LichessClient,
    params: &LichessChunkParams<'_>,
    on_game: &mut impl FnMut(LoadedGame) -> Result<(), String>,
) -> Result<(StreamOutcome, u32, u32, Option<i64>), String> {
    let LichessChunkParams {
        username,
        color,
        since,
        until,
        perf_types,
        time_controls,
        cancel,
    } = params;
    let mut request = client
        .games()
        .export_user(username)
        .max(LICHESS_CHUNK_SIZE)
        .color(color.lichess_param());
    if let Some(since) = *since {
        request = request.since(since);
    }
    if let Some(until) = *until {
        request = request.until(until);
    }
    if let Some(perf_types) = perf_types {
        request = request.perf_type(perf_types);
    }

    let mut stream = request
        .stream()
        .await
        .map_err(|err| format!("Lichess request failed: {err}"))?;

    let mut ingested = 0u32;
    let mut games_in_response = 0u32;
    let mut oldest_ts: Option<i64> = None;

    while let Some(item) = stream.next().await {
        if cancelled(cancel) {
            return Ok((
                StreamOutcome::Cancelled,
                ingested,
                games_in_response,
                oldest_ts,
            ));
        }
        let game = item.map_err(|err| format!("Lichess stream: {err}"))?;
        games_in_response += 1;
        if let Some(ts) = game.last_move_at {
            oldest_ts = Some(oldest_ts.map_or(ts, |current| current.min(ts)));
        }
        let Some(moves) = game.moves.clone() else {
            continue;
        };
        if moves.is_empty() {
            continue;
        }
        if let Some(speed) = game.speed
            && !time_controls.matches_speed(speed_name(speed))
        {
            continue;
        }
        if let Some(since) = *since
            && let Some(ts) = game.last_move_at
            && ts < since
        {
            continue;
        }
        let meta = lichess_meta(&game);
        let _ = on_game(LoadedGame::Moves {
            moves,
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

async fn stream_lichess_async(
    username: &str,
    color: PlayerColor,
    period: LoadPeriod,
    time_controls: TimeControlFilter,
    token: Option<&str>,
    cancel: &Arc<AtomicBool>,
    on_game: &mut impl FnMut(LoadedGame) -> Result<(), String>,
) -> Result<(StreamOutcome, u32), String> {
    let client = lichess::lichess_client(token)?;
    let since = period.since_millis();
    let perf_types = time_controls.lichess_perf_types();
    let mut ingested = 0u32;
    let mut until: Option<i64> = None;

    loop {
        if cancelled(cancel) {
            return Ok((StreamOutcome::Cancelled, ingested));
        }
        let chunk_params = LichessChunkParams {
            username,
            color,
            since,
            until,
            perf_types: perf_types.as_deref(),
            time_controls,
            cancel,
        };
        let (outcome, chunk_ingested, games_in_response, oldest_ts) = stream_lichess_chunk(
            &client,
            &chunk_params,
            on_game,
        )
        .await?;
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

pub(crate) fn stream_lichess(
    username: &str,
    color: PlayerColor,
    period: LoadPeriod,
    time_controls: TimeControlFilter,
    token: Option<&str>,
    cancel: &Arc<AtomicBool>,
    on_game: &mut impl FnMut(LoadedGame) -> Result<(), String>,
) -> Result<(StreamOutcome, u32), String> {
    block_on(stream_lichess_async(
        username,
        color,
        period,
        time_controls,
        token,
        cancel,
        on_game,
    ))
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
