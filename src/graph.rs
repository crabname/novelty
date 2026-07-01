//! Opening repertoire graph.

use std::collections::HashMap;

use gpui_chessboard::draw::{DrawModifiers, DrawShape};
use gpui_chessboard::{Color, Dests, Key};
use shakmaty::fen::Fen;
use shakmaty::uci::UciMove;
use shakmaty::{Chess, File, Move, Position, Rank, Square};

use crate::fetch::PlayerColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GameResult {
    WhiteWin,
    BlackWin,
    #[default]
    Draw,
}

#[derive(Clone, Debug, Default)]
pub struct GameMeta {
    pub result: GameResult,
    pub white_elo: Option<u32>,
    pub black_elo: Option<u32>,
    pub date: Option<String>,
    pub url: Option<String>,
    pub timestamp: Option<i64>,
}

impl GameMeta {
    pub fn opponent_elo(&self, player_color: PlayerColor) -> Option<u32> {
        match player_color {
            PlayerColor::White => self.black_elo,
            PlayerColor::Black => self.white_elo,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LastGame {
    pub url: Option<String>,
    pub date: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct PositionDetails {
    pub white_wins: u32,
    pub black_wins: u32,
    pub draws: u32,
    pub total_opponent_elo: u64,
    pub opponent_elo_games: u32,
    pub last_game: Option<LastGame>,
}

impl PositionDetails {
    pub fn total(&self) -> u32 {
        self.white_wins + self.black_wins + self.draws
    }

    pub fn average_opponent_elo(&self) -> Option<u32> {
        if self.opponent_elo_games == 0 {
            None
        } else {
            Some((self.total_opponent_elo / self.opponent_elo_games as u64) as u32)
        }
    }

    pub fn merge_game(&mut self, meta: &GameMeta, player_color: PlayerColor) {
        match meta.result {
            GameResult::WhiteWin => self.white_wins += 1,
            GameResult::BlackWin => self.black_wins += 1,
            GameResult::Draw => self.draws += 1,
        }
        if let Some(elo) = meta.opponent_elo(player_color) {
            self.total_opponent_elo += elo as u64;
            self.opponent_elo_games += 1;
        }
        let replace_last = match (&self.last_game, meta.timestamp, meta.date.as_deref()) {
            (None, _, _) => true,
            (Some(prev), Some(new_ts), _) => new_ts > prev.timestamp.unwrap_or(0),
            (Some(prev), None, Some(date)) => prev.date.as_deref() < Some(date),
            _ => false,
        };
        if replace_last {
            self.last_game = Some(LastGame {
                url: meta.url.clone(),
                date: meta.date.clone(),
                timestamp: meta.timestamp,
            });
        }
    }

    pub fn white_pct(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            0.
        } else {
            self.white_wins as f32 * 100. / total as f32
        }
    }

    pub fn draw_pct(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            0.
        } else {
            self.draws as f32 * 100. / total as f32
        }
    }

    pub fn black_pct(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            0.
        } else {
            self.black_wins as f32 * 100. / total as f32
        }
    }
}

#[derive(Clone, Debug)]
pub struct MoveStat {
    pub san: String,
    pub orig: Key,
    pub dest: Key,
    pub count: u32,
    pub level: u8,
    pub target_fen: String,
    pub details: PositionDetails,
}

#[derive(Clone, Debug, Default)]
struct Node {
    edges: HashMap<String, EdgeAgg>,
    max_count: u32,
    details: PositionDetails,
}

#[derive(Clone, Debug)]
struct EdgeAgg {
    san: String,
    orig: Key,
    dest: Key,
    count: u32,
    target_fen: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveNotation {
    San,
    Uci,
}

#[derive(Clone, Debug, Default)]
pub struct OpeningGraph {
    nodes: HashMap<String, Node>,
    games: u32,
}

impl OpeningGraph {
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.games = 0;
    }

    pub fn game_count(&self) -> u32 {
        self.games
    }

    pub fn add_game_san(&mut self, san_moves: &str) -> Result<(), String> {
        self.add_game(san_moves, MoveNotation::San, None, PlayerColor::White)
    }

    pub fn add_game(
        &mut self,
        moves: &str,
        notation: MoveNotation,
        meta: Option<&GameMeta>,
        player_color: PlayerColor,
    ) -> Result<(), String> {
        let mut pos = Chess::default();
        for token in moves.split_whitespace() {
            if token.is_empty() {
                continue;
            }
            match notation {
                MoveNotation::San => self.play_san_token(&mut pos, token, meta, player_color)?,
                MoveNotation::Uci => self.play_uci_token(&mut pos, token, meta, player_color)?,
            }
        }
        self.games += 1;
        Ok(())
    }

