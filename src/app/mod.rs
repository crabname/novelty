mod lichess_account;
mod load;
mod uci;

use std::collections::HashMap;

use gpui::*;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::searchable_list::SearchableVec;
use gpui_component::select::{SelectEvent, SelectState};

use crate::analysis_session::AnalysisSession;
use crate::engine_catalog::{self, CatalogOffer};
use crate::engines::{self, LocalEngine};
use crate::fetch::{LoadPeriod, PlayerColor, Site, TimeControlFilter};
use crate::lichess::{self, LichessSession};
use crate::profiles::remember_profile;
use crate::session::ProfileSession;
use crate::tab::{AppTab, TabKind};

use gpui_chessboard::{ChessboardCallbacks, ChessboardView, Chessground, Config};

pub use uci::{UciConnectionStatus, UciEngineState};

pub struct NoveltyApp {
    pub(crate) tabs: Vec<AppTab>,
    pub(crate) active_tab: usize,
    next_tab_id: u64,
    pub(crate) username: Entity<InputState>,
    pub(crate) period_select: Entity<SelectState<Vec<LoadPeriod>>>,
    pub(crate) profile_select: Entity<SelectState<SearchableVec<String>>>,
    pub(crate) profile_history: Vec<String>,
    pub(crate) site: Site,
    pub(crate) color: PlayerColor,
    pub(crate) lichess_session: Option<LichessSession>,
    pub(crate) lichess_auth_status: SharedString,
    pub(crate) time_controls: TimeControlFilter,
    pub(crate) pending_username: Option<String>,
    pub(crate) pending_profile_remember: Option<String>,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) needs_focus: bool,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) engines: Vec<LocalEngine>,
    pub(crate) engine_status: SharedString,
    pub(crate) catalog_offers: Vec<CatalogOffer>,
    pub(crate) catalog_loading: bool,
    pub(crate) catalog_error: Option<String>,
    pub(crate) downloading_catalog_id: Option<String>,
    pub(crate) uci_states: HashMap<String, UciEngineState>,
    pub(crate) uci_sessions: HashMap<String, crate::engine_uci::UciSession>,
    pub(crate) pending_analysis: Option<uci::PendingConnectAnalysis>,
}

impl NoveltyApp {
    pub fn new(
        username: Entity<InputState>,
        period_select: Entity<SelectState<Vec<LoadPeriod>>>,
        profile_select: Entity<SelectState<SearchableVec<String>>>,
        profile_history: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let profile_select_watch = profile_select.clone();
        cx.subscribe(&profile_select_watch, |app, _, event, cx| {
            if let SelectEvent::Confirm(Some(name)) = event {
                app.pending_username = Some(name.clone());
                cx.notify();
            }
        })
        .detach();

        let lichess_session = lichess::load_session();
        let lichess_auth_status = lichess_session.as_ref().map_or(SharedString::default(), |s| {
            format!("Connected to Lichess as {}", s.username).into()
        });
        if let Some(session) = &lichess_session {
            username.update(cx, |input, cx| {
                input.set_value(session.username.clone(), window, cx);
            });
        }

        Self {
            tabs: vec![AppTab::Home { id: 0 }],
            active_tab: 0,
            next_tab_id: 1,
            username,
            period_select,
            profile_select,
            profile_history,
            site: Site::Lichess,
            color: PlayerColor::White,
            lichess_session,
            lichess_auth_status,
            time_controls: TimeControlFilter::default(),
            pending_username: None,
            pending_profile_remember: None,
            focus_handle: cx.focus_handle(),
            needs_focus: true,
            sidebar_collapsed: false,
            engines: engines::load_engines(),
            engine_status: SharedString::default(),
            catalog_offers: Vec::new(),
            catalog_loading: false,
            catalog_error: None,
            downloading_catalog_id: None,
            uci_states: HashMap::new(),
            uci_sessions: HashMap::new(),
            pending_analysis: None,
        }
    }

    pub fn tab_by_id_mut(&mut self, id: u64) -> Option<&mut AppTab> {
        self.tabs.iter_mut().find(|tab| tab.id() == id)
    }

    pub(crate) fn active_opening_tree(&self) -> Option<&ProfileSession> {
        self.tabs.get(self.active_tab)?.opening_tree()
    }

    pub(crate) fn active_opening_tree_mut(&mut self) -> Option<&mut ProfileSession> {
        self.tabs.get_mut(self.active_tab)?.opening_tree_mut()
    }

    pub(crate) fn opening_tree_at(&self, index: usize) -> Option<&ProfileSession> {
        self.tabs.get(index)?.opening_tree()
    }

    pub(crate) fn opening_tree_at_mut(&mut self, index: usize) -> Option<&mut ProfileSession> {
        self.tabs.get_mut(index)?.opening_tree_mut()
    }

    pub(crate) fn game_analysis_at(&self, index: usize) -> Option<&AnalysisSession> {
        self.tabs.get(index)?.game_analysis()
    }

    pub(crate) fn game_analysis_at_mut(&mut self, index: usize) -> Option<&mut AnalysisSession> {
        self.tabs.get_mut(index)?.game_analysis_mut()
    }

