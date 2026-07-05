use gpui::*;
use gpui_chessboard::{
    config::DrawableConfigPatch, config::MovableConfigPatch, ChessboardApi, Config, MovableColor,
};

use crate::graph::turn_color;

use super::BoardConfig;

pub fn apply_board_config(api: &ChessboardApi, config: &BoardConfig, cx: &mut App) {
    api.set(
        Config {
            fen: Some(config.fen.clone()),
            orientation: Some(config.orientation),
            turn_color: Some(turn_color(&config.fen)),
            view_only: Some(false),
            last_move: Some(config.last_move.clone()),
            movable: Some(MovableConfigPatch {
                free: Some(false),
                color: Some(Some(MovableColor::Both)),
                dests: Some(Some(config.dests.clone())),
                show_dests: Some(config.show_dests),
                ..Default::default()
            }),
            drawable: Some(DrawableConfigPatch {
                auto_shapes: Some(config.shapes.clone()),
                ..Default::default()
            }),
            eval: config.eval.clone(),
            ..Default::default()
        },
        cx,
    );
}