    pub fn add_game_pgn(&mut self, pgn: &str, meta: &GameMeta, player_color: PlayerColor) -> Result<(), String> {
        let movetext = crate::pgn::movetext_from_pgn(pgn).ok_or("PGN has no movetext")?;
        let mut pos = Chess::default();
        for token in crate::pgn::tokenize_movetext(&movetext) {
            self.play_san_token(&mut pos, &token, Some(meta), player_color)?;
        }
        self.games += 1;
        Ok(())
    }

    fn play_san_token(
        &mut self,
        pos: &mut Chess,
        token: &str,
        meta: Option<&GameMeta>,
        player_color: PlayerColor,
    ) -> Result<(), String> {
        let san: shakmaty::san::San = token
            .parse()
            .map_err(|_| format!("invalid SAN: {token}"))?;
        let m = san
            .to_move(pos)
            .map_err(|_| format!("illegal SAN in position: {token}"))?;
        let source = position_fen(pos);
        let san_label = san.to_string();
        let (orig, dest) = move_keys(m);
        pos.play_unchecked(m);
        let target = position_fen(pos);
        self.record_move(&source, &target, &san_label, orig, dest, meta, player_color);
        Ok(())
    }

    fn play_uci_token(
        &mut self,
        pos: &mut Chess,
        token: &str,
        meta: Option<&GameMeta>,
        player_color: PlayerColor,
    ) -> Result<(), String> {
        let uci: UciMove = token
            .parse()
            .map_err(|_| format!("invalid UCI: {token}"))?;
        let m = uci
            .to_move(pos)
            .map_err(|_| format!("illegal UCI in position: {token}"))?;
        let source = position_fen(pos);
        let san_label = shakmaty::san::San::from_move(pos, m).to_string();
        let (orig, dest) = move_keys(m);
        pos.play_unchecked(m);
        let target = position_fen(pos);
        self.record_move(&source, &target, &san_label, orig, dest, meta, player_color);
        Ok(())
    }

    fn record_move(
        &mut self,
        source_fen: &str,
        target_fen: &str,
        san: &str,
        orig: Key,
        dest: Key,
        meta: Option<&GameMeta>,
        player_color: PlayerColor,
    ) {
        let source_key = simplified_fen(source_fen);
        let node = self.nodes.entry(source_key).or_default();
        let edge = node.edges.entry(san.to_string()).or_insert_with(|| EdgeAgg {
            san: san.to_string(),
            orig: orig.clone(),
            dest: dest.clone(),
            count: 0,
            target_fen: simplified_fen(target_fen),
        });
        edge.count += 1;
        edge.target_fen = simplified_fen(target_fen);
        node.max_count = node.max_count.max(edge.count);

        if let Some(meta) = meta {
            let target_key = simplified_fen(target_fen);
            let target = self.nodes.entry(target_key).or_default();
            target.details.merge_game(meta, player_color);
        }
    }

    pub fn moves_at(&self, fen: &str) -> Vec<MoveStat> {
        let key = simplified_fen(fen);
        let Some(node) = self.nodes.get(&key) else {
            return Vec::new();
        };
        let mut moves: Vec<MoveStat> = node
            .edges
            .values()
            .map(|edge| {
                let details = self
                    .nodes
                    .get(&edge.target_fen)
                    .map(|n| n.details.clone())
                    .unwrap_or_default();
                MoveStat {
                    san: edge.san.clone(),
                    orig: edge.orig.clone(),
                    dest: edge.dest.clone(),
                    count: edge.count,
                    level: level_for(edge.count, node.max_count),
                    target_fen: edge.target_fen.clone(),
                    details,
                }
            })
            .collect();
        moves.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.san.cmp(&b.san)));
        moves
    }

    pub fn auto_shapes(fen: &str, graph: &Self) -> Vec<DrawShape> {
        graph
            .moves_at(fen)
            .into_iter()
            .map(move_to_shape)
            .collect()
    }

    pub fn dests_for_moves(moves: &[MoveStat]) -> Dests {
        let mut dests = Dests::new();
        for mv in moves {
            dests
                .entry(mv.orig.clone())
                .or_default()
                .push(mv.dest.clone());
        }
        dests
    }
}

/// Side to move parsed from a simplified or full FEN.
pub fn turn_color(fen: &str) -> Color {
    match fen.split_whitespace().nth(1) {
        Some("b") => Color::Black,
        _ => Color::White,
    }
}

/// Frequency tier for arrow width.
pub fn level_for(count: u32, max: u32) -> u8 {
    if max == 0 || count as f32 / max as f32 > 0.8 {
        3
    } else if count as f32 / max as f32 > 0.3 {
        2
    } else {
        1
    }
}

pub fn move_to_shape(mv: MoveStat) -> DrawShape {
    let (line_width, opacity, color) = match mv.level {
        3 => (16.0, 1.0, "#15781B"),
        2 => (10.0, 0.72, "#4a9e54"),
        _ => (6.0, 0.42, "#7aa883"),
    };
    DrawShape {
        orig: mv.orig,
        dest: Some(mv.dest),
        brush: Some("g".into()),
        modifiers: Some(DrawModifiers {
            color: Some(color.into()),
            opacity: Some(opacity),
            line_width: Some(line_width),
            hilite: None,
        }),
        label: None,
        below: false,
    }
}

