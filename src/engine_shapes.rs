//! Engine best-move arrows on the chessboard (distinct from repertoire greens).

use gpui_chessboard::draw::{DrawModifiers, DrawShape};
use gpui_chessboard::Key;
use shakmaty::uci::UciMove;

use crate::engine_uci::AnalysisResult;

const ENGINE_ARROW_COLOR: &str = "#2563eb";

/// Line width and opacity by rank (0 = best line, boldest).
fn line_style(index: usize) -> (f32, f32) {
    match index {
        0 => (16.0, 0.95),
        1 => (12.0, 0.72),
        2 => (9.0, 0.52),
        3 => (7.0, 0.38),
        _ => (6.0, 0.28),
    }
}

pub fn engine_line_shapes(result: &AnalysisResult) -> Vec<DrawShape> {
    result
        .lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            line.pv
                .first()
                .and_then(|uci| uci_move_shape(uci, index))
        })
        .collect()
}

fn uci_move_shape(uci: &str, index: usize) -> Option<DrawShape> {
    let uci: UciMove = uci.parse().ok()?;
    let from = uci.from()?;
    let to = uci.to()?;
    let (line_width, opacity) = line_style(index);
    Some(DrawShape {
        orig: square_to_key(from),
        dest: Some(square_to_key(to)),
        brush: Some("g".into()),
        modifiers: Some(DrawModifiers {
            color: Some(ENGINE_ARROW_COLOR.into()),
            opacity: Some(opacity),
            line_width: Some(line_width),
            hilite: None,
        }),
        label: None,
        below: false,
    })
}

fn square_to_key(sq: shakmaty::Square) -> Key {
    let file = sq.file().to_string();
    let rank = (sq.rank() as u8) + 1;
    Key::new(&format!("{file}{rank}")).expect("valid square")
}
