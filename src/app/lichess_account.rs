use gpui::*;

use crate::lichess::{self, LichessSession};

use super::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn lichess_logged_in(&self) -> bool {
        self.lichess_session.is_some()
    }

    pub(crate) fn lichess_username(&self) -> Option<String> {
        self.lichess_session.as_ref().map(|s| s.username.clone())
    }

    pub(crate) fn lichess_auth_status(&self) -> SharedString {
        self.lichess_auth_status.clone()
    }

    pub(crate) fn lichess_token_for_load(&self, username: &str) -> Option<String> {
        self.lichess_session
            .as_ref()
            .and_then(|session| lichess::token_for_username(session, username))
            .map(str::to_string)
    }

    pub(crate) fn logout_lichess(&mut self, cx: &mut Context<Self>) {
        if lichess::clear_session().is_ok() {
            self.lichess_session = None;
            self.lichess_auth_status = "Logged out of Lichess".into();
        } else {
            self.lichess_auth_status = "Failed to clear Lichess session".into();
        }
        cx.notify();
    }

    pub(crate) fn login_lichess_browser(&mut self, cx: &mut Context<Self>) {
        let username = self.username.read(cx).value().to_string();
        if username.trim().is_empty() {
            self.lichess_auth_status = "Enter your Lichess username first".into();
            cx.notify();
            return;
        }

        self.lichess_auth_status = "Waiting for Lichess login in browser…".into();
        cx.notify();

        let entity = cx.entity();
        let username = username.trim().to_string();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { lichess::authenticate_in_browser(&username) })
                .await;

            entity.update(cx, |app, cx| {
                match result {
                    Ok(session) => {
                        app.apply_lichess_session(session);
                        app.lichess_auth_status = format!(
                            "Connected to Lichess as {}",
                            app.lichess_session.as_ref().unwrap().username
                        )
                        .into();
                    }
                    Err(err) => {
                        app.lichess_auth_status = err.into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_lichess_session(&mut self, session: LichessSession) {
        self.pending_username = Some(session.username.clone());
        self.lichess_session = Some(session);
    }
}
