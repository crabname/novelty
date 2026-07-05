mod history;
mod import;
mod sidebar;

use gpui::*;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::panel_tabs::SidePanelTab;

impl NoveltyApp {
    pub(crate) fn render_repertoire(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if let Some(session) = self.active_repertoire_mut() {
            session.flush_pgn_ui_if_needed(window, cx);
        }

        let active = self.active_tab;

        h_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .items_stretch()
            .child(self.render_repertoire_sidebar(window, cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .items_stretch()
                    .child(self.render_repertoire_history(active, cx))
                    .child(
                        div()
                            .id(SharedString::from(format!("repertoire-board-{active}")))
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_hidden()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.focus_handle.focus(window, cx);
                                }),
                            )
                            .child(self.render_repertoire_board(active)),
                    )
                    .child(self.render_repertoire_panel(active, cx)),
            )
    }

    fn render_repertoire_panel(
        &mut self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if let Some(session) = self.repertoire_at(session_index) {
            self.refresh_explorer_if_needed(session.id, cx);
        }
        let tab_id = self
            .repertoire_at(session_index)
            .map(|session| session.id)
            .unwrap_or(0);
        let tabs = SidePanelTab::REPERTOIRE;
        let selected_index = self
            .repertoire_at(session_index)
            .map(|session| session.side_panel_tab.index_in(tabs))
            .unwrap_or(0);

        let content = self
            .render_explorer_moves_table(session_index, cx)
            .into_any_element();

        let panel_id = SharedString::from(format!("repertoire-panel-{tab_id}"));

        self.render_right_panel(
            panel_id,
            tabs,
            selected_index,
            move |this, index, cx| {
                let tab_id = this
                    .repertoire_at(session_index)
                    .map(|session| session.id)
                    .unwrap_or(0);
                if let Some(session) = this.repertoire_at_mut(session_index) {
                    session.side_panel_tab = SidePanelTab::from_index(tabs, index);
                }
                this.refresh_explorer_if_needed(tab_id, cx);
                cx.notify();
            },
            content,
            cx,
        )
    }
}
