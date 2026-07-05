//! One loaded (or empty) profile tab: board, graph, history, fetch state.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use gpui::*;

use gpui_chessboard::{ChessboardApi, ChessboardView, Key};
use gpui_chessboard::types::Color;

use crate::analysis_session::AnalysisSettings;
use crate::board::{apply_board_config, BoardConfig};
use crate::engine_state::EngineState;
use crate::engine_uci::AnalysisResult;
use crate::graph::{start_fen, OpeningGraph};
use crate::opening_book::{format_opening, lookup_fens};
use crate::panel_tabs::SidePanelTab;

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
    pub side_panel_tab: SidePanelTab,
    pub engine: EngineState,
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
            side_panel_tab: SidePanelTab::Moves,
            engine: EngineState {
                selected_engine_id: None,
                analyzing: false,
                analysis: None,
                settings: AnalysisSettings {
                    depth: 14,
                    line_count: 3,
                    show_engine_lines: true,
                },
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
        let mut shapes = OpeningGraph::auto_shapes(&self.current_fen, &graph);
        shapes.extend(self.engine.board_shapes());
        let dests = OpeningGraph::dests_for_moves(&moves);
        drop(graph);

        let last_move = self
            .history
            .get(self.history_index)
            .and_then(|step| match (&step.orig, &step.dest) {
                (Some(orig), Some(dest)) => Some(vec![orig.clone(), dest.clone()]),
                _ => None,
            });

        let config = BoardConfig {
            fen: self.current_fen.clone(),
            orientation: Color::White,
            last_move,
            dests,
            show_dests: self.next_move_count > 0,
            shapes,
            eval: self.engine.eval_patch(),
        };
        apply_board_config(&self.api, &config, cx);
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
        self.engine.set_eval_pending();
        self.refresh_board(cx);
    }

    pub fn apply_engine_analysis(&mut self, result: &AnalysisResult, cx: &mut App) {
        self.engine.apply_analysis(result);
        self.refresh_board(cx);
    }

    pub fn set_engine_analysis_error(&mut self) {
        self.engine.clear_analysis();
    }

    fn clear_engine_analysis(&mut self) {
        self.engine.clear_analysis();
    }
}
