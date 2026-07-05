use gpui_chessboard::config::EvalConfigPatch;
use gpui_chessboard::{Dests, DrawShape, Key};
use gpui_chessboard::types::Color;

/// Serializable board view state passed to `apply_board_config`.
#[derive(Clone, Debug)]
pub struct BoardConfig {
    pub fen: String,
    pub orientation: Color,
    pub last_move: Option<Vec<Key>>,
    pub dests: Dests,
    pub show_dests: bool,
    pub shapes: Vec<DrawShape>,
    pub eval: Option<EvalConfigPatch>,
}
