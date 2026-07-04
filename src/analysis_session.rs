//! Game analysis tab: PGN input, board navigation, engine evaluation.

use gpui::*;
use gpui_component::input::InputState;
use gpui_chessboard::{
    config::DrawableConfigPatch, config::EvalConfigPatch, config::MovableConfigPatch,
    ChessboardApi, ChessboardView, Config, EvalBarPosition, Key, MovableColor,
};

use crate::engine_shapes::engine_line_shapes;
use crate::engine_uci::AnalysisResult;
use crate::graph::{legal_dests_at, play_move_keys, start_fen, turn_color};
use crate::opening_book::{format_opening, lookup_fens};
use crate::pgn::{self, ParsedGame};
use crate::session::HistoryStep;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AnalysisPanelTab {
    #[default]
    Engine,
    Game,
}

impl AnalysisPanelTab {
    pub fn index(self) -> usize {
        match self {
            Self::Engine => 0,
            Self::Game => 1,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Game,
            _ => Self::Engine,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnalysisSettings {
    pub depth: u32,
    pub line_count: u32,
    pub show_engine_lines: bool,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            depth: 16,
            line_count: 3,
            show_engine_lines: true,
        }
    }
}

pub struct AnalysisSession {
    pub id: u64,
    pub label: SharedString,
    pub board: Entity<ChessboardView>,
    pub api: ChessboardApi,
    pub pgn_input: Entity<InputState>,
    pub headers: Vec<(String, String)>,
    pub current_fen: String,
    pub history: Vec<HistoryStep>,
    pub history_index: usize,
    pub last_synced_move: Option<(Key, Key)>,
    pub last_parsed_pgn: String,
    pub status: SharedString,
    pub selected_engine_id: Option<String>,
    pub analyzing: bool,
    pub analysis: Option<AnalysisResult>,
    pub settings: AnalysisSettings,
    pub panel_tab: AnalysisPanelTab,
}

impl AnalysisSession {
    pub fn new(
        id: u64,
        board: Entity<ChessboardView>,
        api: ChessboardApi,
        pgn_input: Entity<InputState>,
    ) -> Self {
        let fen = start_fen();
        Self {
            id,
            label: "Game Analysis".into(),
            board,
            api,
            pgn_input,
            headers: Vec::new(),
            current_fen: fen.clone(),
            history: vec![HistoryStep::start(fen)],
            history_index: 0,
            last_synced_move: None,
            last_parsed_pgn: String::new(),
            status: "Paste PGN or play moves on the board".into(),
            selected_engine_id: None,
            analyzing: false,
            analysis: None,
            settings: AnalysisSettings::default(),
            panel_tab: AnalysisPanelTab::Engine,
        }
    }

    pub fn try_load_pgn_from_text(&mut self, text: &str, cx: &mut App) -> bool {
        let text = text.trim();
        if text.is_empty() {
            self.last_parsed_pgn.clear();
            self.status = "Paste PGN or play moves on the board".into();
            return false;
        }
        if text == self.last_parsed_pgn {
            return false;
        }
        match pgn::parse_pgn(text) {
            Ok(game) => {
                self.last_parsed_pgn = text.to_string();
                self.load_game(game, cx);
                true
            }
            Err(err) => {
                self.status = err.into();
                false
            }
        }
    }

    pub fn load_game(&mut self, game: ParsedGame, cx: &mut App) {
        self.label = game.label.into();
        self.headers = game.headers.into_iter().collect();
        self.history = game.history;
        self.history_index = 0;
        self.current_fen = self.history[0].fen.clone();
        self.last_synced_move = None;
        self.clear_analysis();
        self.status = format!(
            "{} moves loaded",
            self.history.len().saturating_sub(1)
        )
        .into();
        self.sync_history_cursor(cx);
    }

