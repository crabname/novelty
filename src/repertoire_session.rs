//! Repertoire tab: variation tree, PGN file sync.

use std::path::PathBuf;

use gpui::*;
use gpui_component::input::InputState;
use gpui_chessboard::{ChessboardApi, ChessboardView};

use crate::board::{apply_board_config, BoardConfig};
use crate::graph::{legal_dests_at, play_san_at};
use crate::move_tree::MoveTree;
use crate::opening_book::{format_opening, lookup_fens};
use crate::opening_explorer::{ExplorerHost, ExplorerState};
use crate::panel_tabs::SidePanelTab;
use crate::pgn_tree::{format_repertoire_pgn_map, parse_repertoire_pgn, ParsedRepertoire};
use crate::repertoire;

pub struct RepertoireSession {
    pub id: u64,
    pub label: SharedString,
    pub board: Entity<ChessboardView>,
    pub api: ChessboardApi,
    pub pgn_input: Entity<InputState>,
    pub name_input: Entity<InputState>,
    pub file_path: Option<PathBuf>,
    pub headers: Vec<(String, String)>,
    pub tree: MoveTree,
    pub last_synced_move: Option<(gpui_chessboard::Key, gpui_chessboard::Key)>,
    pub last_parsed_pgn: String,
    pub dirty: bool,
    pub needs_pgn_ui_sync: bool,
    pub status: SharedString,
    pub side_panel_tab: SidePanelTab,
    pub explorer: ExplorerState,
}

impl RepertoireSession {
    pub fn new(
        id: u64,
        board: Entity<ChessboardView>,
        api: ChessboardApi,
        pgn_input: Entity<InputState>,
        name_input: Entity<InputState>,
    ) -> Self {
        let headers = repertoire::initial_headers("New repertoire");
        let tree = MoveTree::new();
        let header_map: std::collections::HashMap<String, String> =
            headers.iter().cloned().collect();
        let pgn = format_repertoire_pgn_map(&header_map, &tree);
        Self {
            id,
            label: "Repertoire".into(),
            board,
            api,
            pgn_input,
            name_input,
            file_path: None,
            headers,
            tree,
            last_synced_move: None,
            last_parsed_pgn: pgn,
            dirty: false,
            needs_pgn_ui_sync: false,
            status: "Create a repertoire or open an existing one".into(),
            side_panel_tab: SidePanelTab::Explorer,
            explorer: ExplorerState::default(),
        }
    }

    pub fn load_from_path(&mut self, path: PathBuf, cx: &mut App) -> Result<(), String> {
        let text = repertoire::load_repertoire_file(&path)?;
        let game = parse_repertoire_pgn(&text)?;
        self.file_path = Some(path);
        self.load_game(game, cx);
        self.dirty = false;
        Ok(())
    }

