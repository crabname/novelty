mod analysis;
mod controls;
mod engine_panel;
mod engine_pick;
mod engines;
mod history;
mod home;
mod next_moves;
mod repertoire;
mod settings;
mod sidebar;
mod stub;
mod tabs;

use gpui::*;
use gpui_component::TitleBar;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::tab::AppTab;

impl Render for NoveltyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_focus {
            self.needs_focus = false;
            self.focus_handle.focus(window, cx);
        }
        if let Some(name) = self.pending_username.take() {
            self.username.update(cx, |input, cx| {
                input.set_value(name, window, cx);
            });
        }
        if let Some(name) = self.pending_profile_remember.take() {
            self.remember_loaded_profile(&name, window, cx);
        }

        v_flex()
            .id("novelty-root")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, _, cx| {
                this.on_key_down(event, cx);
            }))
            .size_full()
            .overflow_hidden()
            .child(
                TitleBar::new().child(
                    div()
                        .text_sm()
                        .child("Novelty"),
                ),
            )
            .child(self.render_app_tabs(cx))
            .child(
                div()
                    .id("novelty-tab-content")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(self.render_active_tab(window, cx)),
            )
    }
}

impl NoveltyApp {
    fn render_active_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        match &self.tabs[self.active_tab] {
            AppTab::Home { .. } => self.render_home(cx).into_any_element(),
            AppTab::OpeningTree { .. } => self.render_opening_tree(window, cx).into_any_element(),
            AppTab::GameAnalysis { .. } => self.render_game_analysis(window, cx).into_any_element(),
            AppTab::Repertoire { .. } => self.render_repertoire(window, cx).into_any_element(),
            AppTab::Engines { .. } => self.render_engines(cx).into_any_element(),
            AppTab::Settings { .. } => self.render_settings(cx).into_any_element(),
            AppTab::Stub { kind, .. } => self.render_stub(*kind, cx).into_any_element(),
        }
    }

    fn render_opening_tree(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.active_tab;
        let shapes_preview = self
            .opening_tree_at(active)
            .map(|session| session.next_move_count)
            .unwrap_or(0);

        h_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .items_stretch()
            .child(self.render_sidebar(window, cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .items_stretch()
                    .child(self.render_history_column(active, cx))
                    .child(
                        div()
                            .id("novelty-board-column")
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.focus_handle.focus(window, cx);
                                }),
                            )
                            .child(self.render_board(active, cx, shapes_preview)),
                    )
                    .child(self.render_controls_panel(active, cx)),
            )
    }
}
