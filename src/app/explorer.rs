use gpui::*;

use crate::opening_explorer::{fetch_lichess_explorer, ExplorerHost};

use super::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn refresh_explorer_if_needed(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let (fen, request_id) = if let Some(session) = self.game_analysis_by_id_mut(tab_id) {
            if !session.explorer_state().needs_fetch(session.explorer_fen()) {
                return;
            }
            let fen = session.explorer_fen().to_string();
            let request_id = session.explorer_state().request_id.saturating_add(1);
            session.explorer_state_mut().begin_fetch(request_id);
            (fen, request_id)
        } else if let Some(session) = self.repertoire_by_id_mut(tab_id) {
            if !session.explorer_state().needs_fetch(session.explorer_fen()) {
                return;
            }
            let fen = session.explorer_fen().to_string();
            let request_id = session.explorer_state().request_id.saturating_add(1);
            session.explorer_state_mut().begin_fetch(request_id);
            (fen, request_id)
        } else {
            return;
        };

        let token = self
            .lichess_session
            .as_ref()
            .map(|session| session.access_token.clone());
        let entity = cx.entity();

        if token.is_none() {
            self.apply_explorer_result(tab_id, request_id, fen, Err(
                "Log in to Lichess in Settings to use Opening Explorer.".into(),
            ));
            cx.notify();
            return;
        }

        cx.spawn(async move |_this, cx| {
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = fetch_lichess_explorer(&fen, token.as_deref());
                let _ = result_tx.send((request_id, fen, result));
            });

            loop {
                if let Ok((request_id, fen, result)) = result_rx.try_recv() {
                    entity.update(cx, |app, cx| {
                        app.apply_explorer_result(tab_id, request_id, fen, result);
                        cx.notify();
                    });
                    break;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(50))
                    .await;
            }
        })
        .detach();
        cx.notify();
    }

    fn apply_explorer_result(
        &mut self,
        tab_id: u64,
        request_id: u64,
        fen: String,
        result: Result<Vec<crate::opening_explorer::ExplorerMove>, String>,
    ) {
        if let Some(session) = self.game_analysis_by_id_mut(tab_id) {
            match result {
                Ok(moves) => session.explorer_state_mut().apply_moves(request_id, fen, moves),
                Err(err) => session.explorer_state_mut().set_error(request_id, err),
            }
            return;
        }
        if let Some(session) = self.repertoire_by_id_mut(tab_id) {
            match result {
                Ok(moves) => session.explorer_state_mut().apply_moves(request_id, fen, moves),
                Err(err) => session.explorer_state_mut().set_error(request_id, err),
            }
        }
    }

    pub(crate) fn play_explorer_move(&mut self, tab_id: u64, san: &str, cx: &mut Context<Self>) {
        if let Some(session) = self.game_analysis_by_id_mut(tab_id) {
            session.play_explorer_san(san, cx);
            self.refresh_analysis_if_engine_selected(tab_id, cx);
            self.refresh_explorer_if_needed(tab_id, cx);
            return;
        }
        if let Some(session) = self.repertoire_by_id_mut(tab_id) {
            session.play_explorer_san(san, cx);
            self.refresh_explorer_if_needed(tab_id, cx);
        }
    }

    pub(crate) fn explorer_table_context(
        &self,
        session_index: usize,
    ) -> Option<(&crate::opening_explorer::ExplorerState, u64)> {
        self.game_analysis_at(session_index)
            .map(|session| (&session.explorer, session.id))
            .or_else(|| {
                self.repertoire_at(session_index)
                    .map(|session| (&session.explorer, session.id))
            })
    }
}
