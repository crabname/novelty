use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::*;

use crate::app::NoveltyApp;

impl NoveltyApp {
    pub(super) fn render_repertoire_board(&self, tab_index: usize) -> impl IntoElement {
        let board = self
            .repertoire_at(tab_index)
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

    pub(super) fn render_repertoire_history(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let scroll_handle = self
            .repertoire_at(session_index)
            .map(|session| session.tree_scroll_handle.clone())
            .unwrap_or_default();
        let rows = self.repertoire_variation_tree_rows(session_index, cx);

        v_flex()
            .flex_shrink_0()
            .w(px(260.))
            .h_full()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .px_2()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted.opacity(0.35))
                    .child(
                        Label::new("Variations")
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "repertoire-tree-scroll-{session_index}"
                            )))
                            .size_full()
                            .flex()
                            .flex_col()
                            .gap_y(px(4.))
                            .px_2()
                            .py_2()
                            .overflow_y_scroll()
                            .track_scroll(&scroll_handle)
                            .children(rows),
                    )
                    .vertical_scrollbar(&scroll_handle),
            )
    }
}
