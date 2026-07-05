use gpui::*;

use crate::repertoire::{self, list_repertoires, repertoire_display_name};
use crate::repertoire_session::RepertoireSession;

use super::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn repertoire_by_id_mut(&mut self, tab_id: u64) -> Option<&mut RepertoireSession> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.repertoire().is_some_and(|session| session.id == tab_id))
            .and_then(|tab| tab.repertoire_mut())
    }

    pub(crate) fn repertoire_at(&self, index: usize) -> Option<&RepertoireSession> {
        self.tabs.get(index)?.repertoire()
    }

    pub(crate) fn repertoire_at_mut(&mut self, index: usize) -> Option<&mut RepertoireSession> {
        self.tabs.get_mut(index)?.repertoire_mut()
    }

    pub(crate) fn active_repertoire(&self) -> Option<&RepertoireSession> {
        self.tabs.get(self.active_tab)?.repertoire()
    }

    pub(crate) fn active_repertoire_mut(&mut self) -> Option<&mut RepertoireSession> {
        self.tabs.get_mut(self.active_tab)?.repertoire_mut()
    }

    pub(crate) fn create_repertoire_from_ui(&mut self, cx: &mut Context<Self>) {
        let name = self
            .active_repertoire()
            .map(|session| session.name_input.read(cx).value().to_string())
            .unwrap_or_default();
        let name = name.trim().to_string();
        if name.is_empty() {
            if let Some(session) = self.active_repertoire_mut() {
                session.status = "Enter a repertoire name (e.g. caro)".into();
            }
            cx.notify();
            return;
        }

        match repertoire::create_repertoire(&name) {
            Ok(path) => {
                if let Some(session) = self.active_repertoire_mut() {
                    match session.load_from_path(path.clone(), cx) {
                        Ok(()) => {
                            session.label = repertoire_display_name(&path).into();
                            session.status = format!("Created {}", session.label).into();
                        }
                        Err(err) => session.status = err.into(),
                    }
                }
            }
            Err(err) => {
                if let Some(session) = self.active_repertoire_mut() {
                    session.status = err.into();
                }
            }
        }
        cx.notify();
    }

    pub(crate) fn save_active_repertoire(&mut self, cx: &mut Context<Self>) {
        let result = self
            .active_repertoire_mut()
            .map(|session| session.save_to_file());
        if let Some(Err(err)) = result {
            if let Some(session) = self.active_repertoire_mut() {
                session.status = err.into();
            }
        }
        cx.notify();
    }

    pub(crate) fn open_repertoire_path(
        &mut self,
        path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.active_repertoire_mut() {
            match session.load_from_path(path.clone(), cx) {
                Ok(()) => {
                    session.label = repertoire_display_name(&path).into();
                }
                Err(err) => session.status = err.into(),
            }
        }
        cx.notify();
    }

    pub(crate) fn list_repertoire_paths(&self) -> Vec<std::path::PathBuf> {
        list_repertoires()
    }
}