    pub(crate) fn active_game_analysis(&self) -> Option<&AnalysisSession> {
        self.tabs.get(self.active_tab)?.game_analysis()
    }

    pub(crate) fn active_game_analysis_mut(&mut self) -> Option<&mut AnalysisSession> {
        self.tabs.get_mut(self.active_tab)?.game_analysis_mut()
    }

    pub(crate) fn game_analysis_by_id_mut(&mut self, tab_id: u64) -> Option<&mut AnalysisSession> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.id() == tab_id)
            .and_then(|tab| tab.game_analysis_mut())
    }

    pub(crate) fn opening_tree_by_id_mut(&mut self, session_id: u64) -> Option<&mut ProfileSession> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.opening_tree().is_some_and(|session| session.id == session_id))
            .and_then(|tab| tab.opening_tree_mut())
    }

    pub(crate) fn opening_tree_by_id(&self, session_id: u64) -> Option<&ProfileSession> {
        self.tabs
            .iter()
            .find(|tab| tab.opening_tree().is_some_and(|session| session.id == session_id))
            .and_then(|tab| tab.opening_tree())
    }

    pub(crate) fn selected_period(&self, cx: &App) -> LoadPeriod {
        self.period_select
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn remember_loaded_profile(
        &mut self,
        username: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        remember_profile(&mut self.profile_history, username);
        self.profile_select.update(cx, |select, cx| {
            select.set_items(
                SearchableVec::new(self.profile_history.clone()),
                window,
                cx,
            );
        });
    }

    fn on_board_changed(&mut self, board: Entity<ChessboardView>, cx: &mut Context<Self>) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.opening_tree().is_some_and(|session| session.board == board))
        {
            if let Some(session) = self.tabs.get_mut(index).and_then(|tab| tab.opening_tree_mut())
            {
                let session_id = session.id;
                let moved = session.on_board_changed(cx);
                if moved && session.selected_engine_id.is_some() {
                    self.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                }
            }
            if index == self.active_tab {
                cx.notify();
            }
            return;
        }

        let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.game_analysis().is_some_and(|session| session.board == board))
        else {
            return;
        };
        if let Some(session) = self.tabs.get_mut(index).and_then(|tab| tab.game_analysis_mut()) {
            let tab_id = session.id;
            if session.on_board_changed(cx) && index == self.active_tab {
                self.refresh_analysis_if_engine_selected(tab_id, cx);
            }
        }
        if index == self.active_tab {
            cx.notify();
        }
    }

    pub(crate) fn add_home_tab(&mut self, cx: &mut Context<Self>) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(AppTab::Home { id });
        self.active_tab = self.tabs.len() - 1;
        cx.notify();
    }

    pub(crate) fn open_mode(
        &mut self,
        kind: TabKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match kind {
            TabKind::OpeningTree => self.open_opening_tree_tab(window, cx),
            TabKind::GameAnalysis => self.open_game_analysis_tab(window, cx),
            TabKind::Engine => self.open_engines_tab(cx),
            TabKind::Settings => self.open_settings_tab(cx),
            _ => {
                let id = self.next_tab_id;
                self.next_tab_id += 1;
                self.tabs.push(AppTab::Stub { id, kind });
                self.active_tab = self.tabs.len() - 1;
                cx.notify();
            }
        }
    }

    fn open_settings_tab(&mut self, cx: &mut Context<Self>) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(AppTab::Settings { id });
        self.active_tab = self.tabs.len() - 1;
        cx.notify();
    }

    fn open_engines_tab(&mut self, cx: &mut Context<Self>) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(AppTab::Engines { id });
        self.active_tab = self.tabs.len() - 1;
        if !self.catalog_loading && self.catalog_offers.is_empty() {
            self.refresh_engine_catalog(cx);
        }
        cx.notify();
    }

    fn open_opening_tree_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let (board, api) = Chessground::new(
            Config::default(),
            ChessboardCallbacks::default(),
            window,
            cx,
        );
        let board_watch = board.clone();
        cx.observe(&board_watch, |app, board, cx| {
            app.on_board_changed(board, cx);
        })
        .detach();
        let mut session = ProfileSession::new(id, TabKind::OpeningTree.label(), board, api);
        session.refresh_board(cx);
        self.tabs.push(AppTab::OpeningTree { id, session });
        self.active_tab = self.tabs.len() - 1;
        cx.notify();
    }

    fn open_game_analysis_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let (board, api) = Chessground::new(
            Config::default(),
            ChessboardCallbacks::default(),
            window,
            cx,
        );
        let board_watch = board.clone();
        cx.observe(&board_watch, |app, board, cx| {
            app.on_board_changed(board, cx);
        })
        .detach();

        let pgn_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(8)
                .placeholder("Paste PGN or movetext…")
        });
        let pgn_watch = pgn_input.clone();
        let tab_id = id;
        cx.subscribe(&pgn_watch, move |app, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                let text = input.read(cx).value();
                let loaded = app
                    .game_analysis_by_id_mut(tab_id)
                    .is_some_and(|session| session.try_load_pgn_from_text(&text, cx));
                if loaded {
                    app.refresh_analysis_if_engine_selected(tab_id, cx);
                }
                cx.notify();
            }
        })
        .detach();

        let mut session = AnalysisSession::new(id, board, api, pgn_input);
        session.refresh_board(cx);
        self.tabs.push(AppTab::GameAnalysis { id, session });
        self.active_tab = self.tabs.len() - 1;
        cx.notify();
    }

    pub(crate) fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index == 0 || self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(index);
        if let AppTab::OpeningTree { session, .. } = &tab {
            session
                .cancel_load
                .store(true, std::sync::atomic::Ordering::Relaxed);
            session.api.destroy(cx);
        }
        if let AppTab::GameAnalysis { session, .. } = &tab {
            session.api.destroy(cx);
        }
        if self.active_tab > index {
            self.active_tab -= 1;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }

    pub(crate) fn select_tab(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index >= self.tabs.len() {
            return;
        }
        self.active_tab = index;
        if let Some(username) = self
            .tabs
            .get(index)
            .and_then(|tab| tab.opening_tree())
            .map(|session| session.username.clone())
            && !username.is_empty()
        {
            self.username.update(cx, |input, cx| {
                input.set_value(username, window, cx);
            });
        }
        cx.notify();
    }

    pub(crate) fn on_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        match event.keystroke.key.as_str() {
            "left" => {
                if let Some(session) = self.active_opening_tree_mut() {
                    cx.stop_propagation();
                    let session_id = session.id;
                    session.go_back(cx);
                    self.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                    cx.notify();
                } else if let Some(session) = self.active_game_analysis_mut() {
                    cx.stop_propagation();
                    let tab_id = session.id;
                    session.go_back(cx);
                    self.refresh_analysis_if_engine_selected(tab_id, cx);
                    cx.notify();
                }
            }
            "right" => {
                if let Some(session) = self.active_opening_tree_mut() {
                    cx.stop_propagation();
                    let session_id = session.id;
                    session.go_forward_popular(cx);
                    self.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                    cx.notify();
                } else if let Some(session) = self.active_game_analysis_mut() {
                    cx.stop_propagation();
                    let tab_id = session.id;
                    session.go_forward(cx);
                    self.refresh_analysis_if_engine_selected(tab_id, cx);
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn stop_loading(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.active_opening_tree_mut() {
            session.stop_loading();
            cx.notify();
        }
    }

    pub(crate) fn refresh_engine_catalog(&mut self, cx: &mut Context<Self>) {
        self.catalog_loading = true;
        self.catalog_error = None;
        cx.notify();

        let entity = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { engine_catalog::resolve_catalog() })
                .await;

            entity.update(cx, |app, cx| {
                app.catalog_loading = false;
                match result {
                    Ok(offers) => {
                        app.catalog_offers = offers;
                        app.catalog_error = None;
                    }
                    Err(err) => {
                        app.catalog_error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn download_catalog_engine(&mut self, catalog_id: &str, cx: &mut Context<Self>) {
        let Some(offer) = self
            .catalog_offers
            .iter()
            .find(|offer| offer.engine.id == catalog_id)
            .cloned()
        else {
            self.engine_status = "Engine is not available for download".into();
            cx.notify();
            return;
        };

        self.downloading_catalog_id = Some(catalog_id.to_string());
        self.engine_status = format!("Downloading {}…", offer.engine.name).into();
        cx.notify();

        let entity = cx.entity();
        let engine_name = offer.engine.name.to_string();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { engine_catalog::install_catalog_engine(&offer) })
                .await;

            entity.update(cx, |app, cx| {
                app.downloading_catalog_id = None;
                match result {
                    Ok((name, path)) => match engines::register_engine(&mut app.engines, name, path)
                    {
                        Ok(()) => {
                            app.engine_status = format!("{engine_name} installed").into();
                        }
                        Err(err) if err == "already in list" => {
                            app.engine_status =
                                format!("{engine_name} is already in the loaded list").into();
                        }
                        Err(err) => app.engine_status = err.into(),
                    },
                    Err(err) => app.engine_status = err.into(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn pick_engine_file(&mut self, cx: &mut Context<Self>) {
        let entity = cx.entity();
        cx.spawn(async move |_, cx| {
            let picked = cx.background_executor().spawn(async {
                rfd::FileDialog::new()
                    .set_title("Select engine binary")
                    .pick_file()
            }).await;
            if let Some(path) = picked {
                entity.update(cx, |app, cx| {
                    match engines::add_engine(&mut app.engines, path) {
                        Ok(()) => app.engine_status = "Engine added".into(),
                        Err(err) if err == "already in list" => {
                            app.engine_status = "This engine is already in the list".into();
                        }
                        Err(err) => app.engine_status = err.into(),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(crate) fn remove_engine(&mut self, id: &str, cx: &mut Context<Self>) {
        self.disconnect_uci_on_remove(id);
        match engines::remove_engine(&mut self.engines, id) {
            Ok(()) => self.engine_status = "Engine removed".into(),
            Err(err) => self.engine_status = err.into(),
        }
        cx.notify();
    }
}
