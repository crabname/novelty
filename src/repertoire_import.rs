//! Extract continuation lines from player games and merge into a repertoire tree.

use std::collections::HashMap;

use shakmaty::{Move, Position, Square};

use crate::fetch::LoadedGame;
use crate::graph::{simplified_fen, start_fen};
use crate::move_tree::MoveTree;
use crate::pgn::{build_history, movetext_from_pgn, parse_headers};
use crate::session::HistoryStep;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationLine {
    pub sans: Vec<String>,
    pub game_count: u32,
    pub status: MergeStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeStatus {
    New,
    Exists,
    Partial { shared_plies: usize },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MergeReport {
    pub lines_added: u32,
    pub plies_added: u32,
    pub lines_skipped: u32,
}

#[derive(Clone, Debug)]
pub struct ExtractOptions {
    pub anchor_fen: String,
    pub depth: usize,
    pub max_lines: usize,
    pub min_games: u32,
}

pub fn extract_continuations(games: &[LoadedGame], options: &ExtractOptions) -> Vec<ContinuationLine> {
    let anchor = simplified_fen(&options.anchor_fen);
    let mut counts: HashMap<Vec<String>, u32> = HashMap::new();

    for game in games {
        let Some(history) = game_mainline_history(game) else {
            continue;
        };
        let Some(ply) = find_position_ply(&history, &anchor) else {
            continue;
        };
        let sans = continuation_sans(&history, ply, options.depth);
        if sans.is_empty() {
            continue;
        }
        *counts.entry(sans).or_insert(0) += 1;
    }

    let mut lines: Vec<ContinuationLine> = counts
        .into_iter()
        .filter(|(_, count)| *count >= options.min_games)
        .map(|(sans, game_count)| ContinuationLine {
            sans,
            game_count,
            status: MergeStatus::New,
        })
        .collect();

    lines.sort_by(|a, b| b.game_count.cmp(&a.game_count).then_with(|| a.sans.cmp(&b.sans)));
    lines.truncate(options.max_lines);
    lines
}

pub fn classify_lines(tree: &MoveTree, anchor_path: &[usize], lines: &mut [ContinuationLine]) {
    for line in lines.iter_mut() {
        line.status = classify_line(tree, anchor_path, &line.sans);
    }
}

pub fn merge_continuations(
    tree: &mut MoveTree,
    anchor_path: &[usize],
    lines: &[ContinuationLine],
) -> MergeReport {
    let mut report = MergeReport::default();
    for line in lines {
        match line.status {
            MergeStatus::Exists => {
                report.lines_skipped += 1;
            }
            MergeStatus::New | MergeStatus::Partial { .. } => {
                let added = tree.merge_line_from_path(anchor_path, &line.sans);
                if added > 0 {
                    report.lines_added += 1;
                    report.plies_added += added;
                } else {
                    report.lines_skipped += 1;
                }
            }
        }
    }
    report
}

fn classify_line(tree: &MoveTree, anchor_path: &[usize], sans: &[String]) -> MergeStatus {
    let mut path = anchor_path.to_vec();
    let mut shared = 0usize;
    for san in sans {
        let Some(node) = tree.node_at(&path) else {
            return if shared == 0 {
                MergeStatus::New
            } else {
                MergeStatus::Partial { shared_plies: shared }
            };
        };
        let Some(index) = node
            .children
            .iter()
            .position(|child| child.san.as_deref() == Some(san.as_str()))
        else {
            return if shared == 0 {
                MergeStatus::New
            } else {
                MergeStatus::Partial { shared_plies: shared }
            };
        };
        path.push(index);
        shared += 1;
    }
    if shared == sans.len() {
        MergeStatus::Exists
    } else {
        MergeStatus::Partial { shared_plies: shared }
    }
}

fn game_mainline_history(game: &LoadedGame) -> Option<Vec<HistoryStep>> {
    match game {
        LoadedGame::Pgn { pgn, .. } => history_from_pgn(pgn),
        LoadedGame::Moves { moves, notation, .. } => {
            let movetext = match notation {
                crate::graph::MoveNotation::San => moves.clone(),
                crate::graph::MoveNotation::Uci => return None,
            };
            build_history(&movetext).ok()
        }
    }
}

fn history_from_pgn(pgn: &str) -> Option<Vec<HistoryStep>> {
    let headers = parse_headers(pgn);
    let movetext = movetext_from_pgn(pgn)?;
    let start = headers.get("FEN").cloned().unwrap_or_else(start_fen);
    let mut history = vec![HistoryStep::start(simplified_fen(&start))];
    let mut pos = crate::graph::chess_from_fen(&start).ok()?;
    for token in crate::pgn::tokenize_movetext(&movetext) {
        let san: shakmaty::san::San = token.parse().ok()?;
        let m = san.to_move(&pos).ok()?;
        let san_label = san.to_string();
        let (orig, dest) = move_keys(m);
        pos.play_unchecked(m);
        let fen = simplified_fen(&position_fen(&pos));
        history.push(HistoryStep::after_move(fen, san_label, orig, dest));
    }
    Some(history)
}

fn find_position_ply(history: &[HistoryStep], target_fen: &str) -> Option<usize> {
    history
        .iter()
        .position(|step| simplified_fen(&step.fen) == target_fen)
}

fn continuation_sans(history: &[HistoryStep], ply: usize, depth: usize) -> Vec<String> {
    history
        .iter()
        .skip(ply + 1)
        .take(depth)
        .filter_map(|step| step.san.clone())
        .collect()
}

fn position_fen(pos: &shakmaty::Chess) -> String {
    shakmaty::fen::Fen::from_position(pos, shakmaty::EnPassantMode::Legal).to_string()
}

fn move_keys(m: Move) -> (gpui_chessboard::Key, gpui_chessboard::Key) {
    let from = m.from().expect("chess move has origin");
    let to = m.to();
    (square_to_key(from), square_to_key(to))
}

fn square_to_key(sq: Square) -> gpui_chessboard::Key {
    let file = sq.file().to_string();
    let rank = (sq.rank() as u8) + 1;
    gpui_chessboard::Key::new(&format!("{file}{rank}")).expect("valid square")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::LoadedGame;
    use crate::graph::{play_move_keys, GameMeta};
    use crate::move_tree::MoveTree;
    use gpui_chessboard::Key;

    fn sample_pgn(movetext: &str) -> LoadedGame {
        LoadedGame::Pgn {
            pgn: format!(
                "[Event \"Test\"]\n[White \"A\"]\n[Black \"B\"]\n\n{movetext}"
            ),
            meta: GameMeta::default(),
        }
    }

    #[test]
    fn extracts_lines_after_e4_from_e4_e5_games() {
        let games = vec![
            sample_pgn("1. e4 e5 2. Nf3 Nc6 *"),
            sample_pgn("1. e4 e5 2. Bc4 Nf6 *"),
        ];
        let anchor = {
            let (fen, ..) =
                play_move_keys(&start_fen(), &Key::new("e2").unwrap(), &Key::new("e4").unwrap())
                    .unwrap();
            fen
        };
        let lines = extract_continuations(
            &games,
            &ExtractOptions {
                anchor_fen: anchor,
                depth: 4,
                max_lines: 20,
                min_games: 1,
            },
        );
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().any(|line| line.sans == vec!["e5", "Nf3", "Nc6"]));
        assert!(lines.iter().any(|line| line.sans == vec!["e5", "Bc4", "Nf6"]));
    }

    #[test]
    fn merge_skips_existing_and_extends_partial() {
        let mut tree = MoveTree::new();
        tree.make_step(
            {
                let (fen, ..) = play_move_keys(
                    &start_fen(),
                    &Key::new("e2").unwrap(),
                    &Key::new("e4").unwrap(),
                )
                .unwrap();
                fen
            },
            "e4".into(),
            Key::new("e2").unwrap(),
            Key::new("e4").unwrap(),
        );
        tree.make_step(
            {
                let (fen, ..) = play_move_keys(
                    tree.current().fen.as_str(),
                    &Key::new("e7").unwrap(),
                    &Key::new("e5").unwrap(),
                )
                .unwrap();
                fen
            },
            "e5".into(),
            Key::new("e7").unwrap(),
            Key::new("e5").unwrap(),
        );

        let anchor_path = vec![0];
        let mut lines = vec![ContinuationLine {
            sans: vec!["e5".into(), "Nf3".into()],
            game_count: 2,
            status: MergeStatus::New,
        }];
        classify_lines(&tree, &anchor_path, &mut lines);
        assert_eq!(lines[0].status, MergeStatus::Partial { shared_plies: 1 });

        let report = merge_continuations(&mut tree, &anchor_path, &lines);
        assert_eq!(report.lines_added, 1);
        assert_eq!(report.plies_added, 1);
        assert_eq!(tree.mainline_steps().len(), 4);
    }
}

#[cfg(test)]
mod wgraif_integration {
    use super::*;
    use crate::fetch::{LoadPeriod, PlayerColor, Site, StreamGamesRequest, TimeControlFilter};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    #[ignore] // manual network test
    fn fetch_wgraif_games() {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut games = Vec::new();
        let result = crate::fetch::stream_games(
            StreamGamesRequest {
                site: Site::Lichess,
                username: "wgraif",
                color: PlayerColor::White,
                period: LoadPeriod::ThreeMonths,
                time_controls: TimeControlFilter::all_enabled(),
                lichess_token: None,
                cancel: &cancel,
            },
            |g| {
                games.push(g);
                Ok(())
            },
        );
        eprintln!("result={result:?} games={}", games.len());
        assert!(result.is_ok());
        assert!(!games.is_empty());

        let lines = extract_continuations(
            &games,
            &ExtractOptions {
                anchor_fen: crate::graph::start_fen(),
                depth: 6,
                max_lines: 50,
                min_games: 1,
            },
        );
        eprintln!("lines={}", lines.len());
        assert!(!lines.is_empty(), "expected continuations from start");
    }
}
