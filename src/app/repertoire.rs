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

    pub(crate) fn create_repertoire_from_ui(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

        let create_color = self
            .active_repertoire()
            .map(|session| session.create_color)
            .unwrap_or(crate::fetch::PlayerColor::White);

        match repertoire::create_repertoire(&name, create_color) {
            Ok(path) => {
                if let Some(session) = self.active_repertoire_mut() {
                    match session.load_from_path(path.clone(), window, cx) {
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
        if let Some(Err(err)) = result
            && let Some(session) = self.active_repertoire_mut()
        {
            session.status = err.into();
        }
        cx.notify();
    }

    pub(crate) fn open_repertoire_path(
        &mut self,
        path: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.active_repertoire_mut() {
            match session.load_from_path(path.clone(), window, cx) {
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

    pub(crate) fn repertoire_tree_promote_at(
        &mut self,
        session_index: usize,
        position: Vec<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self
            .repertoire_at(session_index)
            .map(|session| session.id)
            .unwrap_or(0);
        let changed = if let Some(session) = self.repertoire_at_mut(session_index) {
            session.go_to_position(position.clone(), cx);
            session.promote_variation_at(position, cx)
        } else {
            false
        };
        if changed {
            self.repertoire_save_after_tree_edit(session_index, window, cx);
            self.refresh_explorer_if_needed(tab_id, cx);
        }
        cx.notify();
    }

    pub(crate) fn repertoire_tree_delete_at(
        &mut self,
        session_index: usize,
        position: Vec<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self
            .repertoire_at(session_index)
            .map(|session| session.id)
            .unwrap_or(0);
        let changed = self
            .repertoire_at_mut(session_index)
            .is_some_and(|session| session.delete_line_at(position, cx));
        if changed {
            self.repertoire_save_after_tree_edit(session_index, window, cx);
            self.refresh_explorer_if_needed(tab_id, cx);
        }
        cx.notify();
    }

    fn repertoire_save_after_tree_edit(
        &mut self,
        session_index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.repertoire_at_mut(session_index) {
            if session.file_path.is_some() {
                let _ = session.save_to_file();
            }
            session.flush_pgn_ui_if_needed(window, cx);
        }
    }

    pub(crate) fn toggle_repertoire_variation_group(
        &mut self,
        session_index: usize,
        branch_path: Vec<usize>,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.repertoire_at_mut(session_index) {
            let key = crate::repertoire_session::variation_group_key(&branch_path);
            if session.collapsed_variation_groups.contains(&key) {
                session.collapsed_variation_groups.remove(&key);
            } else {
                session.collapsed_variation_groups.insert(key);
            }
        }
        cx.notify();
    }
}