    pub fn try_load_pgn_from_text(&mut self, text: &str, cx: &mut App) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        if text == self.last_parsed_pgn {
            return false;
        }
        match parse_repertoire_pgn(text) {
            Ok(game) => {
                self.last_parsed_pgn = text.to_string();
                self.load_game(game, cx);
                self.dirty = true;
                true
            }
            Err(err) => {
                self.status = err.into();
                false
            }
        }
    }

    fn load_game(&mut self, game: ParsedRepertoire, cx: &mut App) {
        self.label = game.label.into();
        self.headers = game.headers.into_iter().collect();
        self.tree = game.tree;
        self.last_synced_move = self.tree.last_move_keys();
        self.status = format!(
            "{} moves on mainline",
            self.tree.mainline_steps().len().saturating_sub(1)
        )
        .into();
        self.needs_pgn_ui_sync = true;
        self.last_parsed_pgn = self.current_pgn();
        self.sync_board_from_tree(cx);
    }

    pub fn refresh_board(&mut self, cx: &mut App) {
        let current = self.tree.current();
        let fen = current.fen.clone();
        let last_move = self.tree.last_move_keys().map(|(orig, dest)| vec![orig, dest]);

        let config = BoardConfig {
            fen,
            last_move,
            dests: legal_dests_at(&self.tree.current().fen).unwrap_or_default(),
            show_dests: true,
            shapes: Vec::new(),
            eval: None,
        };
        apply_board_config(&self.api, &config, cx);
    }

    fn sync_board_from_tree(&mut self, cx: &mut App) {
        self.last_synced_move = self.tree.last_move_keys();
        self.refresh_board(cx);
    }

    pub fn flush_pgn_ui_if_needed(&mut self, window: &mut Window, cx: &mut App) {
        if !self.needs_pgn_ui_sync {
            return;
        }
        self.needs_pgn_ui_sync = false;
        self.sync_pgn_to_input(window, cx);
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

        if self.tree.make_move_from_board(&orig, &dest).is_err() {
            return false;
        }

        self.last_synced_move = Some((orig, dest));
        self.dirty = true;
        self.status = if self.tree.variation_mode {
            "Added variation".into()
        } else {
            "Move added".into()
        };
        self.needs_pgn_ui_sync = true;
        self.last_parsed_pgn = self.current_pgn();
        self.sync_board_from_tree(cx);
        if self.file_path.is_some() {
            let _ = self.save_to_file();
        }
        true
    }

    pub fn enable_variation_mode(&mut self) {
        self.tree.variation_mode = true;
        self.status = "Next move will be added as a variation".into();
    }

    pub fn promote_variation(&mut self) -> bool {
        if self.tree.promote_current_variation() {
            self.dirty = true;
            self.needs_pgn_ui_sync = true;
            self.last_parsed_pgn = self.current_pgn();
            self.status = "Variation promoted to mainline".into();
            true
        } else {
            false
        }
    }

    pub fn sync_pgn_to_input(&mut self, window: &mut Window, cx: &mut App) {
        let header_map: std::collections::HashMap<String, String> =
            self.headers.iter().cloned().collect();
        let pgn = format_repertoire_pgn_map(&header_map, &self.tree);
        self.last_parsed_pgn = pgn.clone();
        self.pgn_input.update(cx, |input, cx| {
            input.set_value(pgn, window, cx);
        });
    }

    pub fn go_to_position(&mut self, position: Vec<usize>, cx: &mut App) {
        self.tree.go_to_position(position);
        self.sync_board_from_tree(cx);
    }

    pub fn go_back(&mut self, cx: &mut App) {
        if self.tree.go_back() {
            self.sync_board_from_tree(cx);
        }
    }

    pub fn go_forward(&mut self, cx: &mut App) {
        if self.tree.go_forward_mainline() {
            self.sync_board_from_tree(cx);
        }
    }

    pub fn next_branch(&mut self, cx: &mut App) {
        if self.tree.next_branch() {
            self.sync_board_from_tree(cx);
        }
    }

    pub fn previous_branch(&mut self, cx: &mut App) {
        if self.tree.previous_branch() {
            self.sync_board_from_tree(cx);
        }
    }

    pub fn opening_label(&self) -> SharedString {
        let path_fens = self.tree.path_fens();
        let fens: Vec<&str> = path_fens.iter().map(String::as_str).collect();
        lookup_fens(&fens)
            .map(|opening| format_opening(&opening))
            .unwrap_or_else(|| "Unknown opening".into())
            .into()
    }

    pub fn current_pgn(&self) -> String {
        let header_map: std::collections::HashMap<String, String> =
            self.headers.iter().cloned().collect();
        format_repertoire_pgn_map(&header_map, &self.tree)
    }

    pub fn save_to_file(&mut self) -> Result<(), String> {
        let path = self
            .file_path
            .as_ref()
            .ok_or_else(|| "No repertoire file — create one first".to_string())?;
        repertoire::save_repertoire(path, &self.current_pgn())?;
        self.dirty = false;
        self.status = format!(
            "Saved {}",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("repertoire")
        )
        .into();
        Ok(())
    }
}

impl ExplorerHost for RepertoireSession {
    fn explorer_fen(&self) -> &str {
        &self.tree.current().fen
    }

    fn explorer_state(&self) -> &ExplorerState {
        &self.explorer
    }

    fn explorer_state_mut(&mut self) -> &mut ExplorerState {
        &mut self.explorer
    }

    fn play_explorer_san(&mut self, san: &str, cx: &mut App) {
        let Ok((_, _, orig, dest)) = play_san_at(&self.tree.current().fen, san) else {
            return;
        };
        if self.tree.make_move_from_board(&orig, &dest).is_err() {
            return;
        }
        self.last_synced_move = Some((orig, dest));
        self.dirty = true;
        self.status = if self.tree.variation_mode {
            "Added variation".into()
        } else {
            "Move added".into()
        };
        self.needs_pgn_ui_sync = true;
        self.last_parsed_pgn = self.current_pgn();
        self.sync_board_from_tree(cx);
        if self.file_path.is_some() {
            let _ = self.save_to_file();
        }
    }
}