    pub fn refresh_board(&mut self, cx: &mut App) {
        let last_move = self
            .history
            .get(self.history_index)
            .and_then(|step| match (&step.orig, &step.dest) {
                (Some(orig), Some(dest)) => Some(vec![orig.clone(), dest.clone()]),
                _ => None,
            });

        let dests = legal_dests_at(&self.current_fen).unwrap_or_default();

        let mut shapes = Vec::new();
        if self.settings.show_engine_lines {
            if let Some(result) = &self.analysis {
                shapes.extend(engine_line_shapes(result));
            }
        }

        let eval = if self.selected_engine_id.is_some() {
            Some(EvalConfigPatch {
                enabled: Some(true),
                position: Some(EvalBarPosition::Left),
                display: Some(if self.analyzing {
                    None
                } else {
                    self.analysis.as_ref().and_then(|result| result.best_eval())
                }),
            })
        } else {
            Some(EvalConfigPatch {
                enabled: Some(false),
                ..Default::default()
            })
        };

        self.api.set(
            Config {
                fen: Some(self.current_fen.clone()),
                turn_color: Some(turn_color(&self.current_fen)),
                view_only: Some(false),
                last_move: Some(last_move),
                movable: Some(MovableConfigPatch {
                    free: Some(false),
                    color: Some(Some(MovableColor::Both)),
                    dests: Some(Some(dests)),
                    show_dests: Some(true),
                    ..Default::default()
                }),
                drawable: Some(DrawableConfigPatch {
                    auto_shapes: Some(shapes),
                    ..Default::default()
                }),
                eval,
                ..Default::default()
            },
            cx,
        );
    }

    pub fn on_board_changed(&mut self, cx: &mut App) -> bool {
        let Some(keys) = self.api.state(cx).last_move.clone() else {
            return false;
        };
        if keys.len() < 2 {
            return false;
        }
        let orig = keys[0].clone();
        let dest = keys[1].clone();
        if self.last_synced_move.as_ref() == Some(&(orig.clone(), dest.clone())) {
            return false;
        }

        let Ok((target_fen, san, orig_key, dest_key)) =
            play_move_keys(&self.current_fen, &orig, &dest)
        else {
            return false;
        };

        self.last_synced_move = Some((orig_key.clone(), dest_key.clone()));
        self.append_history_step(target_fen, san, orig_key, dest_key, cx);
        true
    }

    fn append_history_step(
        &mut self,
        fen: String,
        san: String,
        orig: Key,
        dest: Key,
        cx: &mut App,
    ) {
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history
            .push(HistoryStep::after_move(fen.clone(), san, orig, dest));
        self.history_index = self.history.len() - 1;
        self.current_fen = fen;
        self.clear_analysis();
        self.status = format!(
            "Move {} · play on board or paste PGN",
            self.history_index
        )
        .into();
        self.refresh_board(cx);
    }

    pub fn go_to_history(&mut self, index: usize, cx: &mut App) {
        if index >= self.history.len() {
            return;
        }
        self.history_index = index;
        self.sync_history_cursor(cx);
    }

    fn sync_history_cursor(&mut self, cx: &mut App) {
        let step = self.history[self.history_index].clone();
        self.current_fen = step.fen;
        self.last_synced_move = step.orig.zip(step.dest);
        self.clear_analysis();
        self.refresh_board(cx);
    }

    pub fn go_back(&mut self, cx: &mut App) {
        if self.history_index > 0 {
            self.go_to_history(self.history_index - 1, cx);
        }
    }

    pub fn go_forward(&mut self, cx: &mut App) {
        if self.history_index + 1 < self.history.len() {
            self.go_to_history(self.history_index + 1, cx);
        }
    }

    fn clear_analysis(&mut self) {
        self.analyzing = false;
        self.analysis = None;
    }

    pub fn set_analysis_pending(&mut self, cx: &mut App) {
        self.analyzing = true;
        self.analysis = None;
        self.refresh_board(cx);
    }

    pub fn apply_analysis(&mut self, result: &AnalysisResult, cx: &mut App) {
        self.analyzing = false;
        self.analysis = Some(result.clone());
        self.refresh_board(cx);
    }

    pub fn set_analysis_error(&mut self, message: String) {
        self.analyzing = false;
        self.analysis = None;
        self.status = message.into();
    }

    pub fn opening_label(&self) -> SharedString {
        let fens: Vec<&str> = self
            .history
            .iter()
            .take(self.history_index + 1)
            .map(|step| step.fen.as_str())
            .collect();
        lookup_fens(&fens)
            .map(|opening| format_opening(&opening))
            .unwrap_or_else(|| "Unknown opening".into())
            .into()
    }
}
