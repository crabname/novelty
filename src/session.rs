//! One loaded (or empty) profile tab: board, graph, history, fetch state.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use gpui::*;

use gpui_chessboard::{
    config::DrawableConfigPatch, config::EvalConfigPatch, config::MovableConfigPatch,
    ChessboardApi, ChessboardView, Config, EvalBarPosition, Key, MovableColor,
};

use crate::analysis_session::AnalysisSettings;
use crate::engine_shapes::engine_line_shapes;
use crate::engine_uci::AnalysisResult;
use crate::graph::{start_fen, turn_color, OpeningGraph};
use crate::opening_book::{format_opening, lookup_fens};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ControlPanelTab {
    #[default]
    NextMoves,
    Engine,
}

impl ControlPanelTab {
    pub fn index(self) -> usize {
        match self {
            Self::NextMoves => 0,
            Self::Engine => 1,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Engine,
            _ => Self::NextMoves,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HistoryStep {
    pub fen: String,
    pub san: Option<String>,
    pub orig: Option<Key>,
    pub dest: Option<Key>,
}

impl HistoryStep {
    pub(crate) fn start(fen: String) -> Self {
        Self {
            fen,
            san: None,
            orig: None,
            dest: None,
        }
    }

    pub(crate) fn after_move(fen: String, san: String, orig: Key, dest: Key) -> Self {
        Self {
            fen,
            san: Some(san),
            orig: Some(orig),
            dest: Some(dest),
        }
    }
}

pub struct ProfileSession {
    pub id: u64,
    pub label: SharedString,
    pub username: String,
    pub board: Entity<ChessboardView>,
    pub api: ChessboardApi,
    pub graph: Arc<Mutex<OpeningGraph>>,
    pub cancel_load: Arc<AtomicBool>,
    pub current_fen: String,
    pub history: Vec<HistoryStep>,
    pub history_index: usize,
    pub last_synced_move: Option<(Key, Key)>,
    pub status: SharedString,
    pub loading: bool,
    pub next_move_count: usize,
    pub control_tab: ControlPanelTab,
    pub selected_engine_id: Option<String>,
    pub analyzing: bool,
    pub analysis: Option<AnalysisResult>,
    pub settings: AnalysisSettings,
}

impl ProfileSession {
    pub fn new(
        id: u64,
        label: impl Into<SharedString>,
        board: Entity<ChessboardView>,
        api: ChessboardApi,
    ) -> Self {
        let label = label.into();
        let current_fen = start_fen();
        Self {
            id,
            label: label.clone(),
            username: String::new(),
            board,
            api,
            graph: Arc::new(Mutex::new(OpeningGraph::default())),
            cancel_load: Arc::new(AtomicBool::new(false)),
            current_fen: current_fen.clone(),
            history: vec![HistoryStep::start(current_fen)],
            history_index: 0,
            last_synced_move: None,
            status: "Load games from the sidebar".into(),
            loading: false,
            next_move_count: 0,
            control_tab: ControlPanelTab::NextMoves,
            selected_engine_id: None,
            analyzing: false,
            analysis: None,
            settings: AnalysisSettings {
                depth: 14,
                line_count: 3,
                show_engine_lines: true,
            },
        }
    }

    pub fn reset_for_load(&mut self, username: String, cx: &mut App) {
        self.username = username.clone();
        self.label = username.into();
        self.cancel_load.store(false, Ordering::Relaxed);
        self.loading = true;
        self.last_synced_move = None;
        let fen = start_fen();
        self.history = vec![HistoryStep::start(fen.clone())];
        self.history_index = 0;
        self.current_fen = fen;
        self.graph.lock().expect("graph lock").clear();
        self.clear_engine_analysis();
        self.refresh_board(cx);
    }

    pub fn refresh_board(&mut self, cx: &mut App) {
        let graph = self.graph.lock().expect("graph lock");
        let moves = graph.moves_at(&self.current_fen);
        self.next_move_count = moves.len();
        let shapes = OpeningGraph::auto_shapes(&self.current_fen, &graph);
        let mut shapes = shapes;
        if self.settings.show_engine_lines
            && let Some(analysis) = &self.analysis
        {
            shapes.extend(engine_line_shapes(analysis));
        }
        let dests = OpeningGraph::dests_for_moves(&moves);
        let turn = turn_color(&self.current_fen);
        drop(graph);

        let last_move = self
            .history
            .get(self.history_index)
            .and_then(|step| match (&step.orig, &step.dest) {
                (Some(orig), Some(dest)) => Some(vec![orig.clone(), dest.clone()]),
                _ => None,
            });

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
                turn_color: Some(turn),
                view_only: Some(false),
                last_move: Some(last_move),
                movable: Some(MovableConfigPatch {
                    free: Some(false),
                    color: Some(Some(MovableColor::Both)),
                    dests: Some(Some(dests)),
                    show_dests: Some(!moves.is_empty()),
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

        let moves = self.graph.lock().expect("graph lock").moves_at(&self.current_fen);
        let Some(mv) = moves.iter().find(|m| m.orig == orig && m.dest == dest) else {
            return false;
        };

        self.last_synced_move = Some((orig.clone(), dest.clone()));
        self.append_history_step(mv.target_fen.clone(), mv.san.clone(), orig, dest, cx);
        true
    }

    pub fn play_move_san(&mut self, san: &str, cx: &mut App) {
        let moves = self.graph.lock().expect("graph lock").moves_at(&self.current_fen);
        let Some(mv) = moves.iter().find(|m| m.san == san) else {
            return;
        };
        self.last_synced_move = Some((mv.orig.clone(), mv.dest.clone()));
        self.append_history_step(
            mv.target_fen.clone(),
            mv.san.clone(),
            mv.orig.clone(),
            mv.dest.clone(),
            cx,
        );
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
        self.refresh_board(cx);
    }

    pub fn go_to_history(&mut self, index: usize, cx: &mut App) {
        if index >= self.history.len() {
            return;
        }
        self.history_index = index;
        let step = self.history[index].clone();
        self.current_fen = step.fen;
        self.last_synced_move = step.orig.zip(step.dest);
        self.refresh_board(cx);
    }

    pub fn go_back(&mut self, cx: &mut App) {
        if self.history_index > 0 {
            self.go_to_history(self.history_index - 1, cx);
        }
    }

    pub fn go_forward_popular(&mut self, cx: &mut App) {
        let Some(mv) = self
            .graph
            .lock()
            .expect("graph lock")
            .moves_at(&self.current_fen)
            .into_iter()
            .next()
        else {
            return;
        };
        self.last_synced_move = Some((mv.orig.clone(), mv.dest.clone()));
        self.append_history_step(
            mv.target_fen.clone(),
            mv.san.clone(),
            mv.orig,
            mv.dest,
            cx,
        );
    }

    pub fn stop_loading(&mut self) {
        if !self.loading {
            return;
        }
        self.cancel_load.store(true, Ordering::Relaxed);
        self.status = "Stopping…".into();
    }

    pub fn game_count(&self) -> u32 {
        self.graph.lock().expect("graph lock").game_count()
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

    pub fn set_eval_pending(&mut self, cx: &mut App) {
        self.analyzing = true;
        self.analysis = None;
        self.refresh_board(cx);
    }

    pub fn apply_engine_analysis(&mut self, result: &AnalysisResult, cx: &mut App) {
        self.analyzing = false;
        self.analysis = Some(result.clone());
        self.refresh_board(cx);
    }

    pub fn set_engine_analysis_error(&mut self) {
        self.analyzing = false;
        self.analysis = None;
    }

    fn clear_engine_analysis(&mut self) {
        self.analyzing = false;
        self.analysis = None;
    }
}
