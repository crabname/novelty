mod history;
mod panel;
mod sidebar;

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::*;

use crate::app::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn render_game_analysis(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.active_tab;

        h_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .items_stretch()
            .child(self.render_analysis_sidebar(window, cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .items_stretch()
                    .child(self.render_analysis_history(active, cx))
                    .child(
                        div()
                            .id(SharedString::from(format!("analysis-board-{active}")))
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
                            .child(self.render_analysis_board(active, cx)),
                    )
                    .child(self.render_analysis_panel(active, cx)),
            )
    }

    pub(super) fn render_analysis_board(&self, tab_index: usize, _cx: &App) -> impl IntoElement {
        let board = self
            .game_analysis_at(tab_index)
            .map(|session| session.board.clone());

        div()
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .when_some(board, |this, board| this.child(board))
    }
}
