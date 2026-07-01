//! Convert engine scores to chessboard eval bar values (white's perspective).

use gpui_chessboard::{Color, EvalDisplay};
use ruci::engine;

pub fn eval_from_score(score: &engine::ScoreWithBound, side_to_move: Color) -> EvalDisplay {
    match score.kind {
        engine::Score::Centipawns(cp) => {
            let cp = if side_to_move == Color::Black { -cp } else { cp };
            EvalDisplay::cp(cp as i32)
        }
        engine::Score::MateIn(mate) => {
            let mate = if side_to_move == Color::Black { -mate } else { mate };
            EvalDisplay::mate(mate.clamp(-127, 127) as i8)
        }
    }
}
