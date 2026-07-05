use gpui::*;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::opening_explorer::{
    aggregate_explorer_details, explorer_grand_total, explorer_move_share_label, ExplorerMove,
};
use crate::performance::simplify_count;
use crate::ui::moves::{
    moves_table_header, render_scrollable_moves_table, results_bar, MovesTableColumn,
};

impl NoveltyApp {
    pub(crate) fn render_explorer_moves_table(
        &self,
        session_index: usize,
        cx: &mut Context<NoveltyApp>,
    ) -> AnyElement {
        let Some((explorer, tab_id)) = self.explorer_table_context(session_index) else {
            return div().into_any_element();
        };
        let moves = explorer.moves.clone();
        let loading = explorer.loading;
        let error = explorer.error.clone();

        if loading && moves.is_empty() {
            return render_scrollable_moves_table(
                moves_table_header(cx, MovesTableColumn::EXPLORER, "Share"),
                Vec::new(),
                Some("Loading Lichess Opening Explorer…"),
                cx,
            );
        }

        if let Some(error) = error {
            return render_scrollable_moves_table(
                moves_table_header(cx, MovesTableColumn::EXPLORER, "Share"),
                Vec::new(),
                Some(error),
                cx,
            );
        }

        if moves.is_empty() {
            return render_scrollable_moves_table(
                moves_table_header(cx, MovesTableColumn::EXPLORER, "Share"),
                Vec::new(),
                Some("No games in Lichess database for this position."),
                cx,
            );
        }

        let grand_total = explorer_grand_total(&moves);
        let total_details = aggregate_explorer_details(&moves);

        let mut rows = Vec::with_capacity(moves.len() + 1);
        for (index, mv) in moves.into_iter().enumerate() {
            rows.push(self.explorer_move_row(
                cx,
                session_index,
                tab_id,
                index,
                mv,
                grand_total,
            ));
        }
        rows.push(Self::explorer_total_row(
            cx,
            session_index,
            grand_total,
            total_details,
        ));

        let header = moves_table_header(cx, MovesTableColumn::EXPLORER, "Share");
        render_scrollable_moves_table(header, rows, None::<&str>, cx)
    }

    fn explorer_move_row(
        &self,
        cx: &mut Context<NoveltyApp>,
        session_index: usize,
        tab_id: u64,
        move_index: usize,
        mv: ExplorerMove,
        grand_total: u32,
    ) -> AnyElement {
        let san = mv.san.clone();
        let san_for_play = san.clone();
        let share_label = explorer_move_share_label(mv.total(), grand_total);
        let details = mv.position_details();
        let results = results_bar(cx, &details);

        h_flex()
            .id(SharedString::from(format!(
                "explorer-move-{session_index}-{move_index}"
            )))
            .flex_shrink_0()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .cursor_pointer()
            .hover(|s| s.bg(cx.theme().muted.opacity(0.45)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.play_explorer_move(tab_id, &san_for_play, cx);
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(44.))
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(san),
            )
            .child(
                div()
                    .w(px(56.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(share_label),
            )
            .child(results)
            .into_any_element()
    }

    fn explorer_total_row(
        cx: &App,
        session_index: usize,
        grand_total: u32,
        details: crate::graph::PositionDetails,
    ) -> AnyElement {
        let games_label = simplify_count(grand_total);
        let results = results_bar(cx, &details);

        h_flex()
            .id(SharedString::from(format!("explorer-total-{session_index}")))
            .flex_shrink_0()
            .gap_2()
            .px_2()
            .py_1()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.35))
            .child(
                div()
                    .w(px(44.))
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Total"),
            )
            .child(
                div()
                    .w(px(56.))
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().muted_foreground)
                    .child(games_label),
            )
            .child(results)
            .into_any_element()
    }
}