pub fn simplified_fen(fen: &str) -> String {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() >= 3 {
        format!("{} {} {}", parts[0], parts[1], parts[2])
    } else {
        fen.to_string()
    }
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

pub fn start_fen() -> String {
    simplified_fen(&position_fen(&Chess::default()))
}

pub fn chess_from_fen(fen: &str) -> Result<Chess, String> {
    let fen_str = fen.to_string();
    let fen: Fen = fen_str
        .parse()
        .map_err(|_| format!("invalid FEN: {fen_str}"))?;
    fen.into_position(shakmaty::CastlingMode::Standard)
        .map_err(|_| format!("illegal position: {fen_str}"))
}

pub fn legal_dests_at(fen: &str) -> Result<Dests, String> {
    let pos = chess_from_fen(fen)?;
    let mut dests = Dests::new();
    for m in pos.legal_moves() {
        let from = m.from().expect("chess move has origin");
        let to = m.to();
        dests
            .entry(square_to_key(from))
            .or_default()
            .push(square_to_key(to));
    }
    Ok(dests)
}

pub fn play_move_keys(fen: &str, orig: &Key, dest: &Key) -> Result<(String, String, Key, Key), String> {
    let pos = chess_from_fen(fen)?;
    let from = square_from_key(orig).ok_or("invalid origin square")?;
    let to = square_from_key(dest).ok_or("invalid destination square")?;
    let m = pos
        .legal_moves()
        .into_iter()
        .find(|m| m.from() == Some(from) && m.to() == to)
        .ok_or("illegal move")?;
    let san = shakmaty::san::San::from_move(&pos, m).to_string();
    let mut next = pos.clone();
    next.play_unchecked(m);
    let target_fen = simplified_fen(&position_fen(&next));
    let (orig_key, dest_key) = move_keys(m);
    Ok((target_fen, san, orig_key, dest_key))
}

pub fn square_from_key(key: &Key) -> Option<Square> {
    let s = key.as_str();
    if s.len() != 2 {
        return None;
    }
    let mut chars = s.chars();
    let file = File::from_char(chars.next()?)?;
    let rank = Rank::from_char(chars.next()?)?;
    Some(Square::from_coords(file, rank))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_for_thresholds() {
        assert_eq!(level_for(80, 80), 3);
        assert_eq!(level_for(50, 80), 2);
        assert_eq!(level_for(10, 80), 1);
    }

    #[test]
    fn dests_for_moves_groups_by_origin() {
        let moves = vec![
            MoveStat {
                san: "e4".into(),
                orig: Key::new("e2").unwrap(),
                dest: Key::new("e4").unwrap(),
                count: 1,
                level: 3,
                target_fen: String::new(),
                details: PositionDetails::default(),
            },
            MoveStat {
                san: "d4".into(),
                orig: Key::new("d2").unwrap(),
                dest: Key::new("d4").unwrap(),
                count: 1,
                level: 3,
                target_fen: String::new(),
                details: PositionDetails::default(),
            },
        ];
        let dests = OpeningGraph::dests_for_moves(&moves);
        assert_eq!(dests.len(), 2);
        assert_eq!(dests.get(&Key::new("e2").unwrap()).unwrap()[0].as_str(), "e4");
    }

    #[test]
    fn turn_color_reads_fen_side() {
        assert_eq!(
            turn_color("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq"),
            Color::White
        );
        assert_eq!(
            turn_color("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq"),
            Color::Black
        );
    }

    #[test]
    fn aggregates_lichess_san_line() {
        let mut graph = OpeningGraph::default();
        graph.add_game_san("e4 e5 Nf3").unwrap();
        graph.add_game_san("e4 e5").unwrap();
        let moves = graph.moves_at(&start_fen());
        let e4 = moves.iter().find(|m| m.san == "e4").unwrap();
        assert_eq!(e4.count, 2);
        assert_eq!(e4.level, 3);
        assert!(!moves.is_empty());
    }

    #[test]
    fn tracks_results_on_target_position() {
        let mut graph = OpeningGraph::default();
        let meta = GameMeta {
            result: GameResult::WhiteWin,
            white_elo: Some(1500),
            black_elo: Some(1400),
            ..Default::default()
        };
        graph
            .add_game("e4", MoveNotation::San, Some(&meta), PlayerColor::White)
            .unwrap();
        let moves = graph.moves_at(&start_fen());
        let e4 = moves.iter().find(|m| m.san == "e4").unwrap();
        assert_eq!(e4.details.white_wins, 1);
        assert_eq!(e4.details.average_opponent_elo(), Some(1400));
    }
}
