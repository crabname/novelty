mod apply;
mod config;

pub use apply::apply_board_config;
pub use config::BoardConfig;

use gpui_chessboard::types::Color;

use crate::fetch::PlayerColor;

pub fn board_orientation(color: PlayerColor) -> Color {
    match color {
        PlayerColor::White => Color::White,
        PlayerColor::Black => Color::Black,
    }
}
