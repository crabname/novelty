use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gpui::*;

use crate::engine_uci::{poll_uci_events, AnalysisRequest, UciEvent, UciSession};
use crate::panel_tabs::SidePanelTab;

use super::NoveltyApp;

#[derive(Clone, Copy, Debug)]
pub enum AnalysisTarget {
    GameAnalysis(u64),
    OpeningTree(u64),
}

#[derive(Clone, Debug)]
pub struct PendingConnectAnalysis {
    pub target: Option<AnalysisTarget>,
    pub engine_id: String,
    pub fen: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UciConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Analyzing,
    Error,
}

#[derive(Clone, Debug, Default)]
pub struct UciEngineState {
    pub status: UciConnectionStatus,
    pub identity: SharedString,
    pub last_result: SharedString,
}

impl NoveltyApp {
    pub(crate) fn uci_state(&self, engine_id: &str) -> UciEngineState {
        self.uci_states
            .get(engine_id)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn select_analysis_engine(
        &mut self,
        tab_id: u64,
        engine_id: &str,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.game_analysis_by_id_mut(tab_id) {
            session.engine.selected_engine_id = Some(engine_id.to_string());
        }
        self.analyze_game_position(tab_id, engine_id, cx);
    }

    pub(crate) fn select_opening_tree_engine(
        &mut self,
        session_id: u64,
        engine_id: &str,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.opening_tree_by_id_mut(session_id) {
            session.engine.selected_engine_id = Some(engine_id.to_string());
            session.side_panel_tab = SidePanelTab::Engine;
        }
        self.analyze_opening_tree_position(session_id, engine_id, cx);
    }

    pub(crate) fn refresh_analysis_if_engine_selected(
        &mut self,
        tab_id: u64,
        cx: &mut Context<Self>,
    ) {
        let engine_id = self
            .game_analysis_by_id_mut(tab_id)
            .and_then(|session| session.engine.selected_engine_id.clone());
        let Some(engine_id) = engine_id else {
            return;
        };
        self.analyze_game_position(tab_id, &engine_id, cx);
    }

    pub(crate) fn refresh_opening_tree_eval_if_engine_selected(
        &mut self,
        session_id: u64,
        cx: &mut Context<Self>,
    ) {
        let engine_id = self
            .opening_tree_by_id_mut(session_id)
            .and_then(|session| session.engine.selected_engine_id.clone());
        let Some(engine_id) = engine_id else {
            return;
        };
        self.analyze_opening_tree_position(session_id, &engine_id, cx);
    }

    pub(crate) fn connect_uci_engine(
        &mut self,
        engine_id: String,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        if self.uci_sessions.contains_key(&engine_id) {
            return;
        }
        if self
            .uci_states
            .get(&engine_id)
            .is_some_and(|state| state.status == UciConnectionStatus::Connecting)
        {
            return;
        }

        self.uci_states.insert(
            engine_id.clone(),
            UciEngineState {
                status: UciConnectionStatus::Connecting,
                ..Default::default()
            },
        );
        self.engine_status = "Connecting to UCI engine…".into();
        cx.notify();

        let (event_tx, event_rx) = mpsc::channel();
        let entity = cx.entity();
        let entity_for_poll = entity.clone();
        let engine_id_for_poll = engine_id.clone();

        cx.spawn(async move |_, cx| {
            let (done_tx, done_rx) = mpsc::channel();
            thread::spawn(move || {
                let _ = done_tx.send(UciSession::connect(path, event_tx));
            });

            loop {
                match done_rx.try_recv() {
                    Ok(connect_result) => {
                        entity.update(cx, |app, cx| {
                            match connect_result {
                                Ok((session, name)) => {
                                    app.uci_sessions.insert(engine_id.clone(), session);
                                    app.uci_states.insert(
                                        engine_id.clone(),
                                        UciEngineState {
                                            status: UciConnectionStatus::Connected,
                                            identity: name.clone().into(),
                                            last_result: "Connected".into(),
                                        },
                                    );
                                    app.engine_status = format!("Connected to {name}").into();

                                    if let Some(pending) = app.pending_analysis.take()
                                        && pending.engine_id == engine_id
                                        && !pending.fen.is_empty()
                                    {
                                        app.dispatch_uci_analysis(
                                            pending.target,
                                            &engine_id,
                                            pending.fen,
                                            cx,
                                        );
                                    }

                                    poll_uci_events(
                                        entity_for_poll,
                                        engine_id_for_poll,
                                        event_rx,
                                        cx,
                                    );
                                }
                                Err(err) => {
                                    app.uci_states.insert(
                                        engine_id.clone(),
                                        UciEngineState {
                                            status: UciConnectionStatus::Error,
                                            last_result: err.clone().into(),
                                            ..Default::default()
                                        },
                                    );
                                    app.engine_status = err.clone().into();
                                    app.fail_pending_analysis(&err, cx);
                                }
                            }
                            cx.notify();
                        });
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        cx.background_executor()
                            .timer(Duration::from_millis(50))
                            .await;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        entity.update(cx, |app, cx| {
                            app.uci_states.insert(
                                engine_id.clone(),
                                UciEngineState {
                                    status: UciConnectionStatus::Error,
                                    last_result: "Engine connection failed".into(),
                                    ..Default::default()
                                },
                            );
                            app.engine_status = "Engine connection failed".into();
                            cx.notify();
                        });
                        break;
                    }
                }
            }
        })
        .detach();
    }

    pub(crate) fn disconnect_uci_engine(&mut self, engine_id: &str, cx: &mut Context<Self>) {
        if let Some(session) = self.uci_sessions.remove(engine_id) {
            session.disconnect();
        }
        self.uci_states.remove(engine_id);
        self.engine_status = "Engine disconnected".into();
        cx.notify();
    }

    pub(crate) fn analyze_with_uci_engine(&mut self, engine_id: &str, cx: &mut Context<Self>) {
        let fen = crate::graph::start_fen();
        self.run_uci_analysis(None, engine_id, fen, cx);
    }

    pub(crate) fn analyze_game_position(
        &mut self,
        tab_id: u64,
        engine_id: &str,
        cx: &mut Context<Self>,
    ) {
        let fen = self
            .tabs
            .iter()
            .find(|tab| tab.id() == tab_id)
            .and_then(|tab| tab.game_analysis())
            .map(|session| session.current_fen.clone());

        let Some(fen) = fen else {
            return;
        };

        let engine_id = engine_id.to_string();
        if let Some(session) = self.game_analysis_by_id_mut(tab_id) {
            session.engine.selected_engine_id = Some(engine_id.clone());
            session.set_analysis_pending(cx);
        }

        self.run_uci_analysis(
            Some(AnalysisTarget::GameAnalysis(tab_id)),
            &engine_id,
            fen,
            cx,
        );
    }

    pub(crate) fn analyze_opening_tree_position(
        &mut self,
        session_id: u64,
        engine_id: &str,
        cx: &mut Context<Self>,
    ) {
        let fen = self
            .opening_tree_by_id_mut(session_id)
            .map(|session| session.current_fen.clone());

        let Some(fen) = fen else {
            return;
        };

        let engine_id = engine_id.to_string();
        if let Some(session) = self.opening_tree_by_id_mut(session_id) {
            session.engine.selected_engine_id = Some(engine_id.clone());
            session.set_eval_pending(cx);
        }

        self.run_uci_analysis(
            Some(AnalysisTarget::OpeningTree(session_id)),
            &engine_id,
            fen,
            cx,
        );
    }

    fn run_uci_analysis(
        &mut self,
        target: Option<AnalysisTarget>,
        engine_id: &str,
        fen: String,
        cx: &mut Context<Self>,
    ) {
        if self.uci_sessions.contains_key(engine_id) {
            self.dispatch_uci_analysis(target, engine_id, fen, cx);
            return;
        }

        let Some(engine) = self.engines.iter().find(|e| e.id == engine_id) else {
            let message = "Engine not found — add one in the Engine tab".to_string();
            self.fail_analysis_target(target, &message, cx);
            self.engine_status = message.into();
            cx.notify();
            return;
        };

        self.pending_analysis = Some(PendingConnectAnalysis {
            target,
            engine_id: engine_id.to_string(),
            fen,
        });

        self.connect_uci_engine(engine_id.to_string(), PathBuf::from(&engine.path), cx);
    }

    fn dispatch_uci_analysis(
        &mut self,
        target: Option<AnalysisTarget>,
        engine_id: &str,
        fen: String,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.uci_sessions.get(engine_id) else {
            let message = "Connect to the engine first".to_string();
            self.fail_analysis_target(target, &message, cx);
            self.engine_status = message.into();
            cx.notify();
            return;
        };

        let (depth, line_count) = self.analysis_settings_for_target(target);

        if let Some(state) = self.uci_states.get_mut(engine_id) {
            state.status = UciConnectionStatus::Analyzing;
            state.last_result = "Analyzing position…".into();
        }
        self.engine_status = format!("Analyzing (depth {}, {} lines)…", depth, line_count).into();

        self.pending_analysis = Some(PendingConnectAnalysis {
            target,
            engine_id: engine_id.to_string(),
            fen: String::new(),
        });

        let request = AnalysisRequest {
            fen,
            depth,
            line_count,
        };

        if let Err(err) = session.analyze(request) {
            self.fail_analysis_target(target, &err, cx);
            if let Some(state) = self.uci_states.get_mut(engine_id) {
                state.status = UciConnectionStatus::Error;
                state.last_result = err.clone().into();
            }
            self.engine_status = err.into();
        }
        cx.notify();
    }

    fn analysis_settings_for_target(&self, target: Option<AnalysisTarget>) -> (u32, u32) {
        match target {
            Some(AnalysisTarget::GameAnalysis(tab_id)) => self
                .tabs
                .iter()
                .find(|tab| tab.id() == tab_id)
                .and_then(|tab| tab.game_analysis())
                .map(|session| {
                    (
                        session.engine.settings.depth,
                        session.engine.settings.line_count,
                    )
                })
                .unwrap_or((16, 3)),
            Some(AnalysisTarget::OpeningTree(session_id)) => self
                .opening_tree_by_id(session_id)
                .map(|session| {
                    (
                        session.engine.settings.depth,
                        session.engine.settings.line_count,
                    )
                })
                .unwrap_or((14, 3)),
            None => (16, 1),
        }
    }

    pub(crate) fn handle_uci_event(&mut self, engine_id: &str, event: UciEvent, cx: &mut Context<Self>) {
        match event {
            UciEvent::Analysis(result) => {
                let summary = result.summary();
                if let Some(state) = self.uci_states.get_mut(engine_id) {
                    state.status = UciConnectionStatus::Connected;
                    state.last_result = summary.clone().into();
                }
                self.engine_status = summary.into();

                if let Some(pending) = self.pending_analysis.take()
                    && pending.engine_id == engine_id
                    && let Some(target) = pending.target
                {
                    match target {
                        AnalysisTarget::GameAnalysis(tab_id) => {
                            if let Some(session) = self.game_analysis_by_id_mut(tab_id) {
                                session.apply_analysis(&result, cx);
                                session.status = format!(
                                    "Depth {} · {} engine lines",
                                    result.depth,
                                    result.lines.len()
                                )
                                .into();
                            }
                        }
                        AnalysisTarget::OpeningTree(session_id) => {
                            if let Some(session) = self.opening_tree_by_id_mut(session_id) {
                                session.apply_engine_analysis(&result, cx);
                            }
                        }
                    }
                }
            }
            UciEvent::Error(message) => {
                if let Some(state) = self.uci_states.get_mut(engine_id) {
                    state.status = UciConnectionStatus::Error;
                    state.last_result = message.clone().into();
                }
                self.engine_status = message.clone().into();
                self.fail_pending_analysis(&message, cx);
            }
            UciEvent::Disconnected => {
                self.uci_sessions.remove(engine_id);
                self.uci_states.remove(engine_id);
                self.engine_status = "Engine disconnected".into();
                self.pending_analysis = None;
            }
        }
        cx.notify();
    }

    pub(crate) fn handle_uci_poll_closed(&mut self, engine_id: &str, cx: &mut Context<Self>) {
        if self
            .uci_states
            .get(engine_id)
            .is_some_and(|state| state.status == UciConnectionStatus::Connecting)
        {
            self.uci_states.insert(
                engine_id.to_string(),
                UciEngineState {
                    status: UciConnectionStatus::Error,
                    last_result: "Engine connection failed unexpectedly".into(),
                    ..Default::default()
                },
            );
            self.engine_status = "Engine connection failed unexpectedly".into();
            self.fail_pending_analysis("Engine connection failed unexpectedly", cx);
            cx.notify();
        }
    }

    pub(crate) fn disconnect_uci_on_remove(&mut self, engine_id: &str) {
        if let Some(session) = self.uci_sessions.remove(engine_id) {
            session.disconnect();
        }
        self.uci_states.remove(engine_id);
    }

    fn fail_pending_analysis(&mut self, message: &str, cx: &mut Context<Self>) {
        if let Some(pending) = self.pending_analysis.take() {
            self.fail_analysis_target(pending.target, message, cx);
        }
    }

    fn fail_analysis_target(
        &mut self,
        target: Option<AnalysisTarget>,
        message: &str,
        cx: &mut Context<Self>,
    ) {
        match target {
            Some(AnalysisTarget::GameAnalysis(tab_id)) => {
                if let Some(session) = self.game_analysis_by_id_mut(tab_id) {
                    session.set_analysis_error(message.to_string());
                    session.refresh_board(cx);
                }
            }
            Some(AnalysisTarget::OpeningTree(session_id)) => {
                if let Some(session) = self.opening_tree_by_id_mut(session_id) {
                    session.set_engine_analysis_error();
                    session.refresh_board(cx);
                }
            }
            None => {}
        }
        self.pending_analysis = None;
    }
}
