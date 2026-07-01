use gpui::*;
use gpui_component::button::Button;
use gpui_component::group_box::GroupBox;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::switch::Switch;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::engine_uci::EngineLine;

#[derive(Clone, Copy, Debug)]
pub(crate) enum EnginePanelTarget {
    GameAnalysis {
        tab_id: u64,
        session_index: usize,
    },
    OpeningTree {
        session_id: u64,
        session_index: usize,
    },
}

impl NoveltyApp {
    pub(crate) fn render_engine_panel(
        &self,
        panel_id_prefix: &str,
        target: EnginePanelTarget,
        analyzing: bool,
        depth: u32,
        line_count: u32,
        show_engine_lines: bool,
        lines: &[EngineLine],
        result_depth: u32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_y_scrollbar()
            .p_4()
            .gap_4()
            .child(
                GroupBox::new()
                    .title("Settings")
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Label::new("Depth")
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(self.render_engine_setting_buttons(
                                        panel_id_prefix,
                                        target,
                                        "depth",
                                        &[12, 14, 16, 20, 24],
                                        depth,
                                        cx,
                                    )),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Label::new("Lines (MultiPV)")
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(self.render_engine_setting_buttons(
                                        panel_id_prefix,
                                        target,
                                        "lines",
                                        &[1, 3, 5],
                                        line_count,
                                        cx,
                                    )),
                            )
                            .child(
                                Switch::new(SharedString::from(format!(
                                    "{panel_id_prefix}-show-lines"
                                )))
                                .label("Show lines on board")
                                .checked(show_engine_lines)
                                .on_click(cx.listener(move |this, checked, _, cx| {
                                    match target {
                                        EnginePanelTarget::GameAnalysis { session_index, .. } => {
                                            if let Some(session) =
                                                this.game_analysis_at_mut(session_index)
                                            {
                                                session.settings.show_engine_lines = *checked;
                                                session.refresh_board(cx);
                                            }
                                        }
                                        EnginePanelTarget::OpeningTree {
                                            session_index, ..
                                        } => {
                                            if let Some(session) =
                                                this.opening_tree_at_mut(session_index)
                                            {
                                                session.settings.show_engine_lines = *checked;
                                                session.refresh_board(cx);
                                            }
                                        }
                                    }
                                    cx.notify();
                                })),
                            ),
                    ),
            )
            .child(
                GroupBox::new()
                    .title(if analyzing {
                        "Analyzing…".into()
                    } else if result_depth > 0 {
                        format!("Lines at depth {result_depth}")
                    } else {
                        "Engine lines".into()
                    })
                    .child(
                        v_flex()
                            .gap_2()
                            .children(if lines.is_empty() {
                                vec![Label::new(if analyzing {
                                    "Waiting for engine…"
                                } else {
                                    "Select an engine in the sidebar"
                                })
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .into_any_element()]
                            } else {
                                lines
                                    .iter()
                                    .map(|line| {
                                        let pv = line
                                            .pv
                                            .iter()
                                            .take(8)
                                            .cloned()
                                            .collect::<Vec<_>>()
                                            .join(" ");
                                        let score = line
                                            .score
                                            .clone()
                                            .unwrap_or_else(|| "—".into());
                                        div()
                                            .p_2()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .bg(cx.theme().group_box)
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        h_flex()
                                                            .justify_between()
                                                            .child(
                                                                Label::new(format!(
                                                                    "#{}",
                                                                    line.rank
                                                                ))
                                                                .text_sm()
                                                                .font_weight(
                                                                    FontWeight::SEMIBOLD,
                                                                ),
                                                            )
                                                            .child(
                                                                Label::new(score)
                                                                    .text_sm()
                                                                    .font_weight(
                                                                        FontWeight::MEDIUM,
                                                                    ),
                                                            ),
                                                    )
                                                    .child(
                                                        Label::new(pv)
                                                            .text_xs()
                                                            .text_color(
                                                                cx.theme().muted_foreground,
                                                            ),
                                                    ),
                                            )
                                            .into_any_element()
                                    })
                                    .collect()
                            }),
                    ),
            )
    }

    fn render_engine_setting_buttons(
        &self,
        panel_id_prefix: &str,
        target: EnginePanelTarget,
        kind: &'static str,
        values: &[u32],
        current: u32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap_1()
            .flex_wrap()
            .children(values.iter().map(|&value| {
                let selected = value == current;
                let kind = kind;
                Button::new(SharedString::from(format!(
                    "{panel_id_prefix}-{kind}-{value}"
                )))
                .label(value.to_string())
                .selected(selected)
                .on_click(cx.listener(move |this, _, _, cx| {
                    match target {
                        EnginePanelTarget::GameAnalysis {
                            tab_id,
                            session_index,
                        } => {
                            if let Some(session) = this.game_analysis_at_mut(session_index) {
                                match kind {
                                    "depth" => session.settings.depth = value,
                                    "lines" => session.settings.line_count = value,
                                    _ => {}
                                }
                            }
                            this.refresh_analysis_if_engine_selected(tab_id, cx);
                        }
                        EnginePanelTarget::OpeningTree {
                            session_id,
                            session_index,
                        } => {
                            if let Some(session) = this.opening_tree_at_mut(session_index) {
                                match kind {
                                    "depth" => session.settings.depth = value,
                                    "lines" => session.settings.line_count = value,
                                    _ => {}
                                }
                            }
                            this.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                        }
                    }
                    cx.notify();
                }))
                .into_any_element()
            }))
    }
}
