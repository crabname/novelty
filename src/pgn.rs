//! PGN parsing for game analysis.

use std::collections::HashMap;

use gpui_chessboard::Key;
use shakmaty::fen::Fen;
use shakmaty::{Chess, Move, Position, Square};

use crate::graph::simplified_fen;
use crate::session::HistoryStep;

#[derive(Clone, Debug)]
pub struct ParsedGame {
    pub label: String,
    pub headers: HashMap<String, String>,
    pub history: Vec<HistoryStep>,
}

pub fn parse_pgn(pgn: &str) -> Result<ParsedGame, String> {
    let headers = parse_headers(pgn);
    let movetext = movetext_from_pgn(pgn).ok_or("PGN has no movetext")?;
    let history = build_history(&movetext)?;
    let label = game_label(&headers);
    Ok(ParsedGame {
        label,
        headers,
        history,
    })
}

pub fn parse_headers(pgn: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in pgn.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            break;
        }
        let Some((tag, value)) = parse_header_line(line) else {
            continue;
        };
        headers.insert(tag, value);
    }
    headers
}

fn parse_header_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix('[')?.strip_suffix(']')?;
    let mut parts = line.splitn(2, ' ');
    let tag = parts.next()?.to_string();
    let value = parts.next()?.trim_matches('"').to_string();
    Some((tag, value))
}

fn game_label(headers: &HashMap<String, String>) -> String {
    match (headers.get("White"), headers.get("Black")) {
        (Some(white), Some(black)) => format!("{white} vs {black}"),
        (Some(white), None) => white.clone(),
        (None, Some(black)) => black.clone(),
        _ => "Game Analysis".to_string(),
    }
}

pub fn movetext_from_pgn(pgn: &str) -> Option<String> {
    let trimmed = pgn.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('[') {
        let movetext = pgn.split("\n\n").skip(1).collect::<Vec<_>>().join("\n\n");
        let movetext = movetext.trim();
        if movetext.is_empty() {
            None
        } else {
            Some(movetext.to_string())
        }
    } else {
        Some(trimmed.to_string())
    }
}

pub fn build_history(movetext: &str) -> Result<Vec<HistoryStep>, String> {
    let start_fen = simplified_fen(&position_fen(&Chess::default()));
    let mut history = vec![HistoryStep::start(start_fen.clone())];
    let mut pos = Chess::default();

    for token in tokenize_movetext(movetext) {
        let san: shakmaty::san::San = token
            .parse()
            .map_err(|_| format!("invalid SAN: {token}"))?;
        let m = san
            .to_move(&pos)
            .map_err(|_| format!("illegal move in position: {token}"))?;
        let san_label = san.to_string();
        let (orig, dest) = move_keys(m);
        pos.play_unchecked(m);
        let fen = simplified_fen(&position_fen(&pos));
        history.push(HistoryStep::after_move(fen, san_label, orig, dest));
    }

    Ok(history)
}

pub fn tokenize_movetext(text: &str) -> Vec<String> {
    let mut cleaned = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            while chars.next().is_some_and(|x| x != '}') {}
            continue;
        }
        cleaned.push(c);
    }
    let mut tokens = Vec::new();
    for token in cleaned.split_whitespace() {
        match token {
            "1-0" | "0-1" | "1/2-1/2" | "*" => break,
            _ => {}
        }
        if let Some((_, san)) = token.split_once('.') {
            if san.is_empty() {
                continue;
            }
            tokens.push(san.to_string());
        } else if token.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        } else {
            tokens.push(token.to_string());
        }
    }
    tokens
}

fn position_fen(pos: &Chess) -> String {
    Fen::from_position(pos, shakmaty::EnPassantMode::Legal).to_string()
}

fn move_keys(m: Move) -> (Key, Key) {
    let from = m.from().expect("chess move has origin");
    let to = m.to();
    (square_to_key(from), square_to_key(to))
}

fn square_to_key(sq: Square) -> Key {
    let file = sq.file().to_string();
    let rank = (sq.rank() as u8) + 1;
    Key::new(&format!("{file}{rank}")).expect("valid square")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_game() {
        let pgn = r#"[Event "Test"]
[White "Alice"]
[Black "Bob"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 1-0"#;
        let game = parse_pgn(pgn).unwrap();
        assert_eq!(game.label, "Alice vs Bob");
        assert_eq!(game.history.len(), 5);
        assert_eq!(game.history[1].san.as_deref(), Some("e4"));
    }

    #[test]
    fn parses_movetext_only() {
        let game = parse_pgn("1. e4 e5 2. Nf3").unwrap();
        assert_eq!(game.history.len(), 4);
        assert_eq!(game.label, "Game Analysis");
    }
}
