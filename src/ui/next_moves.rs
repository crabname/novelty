use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::link::Link;
use gpui_component::tooltip::Tooltip;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::graph::MoveStat;
use crate::performance::{performance_details, simplify_count};
use crate::ui::moves::{
    moves_table_header, render_scrollable_moves_table, results_bar, MovesTableColumn,
};

impl NoveltyApp {
    fn performance_tooltip(
        details: crate::graph::PositionDetails,
        player_color: crate::fetch::PlayerColor,
        has_player: bool,
        san: String,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + Clone {
        move |window, cx| {
            let perf = performance_details(&details, player_color, has_player);
            let win_pct = if perf.win_percent.fract() < f32::EPSILON {
                format!("{:.0}%", perf.win_percent)
            } else {
                format!("{:.1}%", perf.win_percent)
            };
            let results = perf.results.clone();
            let score = perf.score.clone();
            let performance_rating = perf.performance_rating;
            let average_opponent_elo = perf.average_opponent_elo;
            let title = format!("{san} — performance");

            Tooltip::element(move |_, _| {
                v_flex()
                    .gap_1()
                    .p_2()
                    .min_w(px(200.))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.clone()),
                    )
                    .when_some(performance_rating, |this, rating| {
                        this.child(
                            h_flex()
                                .justify_between()
                                .gap_3()
                                .child(div().text_xs().child("Performance"))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(rating.to_string()),
                                ),
                        )
                    })
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_3()
                            .child(div().text_xs().child("Results"))
                            .child(div().text_xs().child(results.clone())),
                    )
                    .when_some(average_opponent_elo, |this, elo| {
                        this.child(
                            h_flex()
                                .justify_between()
                                .gap_3()
                                .child(div().text_xs().child("Avg opponent"))
                                .child(div().text_xs().child(elo.to_string())),
                        )
                    })
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_3()
                            .child(div().text_xs().child("Win %"))
                            .child(div().text_xs().child(win_pct.clone())),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_3()
                            .child(div().text_xs().child("Score"))
                            .child(div().text_xs().child(score.clone())),
                    )
            })
            .build(window, cx)
        }
    }

    fn last_game_cell(
        &self,
        cx: &App,
        session_index: usize,
        move_index: usize,
        san: &str,
        details: &crate::graph::PositionDetails,
    ) -> AnyElement {
        let Some(last) = details.last_game.as_ref() else {
            return div().w(px(32.)).into_any_element();
        };
        let Some(url) = last.url.clone() else {
            return div()
                .w(px(32.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("—")
                .into_any_element();
        };
        let date = last.date.clone().unwrap_or_else(|| "Last game".into());
        let link_id = SharedString::from(format!("last-game-{session_index}-{move_index}-{san}"));

        div()
            .w(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .id(SharedString::from(format!("last-game-wrap-{session_index}-{move_index}-{san}")))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .tooltip(move |window, cx| Tooltip::new(date.clone()).build(window, cx))
            .child(
                Link::new(link_id)
                    .href(url)
                    .child(
                        Icon::new(IconName::ExternalLink)
                            .xsmall()
                            .text_color(cx.theme().link),
                    ),
            )
            .into_any_element()
    }

    fn next_move_row(
        &self,
        cx: &mut Context<NoveltyApp>,
        session_index: usize,
        move_index: usize,
        mv: MoveStat,
    ) -> AnyElement {
        let san = mv.san.clone();
        let san_for_play = san.clone();
        let count_label = simplify_count(mv.count);
        let tooltip = Self::performance_tooltip(
            mv.details.clone(),
            self.color,
            self.opening_tree_at(session_index)
                .is_none_or(|session| session.username.is_empty()),
            san.clone(),
        );
        let results = results_bar(cx, &mv.details);
        let last_game = self.last_game_cell(cx, session_index, move_index, &san, &mv.details);

        h_flex()
            .id(SharedString::from(format!("next-move-{session_index}-{move_index}")))
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
                    if let Some(session) = this.opening_tree_at_mut(session_index) {
                        let session_id = session.id;
                        session.play_move_san(&san_for_play, cx);
                        this.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                    }
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
                h_flex()
                    .w(px(56.))
                    .gap_0p5()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(count_label),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "move-info-{session_index}-{move_index}"
                            )))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .tooltip(tooltip)
                            .child(
                                Icon::new(IconName::Info)
                                    .xsmall()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    ),
            )
            .child(results)
            .child(last_game)
            .into_any_element()
    }

    pub(crate) fn render_next_moves_table(
        &self,
        session_index: usize,
        cx: &mut Context<NoveltyApp>,
    ) -> impl IntoElement {
        let session = self
            .opening_tree_at(session_index)
            .expect("opening tree tab");
        let moves = session
            .graph
            .lock()
            .expect("graph lock")
            .moves_at(&session.current_fen);

        if moves.is_empty() {
            let header = moves_table_header(cx, MovesTableColumn::OPENING_TREE, "Games");
            return render_scrollable_moves_table(
                header,
                Vec::new(),
                Some("No moves in this position. Load games to explore your repertoire."),
                cx,
            );
        }

        let mut rows = Vec::with_capacity(moves.len());
        for (index, mv) in moves.into_iter().enumerate() {
            rows.push(self.next_move_row(cx, session_index, index, mv));
        }

        let header = moves_table_header(cx, MovesTableColumn::OPENING_TREE, "Games");
        render_scrollable_moves_table(header, rows, None::<&str>, cx)
    }
}
