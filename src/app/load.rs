use std::sync::Arc;

use gpui::*;

use crate::constants::{UI_PROGRESS_INTERVAL, UI_TICK_MS};
use crate::fetch::{LoadedGame, StreamOutcome};

use super::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn load_games(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self
            .active_opening_tree()
            .is_some_and(|session| session.loading)
        {
            return;
        }
        let username = self.username.read(cx).value().to_string();
        if username.trim().is_empty() {
            if let Some(session) = self.active_opening_tree_mut() {
                session.status = "Username required".into();
            }
            cx.notify();
            return;
        }
        if !self.time_controls.any_selected() {
            if let Some(session) = self.active_opening_tree_mut() {
                session.status = "Select at least one time control".into();
            }
            cx.notify();
            return;
        }

        let period = self.selected_period(cx);
        let trimmed = username.trim().to_string();
        self.pending_profile_remember = Some(trimmed.clone());

        let session_id = self
            .active_opening_tree()
            .expect("opening tree tab")
            .id;
        let site = self.site;
        let color = self.color;
        let time_controls = self.time_controls;
        let session = self.active_opening_tree_mut().expect("opening tree tab");
        session.reset_for_load(trimmed.clone(), cx);
        session.status = format!(
            "Loading {username} ({})…",
            period.loading_label()
        )
        .into();

        let graph = session.graph.clone();
        let cancel = session.cancel_load.clone();
        let entity = cx.entity();
        let display_user = trimmed.clone();

        cx.spawn(async move |_this, cx| {
            use std::sync::atomic::{AtomicU32, Ordering};
            use std::sync::mpsc;
            use std::time::Duration;

            let (progress_tx, progress_rx) = mpsc::channel::<(u32, u32)>();
            let (done_tx, done_rx) = mpsc::channel::<(Result<(StreamOutcome, u32), String>, u32)>();
            let graph_bg = graph.clone();
            let errors = Arc::new(AtomicU32::new(0));
            let fetch_user = trimmed.clone();
            let fetch_user_thread = fetch_user.clone();
            let cancel_bg = cancel.clone();

            std::thread::spawn(move || {
                let result = crate::fetch::stream_games(
                    site,
                    &fetch_user_thread,
                    color,
                    period,
                    time_controls,
                    &cancel_bg,
                    |game| {
                        let mut g = graph_bg.lock().expect("graph lock");
                        let ingest = match &game {
                            LoadedGame::Moves {
                                moves,
                                notation,
                                meta,
                            } => g.add_game(moves, *notation, Some(meta), color),
                            LoadedGame::Pgn { pgn, meta } => {
                                g.add_game_pgn(pgn, meta, color)
                            }
                        };
                        let loaded = g.game_count();
                        if ingest.is_err() {
                            errors.fetch_add(1, Ordering::Relaxed);
                        }
                        let skipped = errors.load(Ordering::Relaxed);
                        drop(g);
                        if loaded.is_multiple_of(UI_PROGRESS_INTERVAL) {
                            let _ = progress_tx.send((loaded, skipped));
                        }
                        Ok(())
                    },
                );
                let skipped = errors.load(Ordering::Relaxed);
                let loaded = graph_bg.lock().expect("graph lock").game_count();
                let _ = progress_tx.send((loaded, skipped));
                let _ = done_tx.send((result, skipped));
            });

            let period_label = period.loading_label();
            let mut last_board_refresh = 0u32;
            loop {
                let mut latest = None;
                while let Ok(progress) = progress_rx.try_recv() {
                    latest = Some(progress);
                }

                if let Some((loaded, skipped)) = latest {
                    let refresh_board =
                        loaded.saturating_sub(last_board_refresh) >= UI_PROGRESS_INTERVAL;
                    let fetch_user = fetch_user.clone();
                    let period_label = period_label.clone();
                    entity.update(cx, |app, cx| {
                        let Some(tab) = app.tab_by_id_mut(session_id) else {
                            return;
                        };
                        let Some(session) = tab.opening_tree_mut() else {
                            return;
                        };
                        session.status = format!(
                            "{fetch_user} — loading ({period_label})… {loaded} games ({skipped} skipped)"
                        )
                        .into();
                        if refresh_board {
                            session.refresh_board(cx);
                            last_board_refresh = loaded;
                        }
                        cx.notify();
                    });
                }

                if let Ok((stream_result, skipped)) = done_rx.try_recv() {
                    entity.update(cx, |app, cx| {
                        let Some(tab) = app.tab_by_id_mut(session_id) else {
                            return;
                        };
                        let Some(session) = tab.opening_tree_mut() else {
                            return;
                        };
                        session.loading = false;
                        session.cancel_load.store(false, Ordering::Relaxed);
                        let loaded = session.game_count();
                        match stream_result {
                            Ok((StreamOutcome::Completed, _)) => {
                                session.status = format!(
                                    "{display_user} — {loaded} games loaded ({skipped} skipped)"
                                )
                                .into();
                                session.refresh_board(cx);
                            }
                            Ok((StreamOutcome::Cancelled, _)) => {
                                session.status = format!(
                                    "{display_user} — stopped at {loaded} games ({skipped} skipped)"
                                )
                                .into();
                                session.refresh_board(cx);
                            }
                            Err(err) if loaded > 0 => {
                                session.status =
                                    format!("{err} — kept {loaded} games ({skipped} skipped)").into();
                                session.refresh_board(cx);
                            }
                            Err(err) => {
                                session.status = err.into();
                            }
                        }
                        cx.notify();
                    });
                    break;
                }

                cx.background_executor()
                    .timer(Duration::from_millis(UI_TICK_MS))
                    .await;
            }
        })
        .detach();
        cx.notify();
    }
}
