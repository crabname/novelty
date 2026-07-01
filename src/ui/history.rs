use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::scroll::ScrollableElement;
use gpui_component::*;

use crate::app::NoveltyApp;

impl NoveltyApp {
    fn history_cell(
        &self,
        cx: &mut Context<Self>,
        session_index: usize,
        index: usize,
        label: SharedString,
    ) -> impl IntoElement {
        let selected = self
            .opening_tree_at(session_index)
            .is_some_and(|session| session.history_index == index);
        div()
            .id(SharedString::from(format!("hist-{session_index}-{index}")))
            .min_w(px(44.))
            .px_2()
            .py_1()
            .rounded_sm()
            .cursor_pointer()
            .when(selected, |el| el.bg(cx.theme().accent.opacity(0.25)))
            .when(!selected, |el| el.hover(|s| s.bg(cx.theme().muted)))
            .text_sm()
            .text_align(TextAlign::Center)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if let Some(session) = this.opening_tree_at_mut(session_index) {
                        let session_id = session.id;
                        session.go_to_history(index, cx);
                        this.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                    }
                    cx.notify();
                }),
            )
            .child(label)
    }

    pub(crate) fn render_history_column(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .flex_shrink_0()
            .w(px(220.))
            .h_full()
            .min_h_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(self.render_history_table(session_index, cx))
    }

    pub(crate) fn render_history_table(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let history = self
            .opening_tree_at(session_index)
            .map(|session| &session.history)
            .expect("opening tree tab");
        let move_rows = history.len().saturating_sub(1).div_ceil(2);
        let header = h_flex()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.35))
            .child(
                div()
                    .w(px(36.))
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("#"),
            )
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("White"),
            )
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Black"),
            );

        let start_row = h_flex()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .py_0p5()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().w(px(36.)).text_xs().text_color(cx.theme().muted_foreground))
            .child(
                div()
                    .flex_1()
                    .child(self.history_cell(cx, session_index, 0, "Start".into())),
            )
            .child(div().flex_1());

        let mut rows = vec![start_row.into_any_element()];
        for move_number in 1..=move_rows {
            let white_index = move_number * 2 - 1;
            let black_index = move_number * 2;
            let white_cell = if white_index < history.len() {
                self.history_cell(
                    cx,
                    session_index,
                    white_index,
                    history[white_index]
                        .san
                        .clone()
                        .unwrap_or_default()
                        .into(),
                )
                .into_any_element()
            } else {
                div().flex_1().into_any_element()
            };
            let black_cell = if black_index < history.len() {
                self.history_cell(
                    cx,
                    session_index,
                    black_index,
                    history[black_index]
                        .san
                        .clone()
                        .unwrap_or_default()
                        .into(),
                )
                .into_any_element()
            } else {
                div().flex_1().into_any_element()
            };
            rows.push(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .px_2()
                    .py_0p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .w(px(36.))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{move_number}.")),
                    )
                    .child(div().flex_1().child(white_cell))
                    .child(div().flex_1().child(black_cell))
                    .into_any_element(),
            );
        }

        v_flex()
            .size_full()
            .min_h_0()
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(header)
                    .children(rows),
            )
    }
}
