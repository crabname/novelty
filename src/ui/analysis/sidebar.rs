use gpui::*;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::ui::engine_pick::EnginePickTarget;

const ANALYSIS_SIDEBAR_WIDTH: f32 = 280.;

impl NoveltyApp {
    pub(super) fn render_analysis_sidebar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.sidebar_collapsed {
            return self.render_collapsed_sidebar("analysis-sidebar-collapsed", cx);
        }

        let session = self.active_game_analysis().expect("analysis tab");
        let tab_id = session.id;
        let status = session.status.clone();
        let history_index = session.history_index;
        let move_total = session.history.len().saturating_sub(1);
        let pgn_input = session.pgn_input.clone();
        let selected_engine = session
            .engine
            .selected_engine_id
            .clone()
            .unwrap_or_default();
        let engines = self.engines.clone();
        let opening_label = session.opening_label();

        let scroll_body = v_flex()
            .gap_3()
            .child(
                GroupBox::new()
                    .title("PGN")
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Label::new("Paste a game or movetext")
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(Input::new(&pgn_input).h(px(140.)).w_full()),
                    ),
            )
            .child(
                GroupBox::new()
                    .title("Opening")
                    .child(
                        Label::new(opening_label)
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(
                GroupBox::new()
                    .title("Engine")
                    .child(
                        v_flex()
                            .gap_2()
                            .children(if engines.is_empty() {
                                vec![Label::new("Add an engine in the Engine tab")
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .into_any_element()]
                            } else {
                                engines
                                    .iter()
                                    .map(|engine| {
                                        self.render_engine_pick_card(
                                            &format!("analysis-sidebar-engine-{}", engine.id),
                                            engine,
                                            selected_engine == engine.id,
                                            EnginePickTarget::GameAnalysis(tab_id),
                                            cx,
                                        )
                                        .into_any_element()
                                    })
                                    .collect()
                            }),
                    ),
            )
            .child(self.render_navigation_group(
                SharedString::from(format!("analysis-back-{tab_id}")),
                SharedString::from(format!("analysis-fwd-{tab_id}")),
                SharedString::from(format!("analysis-start-{tab_id}")),
                history_index == 0,
                history_index >= move_total,
                history_index == 0,
                move |this, cx| {
                    if let Some(session) = this.game_analysis_by_id_mut(tab_id) {
                        session.go_back(cx);
                    }
                    this.refresh_analysis_if_engine_selected(tab_id, cx);
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                },
                move |this, cx| {
                    if let Some(session) = this.game_analysis_by_id_mut(tab_id) {
                        session.go_forward(cx);
                    }
                    this.refresh_analysis_if_engine_selected(tab_id, cx);
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                },
                move |this, cx| {
                    if let Some(session) = this.game_analysis_by_id_mut(tab_id) {
                        session.go_to_history(0, cx);
                    }
                    this.refresh_analysis_if_engine_selected(tab_id, cx);
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                },
                cx,
            ));

        self.render_expanded_sidebar(
            "analysis-sidebar",
            "Analysis",
            ANALYSIS_SIDEBAR_WIDTH,
            status,
            scroll_body,
            cx,
        )
    }
}
