//! Fetch player games and merge lines into the active repertoire.

use std::sync::atomic::Ordering;
use std::time::Duration;

use gpui::*;

use crate::constants::UI_TICK_MS;
use crate::fetch::{LoadedGame, StreamGamesRequest};
use crate::repertoire::set_linked_profile;
use crate::repertoire_import::{
    classify_lines, extract_continuations, merge_continuations, ExtractOptions,
};

use super::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn import_repertoire_from_profile(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.import_repertoire_lines(cx);
    }

    fn import_repertoire_lines(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.active_repertoire() else {
            return;
        };
        if session.import_loading {
            return;
        }
        if session.file_path.is_none() {
            if let Some(session) = self.active_repertoire_mut() {
                session.status = "Create or open a repertoire file first".into();
            }
            cx.notify();
            return;
        }

        let tab_id = session.id;
        let username = session.profile_input.read(cx).value().trim().to_string();
        if username.is_empty() {
            if let Some(session) = self.repertoire_by_id_mut(tab_id) {
                session.status = "Enter a Lichess / Chess.com username".into();
            }
            cx.notify();
            return;
        }

        let site = session.import_site;
        let color = session.player_color();
        let anchor_fen = session.tree.current().fen.clone();
        let anchor_path = session.tree.position.clone();
        let depth = session.import_depth as usize;
        let cancel = session.cancel_import.clone();

        let period = self.selected_period(cx);
        if !self.time_controls.any_selected() {
            if let Some(session) = self.repertoire_by_id_mut(tab_id) {
                session.status =
                    "Select at least one time control below (Bullet, Blitz, …)".into();
            }
            cx.notify();
            return;
        }

        let time_controls = self.time_controls;
        let lichess_token = self.lichess_token_for_load(&username);
        let period_label = period.loading_label();

        if let Some(session) = self.repertoire_by_id_mut(tab_id) {
            set_linked_profile(&mut session.headers, &username, site, color);
            session.import_loading = true;
            session.games_loaded = 0;
            session.cancel_import.store(false, Ordering::Relaxed);
            session.status = format!(
                "Loading {username} as {} at current position ({period_label})…",
                color.orientation_value()
            )
            .into();
        }

        let entity = cx.entity();
        self.pending_profile_remember = Some(username.clone());

        cx.spawn(async move |_this, cx| {
            use std::sync::mpsc;

            let (done_tx, done_rx) = mpsc::channel::<(Result<(crate::fetch::StreamOutcome, u32), String>, Vec<LoadedGame>)>();
            let username_bg = username.clone();
            let lichess_token_bg = lichess_token.clone();
            let cancel_bg = cancel.clone();

            std::thread::spawn(move || {
                let mut games: Vec<LoadedGame> = Vec::new();
                let result = crate::fetch::stream_games(
                    StreamGamesRequest {
                        site,
                        username: &username_bg,
                        color,
                        period,
                        time_controls,
                        lichess_token: lichess_token_bg.as_deref(),
                        cancel: &cancel_bg,
                    },
                    |game| {
                        games.push(game);
                        Ok(())
                    },
                );
                let _ = done_tx.send((result, games));
            });

            loop {
                if let Ok((fetch_result, games)) = done_rx.try_recv() {
                    entity.update(cx, |app, cx| {
                        app.finish_repertoire_import(
                            tab_id,
                            fetch_result,
                            games,
                            anchor_fen,
                            anchor_path,
                            depth,
                            cx,
                        );
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

    fn finish_repertoire_import(
        &mut self,
        tab_id: u64,
        fetch_result: Result<(crate::fetch::StreamOutcome, u32), String>,
        games: Vec<LoadedGame>,
        anchor_fen: String,
        anchor_path: Vec<usize>,
        depth: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.repertoire_by_id_mut(tab_id) else {
            return;
        };
        session.import_loading = false;
        session.games_loaded = games.len() as u32;
        session.cancel_import.store(false, Ordering::Relaxed);

        if let Err(err) = &fetch_result {
            session.status = err.clone().into();
            cx.notify();
            return;
        }

        let (outcome, _) = fetch_result.unwrap_or((crate::fetch::StreamOutcome::Completed, 0));
        if games.is_empty() {
            let side = session.player_color();
            session.status = format!(
                "No games as {} in this period — try a longer period or check repertoire color",
                side.orientation_value()
            )
            .into();
            cx.notify();
            return;
        }

        let mut lines = extract_continuations(
            &games,
            &ExtractOptions {
                anchor_fen: anchor_fen.clone(),
                depth,
                max_lines: 50,
                min_games: 1,
            },
        );

        if lines.is_empty() {
            session.status = format!(
                "Loaded {} games at {} — no games contain this position",
                games.len(),
                session.opening_label()
            )
            .into();
            cx.notify();
            return;
        }

        classify_lines(&session.tree, &anchor_path, &mut lines);
        let report = merge_continuations(&mut session.tree, &anchor_path, &lines);

        session.dirty = true;
        session.needs_pgn_ui_sync = true;
        session.last_parsed_pgn = session.current_pgn();
        session.sync_board_from_tree(cx);
        if session.file_path.is_some() {
            if let Err(err) = session.save_to_file() {
                session.status = err.into();
                cx.notify();
                return;
            }
        }

        let position_label = session.opening_label().to_string();
        if report.lines_added == 0 && report.plies_added == 0 {
            session.status = format!(
                "Loaded {} games at {position_label} — all {} candidate lines already in repertoire",
                games.len(),
                lines.len()
            )
            .into();
        } else if outcome == crate::fetch::StreamOutcome::Cancelled {
            session.status = format!(
                "Stopped at {position_label} — added {} lines ({} plies) from {} games",
                report.lines_added,
                report.plies_added,
                games.len(),
            )
            .into();
        } else {
            session.status = format!(
                "Imported {} lines ({} plies) from {} games at {position_label}",
                report.lines_added,
                report.plies_added,
                games.len(),
            )
            .into();
        }
        cx.notify();
    }
}
