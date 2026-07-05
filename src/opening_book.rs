//! Local ECO opening book: TSV → played positions → `Setup` lookup.

use std::sync::OnceLock;

use shakmaty::fen::Fen;
use shakmaty::san::San;
use shakmaty::{Chess, EnPassantMode, Position, Setup};

use crate::graph::chess_from_fen;

const TSV_FILES: &[&[u8]] = &[
    include_bytes!("../data/openings/a.tsv"),
    include_bytes!("../data/openings/b.tsv"),
    include_bytes!("../data/openings/c.tsv"),
    include_bytes!("../data/openings/d.tsv"),
    include_bytes!("../data/openings/e.tsv"),
];

const FRC_TSV: &[u8] = include_bytes!("../data/openings/frc.tsv");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningMatch {
    pub eco: String,
    pub name: String,
}

struct OpeningEntry {
    eco: String,
    name: String,
    setup: Setup,
}

static OPENINGS: OnceLock<Vec<OpeningEntry>> = OnceLock::new();

fn openings() -> &'static [OpeningEntry] {
    OPENINGS.get_or_init(load_openings)
}

fn load_openings() -> Vec<OpeningEntry> {
    let mut entries = Vec::with_capacity(4_800);

    entries.push(OpeningEntry {
        eco: "—".into(),
        name: "Starting Position".into(),
        setup: normalize_setup(Setup::default()),
    });

    for tsv in TSV_FILES {
        load_eco_tsv(tsv, &mut entries);
    }
    load_frc_tsv(FRC_TSV, &mut entries);

    entries
}

fn load_eco_tsv(bytes: &[u8], entries: &mut Vec<OpeningEntry>) {
    let text = std::str::from_utf8(bytes).expect("opening TSV utf-8");
    for line in text.lines().skip(1) {
        let Some((eco, name, pgn)) = parse_eco_line(line) else {
            continue;
        };
        let Some(setup) = setup_after_pgn(pgn) else {
            continue;
        };
        entries.push(OpeningEntry {
            eco: eco.into(),
            name: name.into(),
            setup: normalize_setup(setup),
        });
    }
}

fn load_frc_tsv(bytes: &[u8], entries: &mut Vec<OpeningEntry>) {
    let text = std::str::from_utf8(bytes).expect("FRC TSV utf-8");
    for line in text.lines().skip(1) {
        let mut parts = line.split('\t');
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(fen) = parts.next() else {
            continue;
        };
        let Ok(fen) = fen.parse::<Fen>() else {
            continue;
        };
        entries.push(OpeningEntry {
            eco: "FRC".into(),
            name: name.into(),
            setup: normalize_setup(fen.into_setup()),
        });
    }
}

fn parse_eco_line(line: &str) -> Option<(&str, &str, &str)> {
    let mut parts = line.splitn(3, '\t');
    let eco = parts.next()?.trim();
    let name = parts.next()?.trim();
    let pgn = parts.next()?.trim();
    if eco.is_empty() || name.is_empty() || pgn.is_empty() {
        return None;
    }
    Some((eco, name, pgn))
}

fn setup_after_pgn(pgn: &str) -> Option<Setup> {
    let mut pos = Chess::default();
    for token in pgn.split_whitespace() {
        if token.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        let san: San = token.parse().ok()?;
        let mv = san.to_move(&pos).ok()?;
        pos.play_unchecked(mv);
    }
    Some(normalize_setup(pos.to_setup(EnPassantMode::Legal)))
}

fn normalize_setup(mut setup: Setup) -> Setup {
    setup.halfmoves = 0;
    setup.fullmoves = std::num::NonZero::new(1).expect("valid fullmoves");
    setup
}

fn setup_from_fen(fen: &str) -> Option<Setup> {
    chess_from_fen(fen)
        .ok()
        .map(|pos| normalize_setup(pos.to_setup(EnPassantMode::Legal)))
}

pub fn lookup_setup(setup: &Setup) -> Option<OpeningMatch> {
    let setup = normalize_setup(setup.clone());
    openings()
        .iter()
        .find(|entry| entry.setup == setup)
        .map(|entry| OpeningMatch {
            eco: entry.eco.clone(),
            name: entry.name.clone(),
        })
}

pub fn lookup_fen(fen: &str) -> Option<OpeningMatch> {
    setup_from_fen(fen).and_then(|setup| lookup_setup(&setup))
}

/// Deepest known opening on the line: walks FENs from current back to start.
pub fn lookup_fens(fens: &[impl AsRef<str>]) -> Option<OpeningMatch> {
    for fen in fens.iter().rev() {
        if let Some(opening) = lookup_fen(fen.as_ref()) {
            return Some(opening);
        }
    }
    None
}

pub fn format_opening(opening: &OpeningMatch) -> String {
    if opening.eco == "—" || opening.eco == "FRC" {
        opening.name.clone()
    } else {
        format!("{} · {}", opening.eco, opening.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_named_opening() {
        let opening = lookup_fen("rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPPKPPP/RNBQ1BNR b kq - 1 2")
            .expect("bongcloud position");
        assert_eq!(opening.name, "Bongcloud Attack");
    }

    #[test]
    fn novelty_fen_matches_book_after_e4_e5() {
        use crate::graph::{play_move_keys, start_fen};
        use gpui_chessboard::Key;

        let (fen_e4, ..) =
            play_move_keys(&start_fen(), &Key::new("e2").unwrap(), &Key::new("e4").unwrap()).unwrap();
        let (fen_e5, ..) =
            play_move_keys(&fen_e4, &Key::new("e7").unwrap(), &Key::new("e5").unwrap()).unwrap();

        let book_setup = setup_after_pgn("1. e4 e5").expect("book line");
        let novelty_setup = setup_from_fen(&fen_e5).expect("novelty fen");

        assert_eq!(
            novelty_setup, book_setup,
            "fen after e5: {fen_e5}"
        );
    }

    #[test]
    fn recognizes_novelty_fen_line() {
        use crate::graph::{play_move_keys, start_fen};
        use gpui_chessboard::Key;

        let mut fens = vec![start_fen()];
        let (fen, ..) =
            play_move_keys(&fens[0], &Key::new("e2").unwrap(), &Key::new("e4").unwrap()).unwrap();
        fens.push(fen.clone());
        assert_eq!(lookup_fen(&fen).unwrap().eco, "B00");

        let (fen, ..) =
            play_move_keys(&fen, &Key::new("e7").unwrap(), &Key::new("e5").unwrap()).unwrap();
        fens.push(fen.clone());
        assert_eq!(lookup_fens(&fens).unwrap().eco, "C20");

        let (fen, ..) =
            play_move_keys(&fen, &Key::new("g1").unwrap(), &Key::new("f3").unwrap()).unwrap();
        fens.push(fen);
        assert!(lookup_fens(&fens).unwrap().eco.starts_with('C'));
    }

    #[test]
    fn walks_back_for_unknown_tail() {
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
            // Position past book depth — should fall back to e4 line.
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/4N3/PPPP1PPP/RNBQKB1R b KQkq - 3 2",
        ];
        let opening = lookup_fens(&fens).expect("king's pawn");
        assert_eq!(opening.eco, "B00");
        assert!(opening.name.contains("King's Pawn"));
    }

    #[test]
    fn book_has_thousands_of_entries() {
        assert!(openings().len() > 3_000);
    }
}
